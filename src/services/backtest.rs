use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{cmp};
use std::fmt::{Display, Formatter};
use chrono::{Datelike, NaiveDate, Weekday};
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::{candlestick};
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::pnl::Position;
use crate::services::aws::AwsClient;
use crate::services::backtest::TradeEvent::NewCandle;
use crate::services::configuration::{kate_configuration_directory, S3_BUCKET_HISTORICAL_DATA};
use crate::strategies::strategy::{CandlestickConfiguration, CandlestickTradingStrategy};
use crate::utils::date;
use simple_bar::ProgressBar;
use crate::models::side::Side;
use rayon::prelude::*;
use crate::models::market::Market;
use crate::models::trade::TradeFile;

const INITIAL_BALANCE: f64 = 10_000.0;
const BASE_TRADE_QTY: u32 = 1;
const DEFAULT_SLIPPAGE_MULTIPLIER: f64 = 0.0001;
const COMPULSORY_CLOSE_FEE: f64 = 25.0;

#[allow(unused)]
pub struct BacktestMetrics {
    pub total_trading_days: u64,
    pub final_balance: f64,
    pub initial_balance: f64,
    pub total_return_percentage: f64,
    pub total_gross_profit: f64,
    pub total_liquid_profit: f64,
    pub avg_trade_liquid_pnl: f64,
    pub avg_daily_liquid_profit: f64,
    pub worse_day_liquid_profit: f64,
    pub best_day_liquid_profit: f64,
    pub median_day_liquid_profit: f64,
    pub total_fees_paid: f64,
    pub minutes_in_the_market: f64,
    pub win_rate_percentage: f64,
    pub total_trades: u64,
    pub total_orders: u64,
    pub total_wins: u64,
    pub total_loses: u64,
    pub avg_risk_reward: f32,
    pub max_win_streak: i32,
    pub max_lose_streak: i32,
}

