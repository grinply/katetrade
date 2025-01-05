use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::cmp::max;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::asset::Asset;
use crate::services::aws::AwsClient;
use crate::services::configuration::{kate_configuration_directory, S3_BUCKET_HISTORICAL_DATA};
use crate::strategies::strategy::{TradingStrategy};
use crate::utils::{date};
use simple_bar::ProgressBar;
use rayon::prelude::*;
use crate::constants::{AWS_REGION, TRADES_FOLDER_NAME};
use crate::models::trade;
use crate::models::trade::{Trade, TradeFile};
use crate::services::backtest::exchange_event_handler::{ExchangeEventHandler, MINIMUM_BALANCE};
use crate::services::backtest::{logger, metrics};
use crate::services::backtest::logger::BacktestEvent;
use crate::services::backtest::metrics::BacktestMetrics;

#[allow(unused)]
pub struct TradingBacktester<'a> {
    qty_history_trading_days: u64,
    qty_history_candles: u64,
    qty_history_trades: u64,
    asset: &'a Asset,
    data_provider: DataProvider,
}

#[allow(unused)]
struct DataProvider {
    candlesticks: Vec<Candlestick>,
    trades: Vec<Trade>,
    data_files: Vec<String>,
    candle_parser: Option<Box<dyn CandlestickGenerator>>,
}

struct BacktestDataFiles {
    trading_days: usize,
    files_to_parse: Vec<String>,
    files_to_download: Vec<String>,
}

impl DataProvider {
    fn new(candlesticks: Vec<Candlestick>, trades: Vec<Trade>) -> Self {
        DataProvider {
            candlesticks,
            trades,
            data_files: vec![],
            candle_parser: None,
        }
    }

    fn from_files(data_files: Vec<String>, candle_parser: Option<Box<dyn CandlestickGenerator>>) -> Self {
        DataProvider {
            candlesticks: vec![],
            trades: vec![],
            data_files,
            candle_parser,
        }
    }

    fn trades_iterator_from_disk(&self) -> TradeDiskIterator {
        TradeDiskIterator {
            buffer: VecDeque::new(),
            files: self.data_files.clone(),
            file_index: 0,
            is_finished: false,
        }
    }
}

struct TradeDiskIterator {
    buffer: VecDeque<Trade>, // VecDeque::new()
    files: Vec<String>,
    file_index: usize,
    is_finished: bool,
}

/// iterator over [Trade] that will only allocate a single trading day into main memory at any given time
/// reducing the memory requirements for accessing historical data
impl Iterator for TradeDiskIterator {
    type Item = Trade;
    fn next(&mut self) -> Option<Trade> {
        if self.is_finished || (self.file_index >= self.files.len() && self.buffer.is_empty()) {
            self.is_finished = true;
            return None;
        }

        if self.buffer.is_empty() {
            let trades_file = self.files.get(self.file_index).unwrap();
            self.buffer = trade::read_trades_from_binary_file(trades_file).unwrap();
            self.file_index += 1;
        }
        self.buffer.pop_front()
    }
}

impl Display for TradingBacktester<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} trading days with {} candlesticks generated from {} trades",
                 self.qty_history_trading_days,
                 self.qty_history_candles,
                 self.qty_history_trades)
    }
}

