use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::{JoinHandle};
use chrono::{DateTime, Datelike, Local, NaiveDate, Weekday};
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::{candlestick, side, trade};
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::pnl::Position;
use crate::services::aws::AwsClient;
use crate::services::backtest::TradeEvent::NewCandle;
use crate::services::configuration;
use crate::services::configuration::{home_dir, S3_BUCKET_HISTORICAL_DATA};
use crate::strategies::execution_manager::OrderExecutor;
use crate::strategies::strategy::{CandlestickConfiguration, CandlestickTradingStrategy};
use crate::utils::date;
use simple_bar::ProgressBar;
use crate::models::side::Side;
use rayon::prelude::*;

#[allow(unused)]
pub struct BacktestMetrics {
    pub final_balance: f64,
    pub initial_balance: f64,
    pub total_return_percentage: f64,
    pub total_gross_profit: f64,
    pub total_liquid_profit: f64,
    pub avg_daily_liquid_profit: f64,
    pub worse_day_liquid_profit: f64,
    pub best_day_liquid_profit: f64,
    pub median_day_liquid_profit: f64,
    pub total_fees_paid: f64,
    pub minutes_in_the_market: f64,
    pub win_rate_percentage: f64,
    pub total_trades: u64,
    pub total_wins: u64,
    pub total_loses: u64,
    pub avg_risk_reward: f32,
    pub max_win_streak: u32,
    pub max_lose_streak: u32,
}

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
    candle_configuration: CandlestickConfiguration,
}

#[allow(unused)]
struct ExchangeEventHandler<'a> {
    asset: &'a Asset,
    current_balance: f64,
    slippage_multiplier: f64, // defaults to 0.0001
    total_candlesticks_processed: u64,
    current_position: Option<Position>,
    events: [TradeEvent<'a>; 4],
    event_index: usize,
}

