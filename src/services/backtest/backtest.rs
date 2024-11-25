use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::cmp::max;
use chrono::{Datelike, NaiveDate, Weekday};
use itertools::{enumerate};
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::action::{TradeAction};
use crate::models::asset::Asset;
use crate::services::aws::AwsClient;
use crate::services::configuration::{kate_configuration_directory, S3_BUCKET_HISTORICAL_DATA};
use crate::strategies::strategy::{CandlestickTradingStrategy, HighResolutionTradingStrategy};
use crate::utils::{date};
use simple_bar::ProgressBar;
use rayon::prelude::*;
use crate::models::trade::{Trade, TradeFile};
use crate::services::backtest::exchange_event_handler::{ExchangeEventHandler, MINIMUM_BALANCE};
use crate::services::backtest::metrics;
use crate::services::backtest::metrics::BacktestMetrics;

#[derive(Debug)]
#[allow(unused)]
pub struct TradingBacktester<'a> {
    pub qty_history_trading_days: u64,
    pub qty_history_candles: u64,
    pub qty_history_trades: u64,
    asset: &'a Asset,
    data_provider: DataProvider,
}

#[derive(Debug)]
#[allow(unused)]
struct DataProvider {
    candlesticks: Vec<Candlestick>,
    trades: Vec<Trade>,
    data_files: Vec<String>
}

impl DataProvider {
    fn new(candlesticks: Vec<Candlestick>, trades: Vec<Trade>, data_files: Vec<String>) -> Self {
        DataProvider {
            candlesticks,
            trades,
            data_files
        }
    }
}

fn add_orders_event(event_handler: &mut ExchangeEventHandler, action: TradeAction) {
    if let Some(enter_order) = action.enter_market_action {
        event_handler.add_order_event(enter_order);
    }

    if let Some(stop_order) = action.stop_loss_action {
        event_handler.add_order_event(stop_order);
    }

    if let Some(profit_order) = action.take_profit_action {
        event_handler.add_order_event(profit_order);
    }
}

impl<'a> TradingBacktester<'a> {

    /// Starts the execution of a Backtest ( trading simulation ) where each datapoint is a [Trade]
    ///
    /// [HighResolutionTradingStrategy] defines how and when to send orders to buy and sell the asset being traded
    /// after evaluating each new [Trade] as they become available
    pub fn run_high_resolution_backtest(&self, mut strategy: Box<dyn HighResolutionTradingStrategy>, base_qty_to_trade: u32, fixed_traded_qty: bool) -> BacktestMetrics {
        let mut event_handler = ExchangeEventHandler::new(self.asset, base_qty_to_trade, fixed_traded_qty);

        let mut index_list_each_start_trading_day = vec![];
        self.add_context_to_high_resolution_strategy(&mut strategy, &mut index_list_each_start_trading_day, 0);

        let index_first_actual_trading_day = *index_list_each_start_trading_day.last().unwrap();
        let mut current_day = self.data_provider.trades.get(index_first_actual_trading_day).unwrap().date;

        for (index, trade) in enumerate(self.data_provider.trades.iter().skip(index_first_actual_trading_day)) {
            if current_day != trade.date {
                self.add_context_to_high_resolution_strategy(
                    &mut strategy,
                    &mut index_list_each_start_trading_day,
                    index_first_actual_trading_day + index,
                );
                current_day = trade.date;
            }

            let trade_action = strategy.process_new_trade(trade);
            event_handler.add_trade_event(&trade);
            add_orders_event(&mut event_handler, trade_action);
            event_handler.process_events();

            if event_handler.current_balance <= MINIMUM_BALANCE || event_handler.removed_for_lack_of_margin {
                event_handler.removed_for_lack_of_margin = true;
                break;
            }
        }

        metrics::calculate_backtest_results(&event_handler)
    }