impl<'a> TradingBacktester<'a> {
    /// Starts the execution of a Backtest ( trading simulation ) where each datapoint is a [Trade]
    ///
    /// [TradingStrategy] defines how and when to send orders to buy and sell the asset being traded
    /// after evaluating each new [Trade] as they become available
    pub fn run_high_resolution_backtest(&self, mut strategy: Box<dyn TradingStrategy>) -> BacktestMetrics {
        if strategy.days_of_context() > 0{
            panic!("initializing one or more days of context is not supported currently in backtests");
        }

        let mut event_handler = ExchangeEventHandler::new(self.asset);

        // running using cached trades from main memory
        if self.data_provider.trades.len() > 0 {
            let mut current_day = -1;
            for trade in self.data_provider.trades.iter() {
                if current_day != trade.date {
                    strategy.init_trading_day(self.asset, [].as_ref());
                    current_day = trade.date;
                }
                let trade_action = strategy.process_new_trade(trade);
                let current_balance = event_handler.current_balance;

                event_handler.process_trade(trade);
                event_handler.process_orders(trade_action);

                let updated_balance = event_handler.current_balance;

                if current_balance != updated_balance {
                    logger::log_balance_change(
                        date::convert_to_naive_date(trade.date as u32).unwrap(),
                        date::convert_to_naive_time(trade.time).unwrap(),
                        BacktestEvent::BalanceUpdate,
                        updated_balance,
                        updated_balance - current_balance
                    );
                }

                if self.should_abort_execution(&mut event_handler) {
                    break;
                }
            }
        } else {
            // running loading trades from disk for each new trading day
            for new_trade in self.data_provider.trades_iterator_from_disk() {
                let trade_action = strategy.process_new_trade(&new_trade);
                event_handler.process_trade(&new_trade);
                event_handler.process_orders(trade_action);
                if self.should_abort_execution(&mut event_handler) {
                    break;
                }
            }
        }
        metrics::calculate_backtest_results(&event_handler)
    }

    fn should_abort_execution(&self, event_handler: &mut ExchangeEventHandler) -> bool {
        if event_handler.current_balance <= MINIMUM_BALANCE || event_handler.removed_for_lack_of_margin {
            event_handler.removed_for_lack_of_margin = true;
            println!("removed for the lack of margin to operate");
            true
        } else {
            false
        }
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


        println!("checking for missing trade files on local disk between {} and {} on {:?}",
                 start_date,
                 end_date,
                 history_trades_folder_path
        );

        let backtest_files_metadata = get_backtest_files_metadata(
            asset,
            start_date,
            end_date,
            &history_trades_folder_path,
            !no_download,
        )?;


        if !no_download && backtest_files_metadata.files_to_download.len() > 0 {
            println!("download in parallel missing files with historical data from aws s3 using {} threads", rayon::current_num_threads());
            backtest_files_metadata.download_missing_files(asset, &history_trades_folder_path)?;
        }

        let backtester = if cache_in_memory {
            println!("parsing binary files with historical data from local disk");
            let (candles, trades) = backtest_files_metadata.parse_data_from_files(candlestick_generator);
            TradingBacktester {
                asset,
                qty_history_trading_days: backtest_files_metadata.trading_days as u64,
                qty_history_candles: candles.len() as u64,
                qty_history_trades: max(
                    trades.len() as u64,
                    candles.iter().fold(0, |total, candle| total + candle.trades),
                ),
                data_provider: DataProvider::new(candles, trades),
            }
        } else {
            TradingBacktester {
                asset,
                qty_history_trading_days: backtest_files_metadata.trading_days as u64,
                qty_history_candles: 0,
                qty_history_trades: 0,
                data_provider: DataProvider::from_files(
                    backtest_files_metadata.files_to_parse,
                    candlestick_generator,
                ),
            }
        };
        Ok(backtester)
    }
}

impl BacktestDataFiles {
    fn download_missing_files(&self, asset: &Asset, history_trades_folder_path: &PathBuf) -> Result<(), anyhow::Error> {
        let mut bar = ProgressBar::default(self.files_to_download.len() as u32 + 1, 100, false);
        bar.update();
        let download_files_bar = Arc::new(Mutex::new(bar));
        let values_to_process: Vec<(&String, Arc<Mutex<ProgressBar>>)> = self.files_to_download.iter().map(|files_to_download| {
            (files_to_download, Arc::clone(&download_files_bar))
        }).collect();

        let uppercase_ticker = asset.ticker.to_ascii_uppercase();
        let aws_client = Arc::new(AwsClient::new(AWS_REGION.to_string())?);

        values_to_process.into_par_iter().for_each(|tuple| {
            let file_to_download = tuple.0;
            let progress_bar = tuple.1;
            download_binary_trades_file(
                aws_client.clone(),
                format!("{TRADES_FOLDER_NAME}/{}/{}", uppercase_ticker, file_to_download),
                history_trades_folder_path.to_str().unwrap().to_string(),
                file_to_download.as_str(),
            ).expect(format!("failed to download file {}", file_to_download).as_str());

            let mut download_progress_bar = progress_bar.lock().unwrap();
            download_progress_bar.update();
        });

        Ok(())
    }