impl DataProvider {
    fn new(candlesticks: Vec<Candlestick>, candle_configuration: CandlestickConfiguration) -> Self {
        DataProvider {
            candlesticks,
            candle_configuration,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum TradeEvent<'a> {
    NewCandle(&'a Candlestick),
    ExecuteOrderLong,
    ExecuteOrderShort,
    ZeroPosition,
    ScheduleOrderLong(f64),
    ScheduleOrderShort(f64),
    None,
}

impl<'a> ExchangeEventHandler<'a> {
    fn new(asset: &'a Asset) -> Self {
        ExchangeEventHandler {
            asset,
            current_balance: 10_000.0,
            slippage_multiplier: 0.0001,
            total_candlesticks_processed: 0,
            current_position: None,
            events: [TradeEvent::None; 4],
            event_index: 0,
        }
    }

    fn add_candlestick_event(&mut self, candlestick: &'a Candlestick) {
        self.events[self.event_index] = NewCandle(candlestick);
        self.event_index += 1;
    }

    fn add_order_event(&mut self, order: OrderAction) {
        let order_event = match order {
            OrderAction::MarketLong => TradeEvent::ExecuteOrderLong,
            OrderAction::MarketShort => TradeEvent::ExecuteOrderShort,
            OrderAction::LimitLong(price) => TradeEvent::ScheduleOrderLong(price),
            OrderAction::LimitShort(price) => TradeEvent::ScheduleOrderShort(price),
        };
        self.events[self.event_index] = order_event;
        self.event_index += 1;
    }

    fn process_events(&mut self) {
        let events = self.events;
        for event in events {
            match event {
                NewCandle(candlestick) => {
                    self.check_compulsory_stop_loss(candlestick);
                    self.check_scheduled_order_trigger(candlestick);
                }
                TradeEvent::ExecuteOrderLong => {
                    self.execute_market_long_order();
                }
                TradeEvent::ExecuteOrderShort => {
                    self.execute_market_short_order();
                }
                TradeEvent::ZeroPosition => {
                    self.zero_current_position();
                }
                TradeEvent::ScheduleOrderLong(price) => {
                    println!("long limit order with price {}", price);
                    panic!("limit orders are not supported currently on backtests");
                }
                TradeEvent::ScheduleOrderShort(price) => {
                    println!("short limit order with price {}", price);
                    panic!("limit orders are not supported currently on backtests");
                }
                TradeEvent::None => ()
            }
        }
        self.event_index = 0;
    }

    fn check_compulsory_stop_loss(&mut self, candlestick: &Candlestick) {}

    fn check_scheduled_order_trigger(&mut self, candlestick: &Candlestick) {}

    fn execute_market_long_order(&mut self) {}

    fn zero_current_position(&mut self) {
        if self.current_position.is_none() {
            return;
        }
    }

    fn execute_market_short_order(&mut self) {
        match &mut self.current_position {
            None => {}
            Some(position) => {}
        }
    }

    fn calculate_entry_price_with_slippage(&self, candlestick: &Candlestick, side: Side) -> f64 {
        if side == Side::BUY {
            candlestick.close + (self.slippage_multiplier * candlestick.close)
        } else {
            candlestick.close - (self.slippage_multiplier * candlestick.close)
        }
    }

    fn calculate_exit_price_with_slippage(&self, candlestick: &Candlestick, side: Side) -> f64 {
        if side == Side::BUY {
            self.calculate_entry_price_with_slippage(candlestick, Side::SELL)
        } else {
            self.calculate_entry_price_with_slippage(candlestick, Side::BUY)
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
    pub fn run_backtest(&self, mut strategy: impl CandlestickTradingStrategy) -> BacktestMetrics {
        let mut event_handler = ExchangeEventHandler::new(self.asset);

        let first_day = self.data_provider.candlesticks.first().unwrap().date;

        // separate the first day of data to initialize the strategy
        let mut index_first_candle_second_day = 0;
        for (index, initialization_candle) in self.data_provider.candlesticks.iter().enumerate() {
            if initialization_candle.date != first_day {
                index_first_candle_second_day = index;
                break;
            }
        }

        strategy.init_trading_day(
            self.asset,
            &self.data_provider.candlesticks[0..index_first_candle_second_day],
        );

        for candlestick in self.data_provider.candlesticks.iter().skip(index_first_candle_second_day) {
            let trade_action = strategy.process_closed_candlestick(candlestick);
            event_handler.add_candlestick_event(&candlestick);
            add_orders_event(&mut event_handler, trade_action);
            event_handler.process_events();
        }

        BacktestMetrics {
            final_balance: 0.0,
            initial_balance: 0.0,
            total_return_percentage: 0.0,
            total_gross_profit: 0.0,
            total_liquid_profit: 0.0,
            avg_daily_liquid_profit: 0.0,
            worse_day_liquid_profit: 0.0,
            best_day_liquid_profit: 0.0,
            median_day_liquid_profit: 0.0,
            total_fees_paid: 0.0,
            minutes_in_the_market: 0.0,
            win_rate_percentage: 0.0,
            total_trades: 0,
            total_wins: 0,
            total_loses: 0,
            avg_risk_reward: 0.0,
            max_win_streak: 0,
            max_lose_streak: 0,
        }
    }

    /// Initializes the TradingBacktester
    /// The historical data will be loaded from the local disk or from AWS S3 if not available locally
    pub fn new(
        asset: &Asset,
        start_date: u32,
        end_date: u32,
        candlestick_configuration: CandlestickConfiguration,
    ) -> Result<TradingBacktester, anyhow::Error> {
        let start_date = date::convert_to_naive_date(start_date)?;
        let end_date = date::convert_to_naive_date(end_date)?;

        if end_date < start_date {
            anyhow::bail!("invalid dates, the end_date is before the start_date");
        }

        let history_trades_folder_path = home_dir()
            .join(configuration::CONFIGURATION_FOLDER)
            .join("history_trades")
            .join(asset.ticker.as_str());

        if !history_trades_folder_path.exists() {
            std::fs::create_dir_all(&history_trades_folder_path)?;
        }

        let exchange = asset.exchange.clone();
        let market = asset.market.clone();

        let mut trading_days = 0;
        let mut files_to_parse: Vec<String> = vec![];
        let aws_client = Arc::new(AwsClient::new("sa-east-1".to_string())?);

        let uppercase_ticker = asset.ticker.to_ascii_uppercase();
        let mut files_to_download: Vec<String> = vec![];

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
                    } else if aws_client.file_exists_on_s3(S3_BUCKET_HISTORICAL_DATA, file_path_on_s3.as_str())? == true {
                        trading_days += 1;
                        files_to_download.push(binary_trades_file_name);
                        files_to_parse.push(binary_trades_file_full_path.to_str().unwrap().to_string());
                    }
                }
            }
        }

        if files_to_download.len() > 0 {
            println!("download missing files with historical data from aws s3");
            let download_files_bar = Arc::new(Mutex::new(ProgressBar::default(files_to_download.len() as u32, 100, false)));
            let mut threads_handle_download_history_files: Vec<JoinHandle<Result<(), anyhow::Error>>> = vec![];
            for file_to_download in files_to_download {
                let thread_handle = download_binary_trades_file(
                    aws_client.clone(),
                    Arc::clone(&download_files_bar),
                    format!("trades/{}/{}", uppercase_ticker, file_to_download),
                    history_trades_folder_path.to_str().unwrap().to_string(),
                    file_to_download,
                );
                threads_handle_download_history_files.push(thread_handle);
            }

            for thread_handle in threads_handle_download_history_files {
                match thread_handle.join() {
                    Ok(result) => {
                        if result.is_err() {
                            panic!("failed to download history file from aws s3. {}", result.err().unwrap());
                        }
                    }
                    Err(_) => {
                        panic!("thread failed to complete successfully.");
                    }
                }
            }
        }

        println!("parsing binary files with historical data from local disk");
        let parse_candles_bar = Arc::new(Mutex::new(ProgressBar::default(files_to_parse.len() as u32, 100, false)));

        let mut candlesticks: Vec<Vec<Candlestick>> = Vec::new();
        files_to_parse
            .into_par_iter()
            .map(|binary_trade_file| {
                let bar = Arc::clone(&parse_candles_bar);
                let configuration = candlestick_configuration.clone();
                let mut candlesticks_current_file =
                    parse_candles_from_file(binary_trade_file, configuration)
                        .expect("should not fail when parsing candlesticks from trades file");
                let mut parse_progress_bar = bar.lock().unwrap();
                parse_progress_bar.update();
                candlesticks_current_file
            }).collect_into_vec(&mut candlesticks);

        let candlesticks: Vec<Candlestick> = candlesticks.into_iter().flatten().collect();

        Ok(TradingBacktester {
            asset,
            qty_history_trading_days: trading_days,
            qty_history_candles: candlesticks.len() as u64,
            qty_history_trades: candlesticks.iter()
                .fold(0, |total, candle| total + candle.trades),
            data_provider: DataProvider::new(candlesticks, candlestick_configuration),
        })
    }
}


fn download_binary_trades_file(
    aws_client: Arc<AwsClient>,
    progress_bar: Arc<Mutex<ProgressBar>>,
    file_path_on_s3: String,
    output_folder_on_disk: String,
    file_name: String,
) -> JoinHandle<Result<(), anyhow::Error>> {
    let bar = Arc::clone(&progress_bar);
    thread::spawn(move || {
        let file_output_path_on_disk = format!("{}/{}",
                                               output_folder_on_disk,
                                               file_name.as_str()
        );
        let file_download_result = aws_client.download_file_from_s3(S3_BUCKET_HISTORICAL_DATA,
                                                                    file_path_on_s3.as_str(),
                                                                    file_output_path_on_disk.as_str(),
        );

        let mut download_progress_bar = bar.lock().unwrap();
        download_progress_bar.update();

        file_download_result
    })
}


fn parse_candles_from_file(
    binary_trades_file_full_path: String,
    candlestick_configuration: CandlestickConfiguration,
) -> Result<Vec<Candlestick>, anyhow::Error> {
    let binary_trades_file_full_path = PathBuf::from(binary_trades_file_full_path);

    if !binary_trades_file_full_path.exists() {
        anyhow::bail!("binary trades file {} doesnt exist on disk",
            binary_trades_file_full_path.to_str().unwrap());
    }

    let trades = trade::read_trades_from_binary_file(
        binary_trades_file_full_path.to_str().unwrap());

    if trades.is_err() {
        anyhow::bail!("failed to read trades from local file {}. err: {}",
            binary_trades_file_full_path.to_str().unwrap(), trades.err().unwrap());
    }

    let mut candlestick_parser = candlestick::select_candlestick_generator(candlestick_configuration);
    let mut candlesticks: Vec<Candlestick> = vec![];

    for trade in trades? {
        if let Some(candle) = candlestick_parser.parse_only_closed_candles(&trade) {
            candlesticks.push(candle);
        }
    }

    Ok(candlesticks)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_calculate_simple_buy_long_and_close_long_correctly() {}

    fn test_candlesticks() -> Vec<Candlestick> {
        vec![
            new_candlestick(130_660.0, 130_700.0, 130_600.0, 130_680.0, 12222.0),
            new_candlestick(130_680.0, 130_750.0, 130_680.0, 130_710.0, 12222.0),
            new_candlestick(130_710.0, 130_750.0, 130_600.0, 130_600.0, 12222.0),
            new_candlestick(130_600.0, 130_635.0, 130_585.0, 130_590.0, 12222.0),
            new_candlestick(130_590.0, 130_620.0, 130_580.0, 130_600.0, 12222.0),
            new_candlestick(130_600.0, 130_635.0, 130_585.0, 130_590.0, 12222.0),
        ]
    }

    fn new_candlestick(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candlestick {
        Candlestick {
            open,
            high,
            low,
            close,
            volume,
            date: 20241107,
            start_time: 113242,
            is_closed: true,
            trades: 50,
        }
    }
}