    /// Starts the execution of a Backtest ( trading simulation ) where each datapoint is closed [Candlestick]
    ///
    /// [CandlestickTradingStrategy] defines how and when to send orders to buy and sell the asset being traded
    /// after evaluating each new [Candlestick] as they become available
    pub fn run_candlestick_backtest(&self, mut strategy: Box<dyn CandlestickTradingStrategy>, base_qty_to_trade: u32, fixed_traded_qty: bool) -> BacktestMetrics {
        let mut event_handler = ExchangeEventHandler::new(self.asset, base_qty_to_trade, fixed_traded_qty);

        let mut index_list_each_start_trading_day = vec![];
        self.add_context_to_candle_strategy(&mut strategy, &mut index_list_each_start_trading_day, 0);

        let index_first_actual_trading_day = *index_list_each_start_trading_day.last().unwrap();
        let mut current_day = self.data_provider.candlesticks.get(index_first_actual_trading_day).unwrap().date;

        for (index, candlestick) in enumerate(self.data_provider.candlesticks.iter().skip(index_first_actual_trading_day)) {
            if current_day != candlestick.date {
                self.add_context_to_candle_strategy(
                    &mut strategy,
                    &mut index_list_each_start_trading_day,
                    index_first_actual_trading_day + index,
                );
                current_day = candlestick.date;
            }

            let trade_action = strategy.process_closed_candlestick(candlestick);
            event_handler.add_candlestick_event(&candlestick);
            add_orders_event(&mut event_handler, trade_action);
            event_handler.process_events();

            if event_handler.current_balance <= MINIMUM_BALANCE || event_handler.removed_for_lack_of_margin {
                event_handler.removed_for_lack_of_margin = true;
                break;
            }
        }

        metrics::calculate_backtest_results(&event_handler)
    }