impl Display for BacktestMetrics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{0: <25} {1: <25.2?}", "Trading Days", self.total_trading_days)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Start Balance", INITIAL_BALANCE)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Final Balance", self.final_balance)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Absolute Return", self.final_balance - INITIAL_BALANCE)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Total Return(%)", self.total_return_percentage)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Fees Paid", self.total_fees_paid)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Win Rate(%)", self.win_rate_percentage)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Total Wins", self.total_wins)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Max Win Streak", self.max_win_streak)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Total Loses", self.total_loses)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Max Lose Streak", self.max_lose_streak)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Total Trades", self.total_trades)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Liquid PNL Best Day", self.best_day_liquid_profit)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Liquid PNL Worst Day", self.worse_day_liquid_profit)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Avg Daily Liquid PNL", self.avg_daily_liquid_profit)?;
        writeln!(f, "{0: <25} {1: <25.2?}", "Avg Trade Liquid PNL", self.avg_trade_liquid_pnl)
    }
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
    base_qty_per_order: u32,
    total_candlesticks_processed: u64,
    current_position: Option<Position>,
    scheduled_orders: Vec<TradeEvent<'a>>,
    events: [TradeEvent<'a>; 4],
    event_index: usize,
    positions: Vec<Position>,
    current_candlestick: Option<&'a Candlestick>,
    total_contracts_executed_current_month: u64,
    current_month_year: i32,
    last_candlestick: Option<&'a Candlestick>,
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
#[allow(unused)]
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
            current_position: None,
            current_candlestick: None,
            last_candlestick: None,
            current_balance: INITIAL_BALANCE,
            base_qty_per_order: BASE_TRADE_QTY,
            slippage_multiplier: DEFAULT_SLIPPAGE_MULTIPLIER,
            total_candlesticks_processed: 0,
            events: [TradeEvent::None; 4],
            event_index: 0,
            positions: vec![],
            scheduled_orders: vec![],
            total_contracts_executed_current_month: 0,
            current_month_year: -1,
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
        for event in self.events {
            match event {
                NewCandle(candlestick) => {
                    self.last_candlestick = self.current_candlestick;
                    self.current_candlestick = Some(candlestick);
                    self.track_current_month();
                    self.check_compulsory_close_position(candlestick);
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
                TradeEvent::ScheduleOrderLong(_) | TradeEvent::ScheduleOrderShort(_) => {
                    panic!("operation not allowed");
                    // self.scheduled_orders.push(event);
                }
                TradeEvent::None => ()
            }
        }
        for index in 0..self.events.len() {
            self.events[index] = TradeEvent::None;
        }
        self.event_index = 0;
    }

    fn check_compulsory_close_position(&mut self, candlestick: &Candlestick) {
        if self.current_position.is_none() {
            return;
        }
        let position = self.current_position.as_ref().unwrap();
        let price_diff = match position.side {
            Side::BUY => candlestick.low - position.avg_entry_price,
            Side::SELL => candlestick.high - position.avg_entry_price
        };
        let pnl_from_spikes = (price_diff / self.asset.min_variation) * position.open_qty as f64 * self.asset.value_per_variation;
        if pnl_from_spikes >= self.current_balance {
            self.current_balance = 0.0;
        }

        // we don't allow holding positions to the next trading day, this is a force close in case the position
        // is still open when the trading day ends
        let position = self.current_position.as_mut();
        if position.is_some() && candlestick.date != self.last_candlestick.unwrap().date {
            let position = position.unwrap();
            let open_qty = position.open_qty;
            let last_candle = *self.last_candlestick.as_ref().unwrap();
            position.total_paid_in_fees+=COMPULSORY_CLOSE_FEE;
            self.current_balance-=COMPULSORY_CLOSE_FEE;
            self.decrease_position(last_candle.close, last_candle, open_qty);
        }
    }

    #[allow(unused)]
    fn check_scheduled_order_trigger(&mut self, candlestick: &Candlestick) {
        if self.scheduled_orders.len() > 0 {
            panic!("currently processing limit orders is not supported on backtests");
        }
    }

    fn execute_market_long_order(&mut self) {
        self.execute_market_order(Side::BUY);
    }

    fn execute_market_short_order(&mut self) {
        self.execute_market_order(Side::SELL);
    }

    fn execute_market_order(&mut self, direction: Side) {
        let current_candle = self.current_candlestick.unwrap();
        match &mut self.current_position {
            None => {
                let entry_price = self.calculate_price_with_slippage(current_candle, direction.clone());
                self.open_new_position(entry_price, direction, current_candle);
            }
            Some(position) => {
                if position.side == direction {
                    let entry_price = self.calculate_price_with_slippage(current_candle, direction);
                    self.increase_position(entry_price);
                } else {
                    let oposit_direction = if direction == Side::BUY {
                        Side::SELL
                    } else {
                        Side::BUY
                    };
                    let entry_price = self.calculate_price_with_slippage(current_candle, oposit_direction);
                    self.decrease_position(entry_price, current_candle, self.base_qty_per_order as i32);
                }
            }
        }
    }

    fn zero_current_position(&mut self) {
        assert!(self.current_position.is_some(), "sanity check, should not try to close a position that doesnt exit");
    }

    /// slippage is a multiplier that makes the price bit worse when executing an order in the selected direction
    /// this aims to emulate the real behavior of the market, since between seen the price, executing the order and
    /// the conclusion in the exchange the actual execution price may ( and probably will be ) slight different
    fn calculate_price_with_slippage(&self, candlestick: &Candlestick, side: Side) -> f64 {
        if side == Side::BUY {
            candlestick.close + (self.slippage_multiplier * candlestick.close)
        } else {
            candlestick.close - (self.slippage_multiplier * candlestick.close)
        }
    }

    fn increase_position(&mut self, current_entry_price: f64) {
        assert!(self.current_position.is_some(), "sanity check, should not try to increase a position that doesnt exit");

        let current_fee = &self.calculate_fees(self.base_qty_per_order as u64);
        let position = self.current_position.as_mut().unwrap();
        let new_total = position.total_qty + self.base_qty_per_order as i32;
        position.avg_entry_price = ((position.avg_entry_price * position.total_qty as f64) + (current_entry_price * self.base_qty_per_order as f64)) / new_total as f64;
        position.total_qty = new_total;
        position.open_qty += new_total;

        position.total_paid_in_fees += current_fee;
        self.total_contracts_executed_current_month += self.base_qty_per_order as u64;
        self.current_balance -= current_fee;
    }

    //self.base_qty_per_order
    fn decrease_position(&mut self, current_exit_price: f64, candlestick: &Candlestick, quantity_to_decrease: i32){
        assert!(self.current_position.is_some(), "sanity check, should not try to decrease a position that doesnt exit");
        let current_fee = self.calculate_fees(quantity_to_decrease as u64);
        let position = self.current_position.as_mut().unwrap();
        assert!(position.open_qty > 0, "cant decrease a position that doesnt have any contract/shares left open");
        assert!(quantity_to_decrease <= position.open_qty, "can´t decrease a position more contracts/shares than it has");

        let executed_qty = position.total_qty - position.open_qty;
        position.avg_exit_price = ((position.avg_exit_price * executed_qty as f64) + (current_exit_price * quantity_to_decrease as f64) )/ position.total_qty as f64;
        position.open_qty -= quantity_to_decrease;

        position.total_paid_in_fees+= current_fee;
        self.current_balance-=current_fee;
        self.total_contracts_executed_current_month+=quantity_to_decrease as u64;

        if position.open_qty == 0 {
            self.close_position(candlestick);
        }
    }

    fn track_current_month(&mut self) {
        let current_month_year = self.current_candlestick.as_ref().unwrap().date / 100;
        if self.current_month_year == -1 {
            self.current_month_year = current_month_year;
        }

        if self.current_month_year != current_month_year {
            self.current_month_year = current_month_year;
            self.total_contracts_executed_current_month = 0;
        }
    }

    fn open_new_position(&mut self, entry_price: f64, side: Side, candlestick: &Candlestick) {
        assert!(self.current_position.is_none(), "sanity check, should not try open a position without closing the existing one");
        let fee_open_position = self.calculate_fees(self.base_qty_per_order as u64);
        let new_position = Position {
            side,
            avg_entry_price: entry_price,
            avg_exit_price: 0.0,
            date: candlestick.date,
            entry_time: candlestick.close_time,
            exit_time: -1,
            start_datapoint_number: self.total_candlesticks_processed as i32,
            end_datapoint_number: 0,
            total_qty: self.base_qty_per_order as i32,
            open_qty: self.base_qty_per_order as i32,
            gross_pnl: 0.0,
            liquid_pnl: 0.0,
            total_paid_in_fees: fee_open_position,
            quantity_of_orders: self.base_qty_per_order,
            is_closed: false,
        };
        self.current_position = Some(new_position);
        self.current_balance -= fee_open_position;
        self.total_contracts_executed_current_month += self.base_qty_per_order as u64;
    }

    fn close_position(&mut self, candlestick: &Candlestick) {
        assert!(self.current_position.is_some(), "sanity check, should not try to close a position that doesnt exit");
        let mut closed_position = self.current_position.as_ref().unwrap().clone();
        assert_eq!(0, closed_position.open_qty, "cant close a position that has open contracts/shares in the market");

        closed_position.is_closed = true;
        closed_position.exit_time = candlestick.close_time;
        closed_position.gross_pnl = Self::calculate_gross_realized_pnl(&closed_position, self.asset);
        closed_position.liquid_pnl = closed_position.gross_pnl - closed_position.total_paid_in_fees;
        closed_position.end_datapoint_number = self.total_candlesticks_processed as i32;
        closed_position.open_qty = 0;

        self.current_balance += closed_position.gross_pnl;
        self.positions.push(closed_position);
        self.current_position = None;
    }

    fn calculate_gross_realized_pnl(position: &Position, asset: &Asset) -> f64 {
        let price_diff = match position.side {
            Side::BUY => position.avg_exit_price - position.avg_entry_price,
            Side::SELL => position.avg_entry_price - position.avg_exit_price
        };
        (price_diff / asset.min_variation) * (position.total_qty - position.open_qty) as f64 * asset.value_per_variation
    }

    fn calculate_fees(&self, total_orders: u64) -> f64 {
        let asset = self.asset;
        let brokerage = if asset.market == Market::FUTURES && asset.ticker == "WIN" || asset.ticker == "WDO" {
            match self.total_contracts_executed_current_month {
                0_u64..=50_u64 => 0.95,
                51_u64..=100_u64 => 0.90,
                101_u64..=200_u64 => 0.87,
                201_u64..=500_u64 => 0.76,
                501_u64..=2500_u64 => 0.63,
                2501_u64..=5000_u64 => 0.33,
                5001_u64..=15000_u64 => 0.12,
                15001u64.. => 0.05
            }
        } else {
            panic!("no brokerage rate defined for asset with ticker {}", asset.ticker);
        };

        let iss_percentage = 0.067;
        let iss_amount = iss_percentage * brokerage * total_orders as f64;
        let exchange_registration_fees = if asset.ticker == "WIN" {
            0.16 * total_orders as f64
        } else if asset.ticker == "WDO" {
            0.75 * total_orders as f64
        } else {
            panic!("no exchange registration fee defined for asset with ticker {}", asset.ticker);
        };

        let exchange_emolument_fees = if asset.ticker == "WIN" {
            0.09 * total_orders as f64
        } else if asset.ticker == "WDO" {
            0.40 * total_orders as f64
        } else {
            panic!("no exchange registration fee defined for asset with ticker {}", asset.ticker);
        };

        brokerage + iss_amount + exchange_registration_fees + exchange_emolument_fees
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


        let mut days_of_context = strategy.days_of_context();
        let mut current_day = self.data_provider.candlesticks.first().unwrap().date;

        // separate the first day of data to initialize the strategy
        let mut index_initial_candle_to_trade = 0;
        for (index, initialization_candle) in self.data_provider.candlesticks.iter().enumerate() {
            if initialization_candle.date != current_day {
                current_day = initialization_candle.date;
                days_of_context -= 1;
                if days_of_context == 0 {
                    index_initial_candle_to_trade = index;
                    break;
                }
            }
        }

        strategy.init_trading_day(
            self.asset,
            &self.data_provider.candlesticks[0..index_initial_candle_to_trade],
        );

        for candlestick in self.data_provider.candlesticks.iter().skip(index_initial_candle_to_trade) {
            let trade_action = strategy.process_closed_candlestick(candlestick);
            event_handler.add_candlestick_event(&candlestick);
            add_orders_event(&mut event_handler, trade_action);
            event_handler.process_events();

            if event_handler.current_balance <= 0.0 {
                break;
            }
        }

        self.calculate_backtest_results(&event_handler)
    }

    /// Initializes the TradingBacktester
    /// The historical data will be loaded from the local disk or from AWS S3 if not available locally
    pub fn new(
        asset: &Asset,
        start_date: u32,
        end_date: u32,
        candlestick_configuration: CandlestickConfiguration,
        sequential_download: bool,
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
        let aws_client = Arc::new(AwsClient::new("sa-east-1".to_string())?);

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
                    } else if aws_client.file_exists_on_s3(S3_BUCKET_HISTORICAL_DATA, file_path_on_s3.as_str())? == true {
                        trading_days += 1;
                        files_to_download.push(binary_trades_file_name);
                        files_to_parse.push(binary_trades_file_full_path.to_str().unwrap().to_string());
                    }
                }
            }
        }

        if files_to_download.len() > 0 {
            if !sequential_download {
                let number_of_threads = rayon::current_num_threads();
                println!("download in parallel missing files with historical data from aws s3 using {} threads", number_of_threads);
                let mut bar =  ProgressBar::default(files_to_download.len() as u32 +1, 100, false);
                bar.update();
                let download_files_bar = Arc::new(Mutex::new(bar));
                let values_to_process: Vec<(&String, Arc<Mutex<ProgressBar>>)> = files_to_download.iter().map(|files_to_download| {
                    (files_to_download, Arc::clone(&download_files_bar))
                }).collect();

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
            } else {
                println!("download sequentially missing files with historical data from aws s3");
                let mut download_progress_bar = ProgressBar::default(files_to_download.len() as u32 + 1, 100, false);
                download_progress_bar.update();
                for file_to_download in files_to_download {
                    download_binary_trades_file(
                        aws_client.clone(),
                        format!("trades/{}/{}", uppercase_ticker, file_to_download),
                        history_trades_folder_path.to_str().unwrap().to_string(),
                        file_to_download.as_str(),
                    ).expect(format!("failed to download file {}", file_to_download).as_str());
                    download_progress_bar.update();
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
                let candlesticks_current_file =
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

    fn calculate_backtest_results(&self, event_handler: &ExchangeEventHandler) -> BacktestMetrics {
        let mut total_trades = 0;
        let mut total_orders: u64 = 0;
        let mut total_fees_paid = 0.0;
        let mut total_liquid_pnl = 0.0;
        let mut total_gross_pnl = 0.0;
        let mut total_wins = 0;
        let mut total_loses = 0;

        let mut max_win_streak = 0;
        let mut max_lose_streak = 0;
        let mut current_win_lose_streak: i32 = 0;


        let mut liquid_profit_per_day = vec![];
        let mut liquid_profit_current_day = 0.0;
        let mut liquid_profit_best_day = 0.0;
        let mut liquid_profit_worst_day = 0.0;
        let mut current_day = if event_handler.positions.len() > 0 {
            event_handler.positions.first().unwrap().date
        } else {
            -1
        };
        let mut total_trading_days = 1;
        for position in &event_handler.positions {
            if position.date != current_day {
                liquid_profit_per_day.push(liquid_profit_current_day);
                liquid_profit_current_day = 0.0;
                total_trading_days += 1;
            }

            total_trades += 1;
            total_liquid_pnl += position.liquid_pnl;
            total_gross_pnl += position.gross_pnl;
            total_orders += position.quantity_of_orders as u64;
            total_fees_paid += position.total_paid_in_fees;
            liquid_profit_current_day += position.liquid_pnl;
            current_day = position.date;

            if position.liquid_pnl > 0.0 {
                total_wins += 1;
                if current_win_lose_streak < 0 {
                    max_lose_streak = cmp::max(max_lose_streak, -current_win_lose_streak);
                    current_win_lose_streak = 1;
                } else {
                    current_win_lose_streak += 1;
                    max_win_streak = cmp::max(max_win_streak, current_win_lose_streak);
                }
            } else {
                total_loses += 1;
                if current_win_lose_streak > 0 {
                    max_win_streak = cmp::max(max_win_streak, current_win_lose_streak);
                    current_win_lose_streak = -1;
                } else {
                    current_win_lose_streak -= 1;
                    max_lose_streak = cmp::max(max_lose_streak, -current_win_lose_streak);
                }
            }

            if position.liquid_pnl > liquid_profit_best_day {
                liquid_profit_best_day = position.liquid_pnl;
            }
            if position.liquid_pnl < liquid_profit_worst_day {
                liquid_profit_worst_day = position.liquid_pnl;
            }
        }

        BacktestMetrics {
            final_balance: event_handler.current_balance,
            initial_balance: INITIAL_BALANCE,
            total_return_percentage: ((event_handler.current_balance - INITIAL_BALANCE) / INITIAL_BALANCE) * 100.0,
            total_gross_profit: total_gross_pnl,
            total_liquid_profit: total_liquid_pnl,
            avg_trade_liquid_pnl: total_liquid_pnl / total_trades as f64,
            avg_daily_liquid_profit: total_liquid_pnl / total_trading_days as f64,
            worse_day_liquid_profit: liquid_profit_worst_day,
            best_day_liquid_profit: liquid_profit_best_day,
            median_day_liquid_profit: 0.0,
            total_fees_paid,
            minutes_in_the_market: 0.0,
            win_rate_percentage: total_wins as f64 / total_trades as f64,
            total_trades,
            total_orders,
            total_wins,
            total_loses,
            avg_risk_reward: 0.0,
            max_win_streak,
            max_lose_streak,
            total_trading_days,
        }
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


fn parse_candles_from_file(
    binary_trades_file_full_path: String,
    candlestick_configuration: CandlestickConfiguration,
) -> Result<Vec<Candlestick>, anyhow::Error> {
    let binary_trades_path = PathBuf::from(binary_trades_file_full_path.clone());

    if !binary_trades_path.exists() {
        anyhow::bail!("binary trades file {} doesnt exist on disk",
            binary_trades_path.to_str().unwrap());
    }

    let trades_file = TradeFile::new_from_binary_file(binary_trades_file_full_path.as_str())?;

    let mut candlestick_parser = candlestick::select_candlestick_generator(candlestick_configuration);
    let mut candlesticks: Vec<Candlestick> = vec![];

    for trade in trades_file {
        if let Some(candle) = candlestick_parser.parse_only_closed_candles(&trade) {
            candlesticks.push(candle);
        }
    }

    Ok(candlesticks)
}