    fn parse_data_from_files(&self, candlestick_generator: Option<Box<dyn CandlestickGenerator>>) -> (Vec<Candlestick>, Vec<Trade>) {
        let progress_bar = Arc::new(Mutex::new(ProgressBar::default(self.files_to_parse.len() as u32, 100, false)));
        let mut candlesticks: Vec<Vec<Candlestick>> = Vec::new();
        let mut trades: Vec<Vec<Trade>> = Vec::new();

        if let Some(candle_generator) = candlestick_generator {
            self.files_to_parse.clone()
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
        } else {
            self.files_to_parse.clone()
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

        let candlesticks: Vec<Candlestick> = candlesticks.into_iter().flatten().collect();
        let trades: Vec<Trade> = trades.into_iter().flatten().collect();

        (candlesticks, trades)
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

    let cut_off_time = NaiveTime::from_hms_opt(18,00,00).unwrap().num_seconds_from_midnight();

    for trade in trades_file {
        if date::convert_to_naive_time(trade.time).unwrap().num_seconds_from_midnight() <= cut_off_time {
            trades.push(trade)
        }
    }
    Ok(trades)
}

/// parse the binary data from a file containing only [Trade], into a [Vec] of [Candlestick]
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

fn get_backtest_files_metadata(
    asset: &Asset,
    start_date: NaiveDate,
    end_date: NaiveDate,
    history_trades_folder_path: &PathBuf,
    download_missing_files: bool,
) -> Result<BacktestDataFiles, anyhow::Error> {
    let mut trading_days = 0;
    let mut files_to_parse = Vec::new();
    let mut files_to_download = Vec::new();
    let uppercase_ticker = asset.ticker.to_ascii_uppercase();

    let market = asset.market.to_string().to_uppercase();
    let exchange = asset.exchange.to_string().to_uppercase();

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
                let file_path_on_s3 = format!("{TRADES_FOLDER_NAME}/{}/{}", uppercase_ticker, binary_trades_file_name);
                if binary_trades_file_full_path.exists() {
                    trading_days += 1;
                    files_to_parse.push(binary_trades_file_full_path.to_str().unwrap().to_string());
                } else if download_missing_files {
                    if AwsClient::new(AWS_REGION.to_string())?.file_exists_on_s3(S3_BUCKET_HISTORICAL_DATA, file_path_on_s3.as_str())? == true {
                        trading_days += 1;
                        files_to_download.push(binary_trades_file_name);
                        files_to_parse.push(binary_trades_file_full_path.to_str().unwrap().to_string());
                    }
                }
            }
        }
    }

    Ok(
        BacktestDataFiles {
            trading_days,
            files_to_parse,
            files_to_download,
        }
    )
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::models::asset::Asset;
    use crate::models::candlestick::{CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
    use crate::models::exchange::Exchange;
    use crate::models::market::Market;
    use crate::services::backtest::backtest::TradingBacktester;
    use crate::strategies::custom::stormer_scalper_strategy::{StormerScalperStrategy};
    use crate::strategies::custom::dummy_strategy::{DummyHighResolutionStrategy};
    use crate::strategies::strategy::{TradingStrategy};

    #[test]
    fn should_create_backtester_with_cached_candlestick_data_correctly() {
        let candle_parser = TimeCandlestickGenerator::new(TimeUnit::Minutes, 1);
        let backtester_engine = candle_backtester(true, candle_parser);
        assert_eq!(1638, backtester_engine.data_provider.candlesticks.len());
    }

    #[test]
    fn should_create_backtester_without_cached_candlestick_data_correctly() {
        let candle_parser = TimeCandlestickGenerator::new(TimeUnit::Minutes, 1);
        let backtester_engine = candle_backtester(false, candle_parser);

        assert_eq!(0, backtester_engine.data_provider.candlesticks.len());
        assert_eq!(0, backtester_engine.data_provider.trades.len());
        assert_eq!(3, backtester_engine.data_provider.data_files.len());
    }

    #[test]
    fn should_create_backtester_with_cached_trade_data_correctly() {
        let backtester_engine = trade_backtester(
            true,
            2024_06_13,
            2024_06_17,
        );
        assert_eq!(43761, backtester_engine.data_provider.trades.len());
    }

    #[test]
    fn should_create_backtester_without_cached_trade_data_correctly() {
        let backtester_engine = trade_backtester(
            false,
            2024_06_13,
            2024_06_17,
        );

        assert_eq!(0, backtester_engine.data_provider.candlesticks.len());
        assert_eq!(0, backtester_engine.data_provider.trades.len());
        assert_eq!(3, backtester_engine.data_provider.data_files.len());
    }


    #[test]
    fn should_run_dummy_strategy_on_backtester_with_cached_trade_data_correctly() {
        let backtester_engine = trade_backtester(
            true,
            2024_06_13,
            2024_06_17,
        );

        let strategy = DummyHighResolutionStrategy::new();
        let metrics = backtester_engine.run_high_resolution_backtest(
            Box::new(strategy),
        );

        assert_eq!(86, metrics.total_wins);
        assert_eq!(99, metrics.total_loses);
        assert_eq!(185, metrics.total_trades);
        assert_eq!(6, metrics.max_win_streak);
        assert_eq!(7, metrics.max_lose_streak);
    }

    //#[test]
    #[allow(unused)]
    fn should_correctly_compute_the_total_return_when_running_multiple_days_at_once_on_real_data() {
        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        let metrics_first_day = TradingBacktester::new(
            &asset,
            20241216,
            20241216,
            true,
            None,
            true,
        ).unwrap().run_high_resolution_backtest(
            Box::new(StormerScalperStrategy::new()),
        );

        let metrics_second_day = TradingBacktester::new(
            &asset,
            20241217,
            20241217,
            true,
            None,
            true,
        ).unwrap().run_high_resolution_backtest(
            Box::new(StormerScalperStrategy::new()),
        );

        let metrics_combined_days = TradingBacktester::new(
            &asset,
            20241216,
            20241217,
            true,
            None,
            true,
        ).unwrap().run_high_resolution_backtest(
            Box::new(StormerScalperStrategy::new()),
        );

        let sum_result_individual_days = metrics_first_day.total_liquid_profit + metrics_second_day.total_liquid_profit;
        let result_combined_days = metrics_combined_days.total_liquid_profit;

        let mut dates = metrics_combined_days.trades.iter().map(|trade| trade.date)
            .collect_vec();
        dates.sort();
        dates.dedup();

        assert_eq!(
            vec![20241216, 20241217],
            dates,
            "the dates when running combined days should be the same of when running the backtest for individual days"
        );

        assert_eq!(
            sum_result_individual_days,
            result_combined_days,
            "the result when running combined days is {result_combined_days} but should be equal to the sum of individual days of {sum_result_individual_days}",
        );
    }

    fn candle_backtester(cache_in_memory: bool, candle_parser: impl CandlestickGenerator + 'static) -> TradingBacktester<'static> {
        let asset = Asset::get_asset("IND", Market::FUTURES, Exchange::B3).unwrap();

        TradingBacktester::new(
            &asset,
            2024_06_13,
            2024_06_17,
            cache_in_memory,
            Some(Box::new(candle_parser)),
            true,
        ).unwrap()
    }

    fn trade_backtester(cache_in_memory: bool, start_date: u32, end_date: u32) -> TradingBacktester<'static> {
        let asset = Asset::get_asset("IND", Market::FUTURES, Exchange::B3).unwrap();
        TradingBacktester::new(
            &asset,
            start_date,
            end_date,
            cache_in_memory,
            None,
            true,
        ).unwrap()
    }


}