    /// Initializes the TradingBacktester
    /// The historical data will be loaded from the local disk or from AWS S3 if not available locally
    pub fn new(
        asset: &Asset,
        start_date: u32,
        end_date: u32,
        cache_in_memory: bool,
        candlestick_generator: Option<Box<dyn CandlestickGenerator>>,
        no_download: bool,
    ) -> Result<TradingBacktester, anyhow::Error> {
        let start_date = date::convert_to_naive_date(start_date)?;
        let end_date = date::convert_to_naive_date(end_date)?;

        if end_date < start_date {
            anyhow::bail!("invalid dates, the end_date is before the start_date");
        }

        let history_trades_folder_path = kate_configuration_directory()
            .join("history_trades")
            .join(asset.ticker.as_str());

        if !history_trades_folder_path.exists() {
            std::fs::create_dir_all(&history_trades_folder_path)?;
        }

        let exchange = asset.exchange.clone();
        let market = asset.market.clone();

        let mut trading_days = 0;
        let mut files_to_parse: Vec<String> = vec![];

        let uppercase_ticker = asset.ticker.to_ascii_uppercase();
        let mut files_to_download: Vec<String> = vec![];

        println!("checking for missing trade files on local disk between {} and {}", start_date, end_date);
        for year in start_date.year()..=end_date.year() {
            let begin_month = if year == start_date.year() {
                start_date.month()
            } else {
                1
            };

            let finish_month = if year == end_date.year() {
                end_date.month()
            } else {
                12
            };

            for month in begin_month..=finish_month {
                let begin_day = if year == start_date.year() && month == start_date.month() {
                    start_date.day()
                } else {
                    1
                };

                let finish_day = if year == end_date.year() && month == end_date.month() {
                    end_date.day()
                } else {
                    31
                };

                for day in begin_day..=finish_day {
                    let naive_date = NaiveDate::from_ymd_opt(year, month, day);
                    if naive_date.is_none() {
                        continue;
                    }
                    let day_of_week = NaiveDate::from_ymd_opt(year, month, day).unwrap().weekday();
                    if day_of_week == Weekday::Sat || day_of_week == Weekday::Sun {
                        continue; // there is no trading on weekends
                    }

                    let formatted_day = if day < 10 {
                        format!("0{}", day)
                    } else {
                        format!("{}", day)
                    };

                    let formatted_month = if month < 10 {
                        format!("0{}", month)
                    } else {
                        format!("{}", month)
                    };

                    //TRADE-B3-FUTURES-IND-20240722
                    let binary_trades_file_name = format!("TRADE-{exchange}-{market}-{uppercase_ticker}-{year}{formatted_month}{formatted_day}");
                    let binary_trades_file_full_path = history_trades_folder_path.join(binary_trades_file_name.as_str());

                    //trades/IND/TRADE-B3-FUTURES-IND-20240722
                    let file_path_on_s3 = format!("trades/{}/{}", uppercase_ticker, binary_trades_file_name);
                    if binary_trades_file_full_path.exists() {
                        trading_days += 1;
                        files_to_parse.push(binary_trades_file_full_path.to_str().unwrap().to_string());
                    } else if !no_download {
                        if AwsClient::new("sa-east-1".to_string())?.file_exists_on_s3(S3_BUCKET_HISTORICAL_DATA, file_path_on_s3.as_str())? == true {
                            trading_days += 1;
                            files_to_download.push(binary_trades_file_name);
                            files_to_parse.push(binary_trades_file_full_path.to_str().unwrap().to_string());
                        }
                    }
                }
            }
        }

        if !no_download && files_to_download.len() > 0 {
            let number_of_threads = rayon::current_num_threads();
            println!("download in parallel missing files with historical data from aws s3 using {} threads", number_of_threads);
            let mut bar = ProgressBar::default(files_to_download.len() as u32 + 1, 100, false);
            bar.update();
            let download_files_bar = Arc::new(Mutex::new(bar));
            let values_to_process: Vec<(&String, Arc<Mutex<ProgressBar>>)> = files_to_download.iter().map(|files_to_download| {
                (files_to_download, Arc::clone(&download_files_bar))
            }).collect();

            let aws_client = Arc::new(AwsClient::new("sa-east-1".to_string())?);
            values_to_process.into_par_iter().for_each(|tuple| {
                let file_to_download = tuple.0;
                let progress_bar = tuple.1;
                download_binary_trades_file(
                    aws_client.clone(),
                    format!("trades/{}/{}", uppercase_ticker, file_to_download),
                    history_trades_folder_path.to_str().unwrap().to_string(),
                    file_to_download.as_str(),
                ).expect(format!("failed to download file {}", file_to_download).as_str());

                let mut download_progress_bar = progress_bar.lock().unwrap();
                download_progress_bar.update();
            })
        }

        println!("parsing binary files with historical data from local disk");
        let progress_bar = Arc::new(Mutex::new(ProgressBar::default(files_to_parse.len() as u32, 100, false)));

        let mut candlesticks: Vec<Vec<Candlestick>> = Vec::new();
        let mut trades: Vec<Vec<Trade>> = Vec::new();

        if cache_in_memory {
            if let Some(candle_generator) =  candlestick_generator {
                files_to_parse.clone()
                    .into_par_iter()
                    .map(|binary_trade_file| {
                        let bar = Arc::clone(&progress_bar);
                        let parser = candle_generator.clone();
                        let candlesticks_current_file =
                            parse_candles_from_file(binary_trade_file.to_string(), parser)
                                .expect("should not fail when parsing candlesticks from trades file");
                        let mut parse_progress_bar = bar.lock().unwrap();
                        parse_progress_bar.update();
                        candlesticks_current_file
                    }).collect_into_vec(&mut candlesticks);
            }else{
                files_to_parse.clone()
                    .into_par_iter()
                    .map(|binary_trade_file| {
                        let bar = Arc::clone(&progress_bar);
                        let trades_current_file =
                            parse_trades_from_file(binary_trade_file.to_string())
                                .expect("should not fail when parsing trades from file");
                        let mut parse_progress_bar = bar.lock().unwrap();
                        parse_progress_bar.update();
                        trades_current_file
                    }).collect_into_vec(&mut trades);
            }
        }


        let candlesticks: Vec<Candlestick> = candlesticks.into_iter().flatten().collect();
        let trades: Vec<Trade> = trades.into_iter().flatten().collect();

        Ok(TradingBacktester {
            asset,
            qty_history_trading_days: trading_days,
            qty_history_candles: candlesticks.len() as u64,
            qty_history_trades: max(
                trades.len() as u64,
                candlesticks.iter()
                    .fold(0, |total, candle| total + candle.trades)
            ),
            data_provider: DataProvider::new(candlesticks, trades, files_to_parse),
        })
    }


    fn add_context_to_candle_strategy(&self, strategy: &mut Box<dyn CandlestickTradingStrategy>, index_start_day_list: &mut Vec<usize>, offset_start_current_day: usize) {
        index_start_day_list.push(offset_start_current_day);
        let context_size: usize = strategy.days_of_context() as usize;

        if context_size == 0 {
            return;
        }

        if index_start_day_list.len() <= context_size {
            let mut days_of_context = context_size;
            let mut current_day = self.data_provider.candlesticks.get(offset_start_current_day).unwrap().date;
            for (index, initialization_candle) in self.data_provider.candlesticks.iter().skip(offset_start_current_day).enumerate() {
                if initialization_candle.date != current_day {
                    index_start_day_list.push(index + offset_start_current_day);
                    current_day = initialization_candle.date;
                    days_of_context -= 1;
                    if days_of_context == 0 {
                        break;
                    }
                }
            }
        }

        let start_index: usize = *index_start_day_list.get(index_start_day_list.len() - context_size - 1).unwrap();
        let end_index: usize = *index_start_day_list.last().unwrap();

        strategy.init_trading_day(
            self.asset,
            &self.data_provider.candlesticks[start_index..end_index],
        );
    }

    fn add_context_to_high_resolution_strategy(&self, strategy: &mut Box<dyn HighResolutionTradingStrategy>, index_start_day_list: &mut Vec<usize>, offset_start_current_day: usize) {
        index_start_day_list.push(offset_start_current_day);
        let context_size: usize = strategy.days_of_context() as usize;

        if context_size == 0 {
            return;
        }

        if index_start_day_list.len() <= context_size {
            let mut days_of_context = context_size;
            let mut current_day = self.data_provider.trades.get(offset_start_current_day).unwrap().date;
            for (index, initialization_trade) in self.data_provider.trades.iter().skip(offset_start_current_day).enumerate() {
                if initialization_trade.date != current_day {
                    index_start_day_list.push(index + offset_start_current_day);
                    current_day = initialization_trade.date;
                    days_of_context -= 1;
                    if days_of_context == 0 {
                        break;
                    }
                }
            }
        }

        let start_index: usize = *index_start_day_list.get(index_start_day_list.len() - context_size - 1).unwrap();
        let end_index: usize = *index_start_day_list.last().unwrap();

        strategy.init_trading_day(
            self.asset,
            &self.data_provider.trades[start_index..end_index],
        );
    }


}

fn download_binary_trades_file(
    aws_client: Arc<AwsClient>,
    file_path_on_s3: String,
    output_folder_on_disk: String,
    file_name: &str,
) -> Result<(), anyhow::Error> {
    let file_output_path_on_disk = format!("{}/{}",
                                           output_folder_on_disk,
                                           file_name
    );
    let file_download_result = aws_client.download_file_from_s3(S3_BUCKET_HISTORICAL_DATA,
                                                                file_path_on_s3.as_str(),
                                                                file_output_path_on_disk.as_str(),
    );
    file_download_result
}

fn parse_trades_from_file(binary_trades_file_full_path: String) -> Result<Vec<Trade>, anyhow::Error> {
    let binary_trades_path = PathBuf::from(binary_trades_file_full_path.clone());

    if !binary_trades_path.exists() {
        anyhow::bail!("binary trades file {} doesnt exist on disk",
            binary_trades_path.to_str().unwrap());
    }

    let trades_file = TradeFile::new_from_binary_file(binary_trades_file_full_path.as_str())?;
    let mut trades: Vec<Trade> = Vec::new();

    for trade in trades_file {
        trades.push(trade)
    }
    Ok(trades)
}

fn parse_candles_from_file(
    binary_trades_file_full_path: String,
    mut candlestick_parser: Box<dyn CandlestickGenerator>,
) -> Result<Vec<Candlestick>, anyhow::Error> {
    let binary_trades_path = PathBuf::from(binary_trades_file_full_path.clone());

    if !binary_trades_path.exists() {
        anyhow::bail!("binary trades file {} doesnt exist on disk",
            binary_trades_path.to_str().unwrap());
    }

    let trades_file = TradeFile::new_from_binary_file(binary_trades_file_full_path.as_str())?;
    let mut candlesticks: Vec<Candlestick> = Vec::new();

    for trade in trades_file {
        if let Some(candle) = candlestick_parser.parse_only_closed_candles(&trade) {
            candlesticks.push(candle);
        }
    }
    Ok(candlesticks)
}

