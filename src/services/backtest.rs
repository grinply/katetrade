use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{cmp};
use std::fmt::{Display, Formatter};
use chrono::{Datelike, NaiveDate, Weekday};
use itertools::{enumerate};
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::pnl::Position;
use crate::services::aws::AwsClient;
use crate::services::backtest::TradeEvent::{NewCandle, NewTrade};
use crate::services::configuration::{kate_configuration_directory, S3_BUCKET_HISTORICAL_DATA};
use crate::strategies::strategy::{CandlestickTradingStrategy, HighResolutionTradingStrategy};
use crate::utils::{date, math};
use simple_bar::ProgressBar;
use crate::models::side::Side;
use rayon::prelude::*;
use crate::models::market::Market;
use crate::models::trade::{Trade, TradeFile};

pub const INITIAL_BALANCE: f64 = 10_000.0;
const MINIMUM_BALANCE: f64 = 800.0;
const DEFAULT_SLIPPAGE_MULTIPLIER: f64 = 0.0001;
const COMPULSORY_CLOSE_FEE: f64 = 25.0;
const TRADING_DAYS_IN_A_YEAR: u32 = 250;
const YEARLY_RISK_FREE_RATE: f64 = 10.5; // Avg yield for the Brazillian Selic BOND from 2009 to 2024
const DAILY_RISK_FREE_RATE: f64 = YEARLY_RISK_FREE_RATE / TRADING_DAYS_IN_A_YEAR as f64;

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
    pub avg_long_trade_liquid_pnl: f64,
    pub avg_long_short_liquid_pnl: f64,
    pub best_day_liquid_profit: f64,
    pub sharp_ratio: f64,
    pub median_day_liquid_profit: f64,
    pub median_minutes_in_the_market_per_day: u64,
    pub total_fees_paid: f64,
    pub minutes_in_the_market: u64,
    pub win_rate_percentage: f64,
    pub total_trades: u64,
    pub total_long_trades: u64,
    pub total_short_trades: u64,
    pub quantity_of_contracts: u64,
    pub total_wins: u64,
    pub total_loses: u64,
    pub avg_risk_reward: f32,
    pub max_win_streak: i32,
    pub max_lose_streak: i32,
    pub max_draw_down_percentage: f64,
    pub avg_winning_trade: f64,
    pub avg_losing_trade: f64,
    pub profit_factor: f64,
    pub profit_factor_log_sum: f64
}

impl Display for BacktestMetrics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let avg_contracts_traded = if self.total_trades == 0 {
            0
        }else{
            self.quantity_of_contracts / (2 * self.total_trades)
        };

        writeln!(f, "{0: <30} {1: <25.2?}", "Trading Days", self.total_trading_days)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Start Balance", INITIAL_BALANCE)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Final Balance", self.final_balance)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Absolute Return", self.final_balance - INITIAL_BALANCE)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Return(%)", self.total_return_percentage)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Sharp Ratio", self.sharp_ratio)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Profit Factor", self.profit_factor)?;
        writeln!(f, "{0: <30} {1: <25}", "Sharp Ratio Description", evaluate_sharp_ratio(self.sharp_ratio))?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Max DrawDown(%)", self.max_draw_down_percentage)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Fees Paid", self.total_fees_paid)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Win Rate(%)", self.win_rate_percentage)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Wins", self.total_wins)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Max Win Streak", self.max_win_streak)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Loses", self.total_loses)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Max Lose Streak", self.max_lose_streak)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Trades", self.total_trades)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Long Trades", self.total_long_trades)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Short Trades", self.total_short_trades)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Contracts Executed", self.quantity_of_contracts)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Contracts per Trade", avg_contracts_traded)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Liquid PNL Best Day", self.best_day_liquid_profit)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Liquid PNL Worst Day", self.worse_day_liquid_profit)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Daily Liquid PNL", self.avg_daily_liquid_profit)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Winning Trade PNL", self.avg_winning_trade)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Losing Trade PNL", self.avg_losing_trade)
    }
}

fn evaluate_sharp_ratio(sharp_ratio: f64) -> String {
    match sharp_ratio {
        (..1.0) => "Sub-optimal".to_string(),
        (1.0..2.0) => "Acceptable".to_string(),
        (2.0..3.0) => "Very Good".to_string(),
        (3.0..) => "Excellent".to_string(),
        _ => unreachable!()
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
    trades: Vec<Trade>
}

#[allow(unused)]
struct ExchangeEventHandler<'a> {
    asset: &'a Asset,
    current_balance: f64,
    slippage_multiplier: f64, // defaults to 0.0001
    initial_qty_per_order: u32,
    base_qty_per_order: u32,
    total_candlesticks_processed: u64,
    current_position: Option<Position>,
    scheduled_orders: Vec<TradeEvent<'a>>,
    events: [TradeEvent<'a>; 4],
    event_index: usize,
    positions: Vec<Position>,
    total_contracts_executed_current_month: u64,
    current_month_year: i32,
    current_datapoint: Option<Box<&'a dyn DataPoint>>,
    last_datapoint: Option<Box<&'a dyn DataPoint>>,
    fixed_traded_qty: bool,
    removed_for_lack_of_margin: bool,
}

impl DataProvider {
    fn new(candlesticks: Vec<Candlestick>, trades: Vec<Trade>) -> Self {
        DataProvider {
            candlesticks,
            trades
        }
    }
}

pub trait DataPoint {
    fn date(&self) -> i32;

    #[allow(unused)]
    fn time(&self) -> i32;

    fn exit_time(&self) -> i32;

    fn current_price(&self) -> f64;
}

#[derive(Copy, Clone, Debug)]
#[allow(unused)]
enum TradeEvent<'a> {
    NewCandle(&'a Candlestick),
    NewTrade(&'a Trade),
    ExecuteOrderLong,
    ExecuteOrderShort,
    ZeroPosition,
    ScheduleOrderLong(f64),
    ScheduleOrderShort(f64),
    Abort,
    None,
}

impl<'a> ExchangeEventHandler<'a> {
    fn new(asset: &'a Asset, base_qty_to_trade: u32, fixed_traded_qty: bool) -> Self {
        ExchangeEventHandler {
            asset,
            current_position: None,
            current_datapoint: None,
            last_datapoint: None,
            current_balance: INITIAL_BALANCE,
            base_qty_per_order: base_qty_to_trade,
            initial_qty_per_order: base_qty_to_trade,
            slippage_multiplier: DEFAULT_SLIPPAGE_MULTIPLIER,
            total_candlesticks_processed: 0,
            events: [TradeEvent::None; 4],
            event_index: 0,
            positions: vec![],
            scheduled_orders: vec![],
            total_contracts_executed_current_month: 0,
            current_month_year: -1,
            fixed_traded_qty,
            removed_for_lack_of_margin: false,
        }
    }

    fn add_trade_event(&mut self, trade: &'a Trade) {
        self.events[self.event_index] = NewTrade(trade);
        self.event_index += 1;
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
            OrderAction::Abort => TradeEvent::Abort
        };
        self.events[self.event_index] = order_event;
        self.event_index += 1;
    }

    fn process_events(&mut self) {
        for event in self.events {
            match event {
                NewTrade(trade) => {
                    self.new_data_point(Box::new(trade));
                }
                NewCandle(candlestick) => {
                    self.new_data_point(Box::new(candlestick));
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
                TradeEvent::Abort => {
                    self.removed_for_lack_of_margin = true;
                }
                TradeEvent::None => (),
            }
        }
        for index in 0..self.events.len() {
            self.events[index] = TradeEvent::None;
        }
        self.event_index = 0;
    }

    fn new_data_point(&mut self, datapoint: Box<&'a (dyn DataPoint + 'a)>) {
        self.last_datapoint = self.current_datapoint.to_owned();
        self.current_datapoint = Some(datapoint.to_owned());

        self.track_current_month();

        self.should_updated_traded_qty(&datapoint);
        self.check_compulsory_close_position(&datapoint);
        self.check_scheduled_order_trigger(&datapoint);
        self.track_mark_to_market(&datapoint);
    }

    fn track_mark_to_market(&mut self, new_datapoint: &Box<&dyn DataPoint>) {
        if let Some(open_position) = self.current_position.as_mut() {
            let diff = new_datapoint.current_price().log10() - self.last_datapoint.as_ref()
                .unwrap().current_price().log10();

            if diff < 0.0 {
                if open_position.side == Side::BUY {
                    // price decrease is a loss for long positions
                    open_position.sum_per_candle_log_lose_returns+=-1.0*diff;
                    open_position.sum_per_candle_total_log_returns+=diff;
                }else{
                    // price decrease is a win for short positions
                    open_position.sum_per_candle_log_win_returns+=-1.0*diff;
                    open_position.sum_per_candle_total_log_returns+=-1.0*diff;
                }
            }else{
                if open_position.side == Side::BUY {
                    // price increase is a win for long positions
                    open_position.sum_per_candle_log_win_returns+=diff;
                    open_position.sum_per_candle_total_log_returns+=diff;
                }else{
                    // price increase is a loss for short positions
                    open_position.sum_per_candle_log_lose_returns+=diff;
                    open_position.sum_per_candle_total_log_returns-=diff;
                }
            }
        }
    }

    fn should_updated_traded_qty(&mut self, datapoint: &Box<&dyn DataPoint>) {
        if self.fixed_traded_qty {
            return;
        }

        // we may adjust the traded qty in each new trading day
        // the amount traded is proportional to the initial balance
        // if the initial_qty_per_order is 5 contracts and given the initial balance of 10.000
        // if on the next trading day the current balance is 20.000 the traded qty will be 10 contracts
        // keeping the proportion as the balance changes
        if self.last_datapoint.is_some() && datapoint.date() != self.last_datapoint.as_ref().unwrap().date() {
            self.base_qty_per_order = (self.current_balance as u32 * self.initial_qty_per_order) / INITIAL_BALANCE as u32;

            // this is the minimum quantity to trade we will never trade less than this amount
            if self.base_qty_per_order < self.initial_qty_per_order {
                self.base_qty_per_order = self.initial_qty_per_order;
            }
        }
    }


    /// close an existing open existing position in the following cases:
    /// 1) the day is change and the position is open ( we don't allow overnight positions to be open )
    /// 2) the PNL of the position is negative and reached the minimum balance allowed
    /// a compulsory close fee of 25$ is applied if any such cases occur
    fn check_compulsory_close_position(&mut self, datapoint: &Box<&dyn DataPoint>) {
        if self.current_position.is_none() {
            return;
        }

        let position = self.current_position.as_mut().unwrap();

        // use high or low instead of close
        let gross_pnl = Self::calculate_gross_open_pnl(position, self.asset, datapoint.current_price());

        if datapoint.date() != self.last_datapoint.as_ref().unwrap().date() ||
            (gross_pnl < 0.0 && -1.0 * gross_pnl >= MINIMUM_BALANCE) {
            let open_qty = position.open_qty;
            let last_datapoint = self.last_datapoint.as_ref().unwrap();
            position.total_paid_in_fees += COMPULSORY_CLOSE_FEE;
            self.current_balance -= COMPULSORY_CLOSE_FEE;
            self.decrease_position(last_datapoint.current_price(), last_datapoint.to_owned(), open_qty);
        }
    }

    #[allow(unused)]
    fn check_scheduled_order_trigger(&mut self, candlestick: &Box<&dyn DataPoint>) {
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
        let datapoint = self.current_datapoint.as_ref().unwrap();
        let entry_price = self.calculate_price_with_slippage(datapoint.current_price(), direction.clone());
        match &mut self.current_position {
            None => self.open_new_position(entry_price, direction, datapoint.to_owned()),
            Some(position) => {
                if position.side == direction {
                    self.increase_position(entry_price);
                } else {
                    self.decrease_position(entry_price, datapoint.to_owned(), self.base_qty_per_order as i32);
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
    fn calculate_price_with_slippage(&self, price: f64, side: Side) -> f64 {
        if side == Side::BUY {
            price + (self.slippage_multiplier * price)
        } else {
            price - (self.slippage_multiplier * price)
        }
    }

    fn increase_position(&mut self, current_entry_price: f64) {
        assert!(self.current_position.is_some(), "sanity check, should not try to increase a position that doesnt exit");
        if !self.has_margin_execute_order() {
            self.removed_for_lack_of_margin = true;
            return;
        }
        let quantity_to_increase = self.base_qty_per_order as i32;


        let current_fee = &self.calculate_fees(quantity_to_increase as u64);
        let position = self.current_position.as_mut().unwrap();
        let new_total = position.total_qty + quantity_to_increase;

        position.avg_entry_price = ((position.avg_entry_price * position.total_qty as f64) + (current_entry_price * self.base_qty_per_order as f64)) / new_total as f64;
        assert!(position.avg_entry_price > 0.0);

        position.total_qty = new_total;
        position.open_qty += quantity_to_increase;

        position.quantity_of_contracts += self.base_qty_per_order;
        position.total_paid_in_fees += current_fee;
        self.total_contracts_executed_current_month += quantity_to_increase as u64;
        self.current_balance -= current_fee;

        assert!(position.total_qty >= position.open_qty, "open quantity cannot be greater than total quantity");
    }

    fn has_margin_execute_order(&self) -> bool {
        self.asset.margin_per_contract * (self.base_qty_per_order as f64) < self.current_balance
    }

    fn decrease_position(&mut self, current_exit_price: f64, datapoint: Box<&dyn DataPoint>, quantity_to_decrease: i32) {
        if !self.has_margin_execute_order() {
            self.removed_for_lack_of_margin = true;
            return;
        }
        assert!(self.current_position.is_some(), "sanity check, should not try to decrease a position that doesnt exit");
        let current_fee = self.calculate_fees(quantity_to_decrease as u64);
        let position = self.current_position.as_mut().unwrap();
        assert!(position.open_qty > 0, "cant decrease a position that doesnt have any contract/shares left open");
        assert!(quantity_to_decrease <= position.open_qty, "can´t decrease a position more contracts/shares than it has");

        let executed_qty = position.total_qty - position.open_qty;
        position.avg_exit_price = ((position.avg_exit_price * executed_qty as f64) + (current_exit_price * quantity_to_decrease as f64)) / (executed_qty + quantity_to_decrease ) as f64;

        assert!(position.avg_exit_price > 0.0);
        position.open_qty -= quantity_to_decrease;

        position.quantity_of_contracts += quantity_to_decrease as u32;
        position.total_paid_in_fees += current_fee;
        self.current_balance -= current_fee;
        self.total_contracts_executed_current_month += quantity_to_decrease as u64;

        assert!(position.total_qty >= position.open_qty, "open quantity quantity cannot be greater than total quantity");
        if position.open_qty == 0 {
            self.close_position(datapoint);
        }
    }



    fn track_current_month(&mut self) {
        let current_month_year = self.current_datapoint.as_ref().unwrap().date() / 100;
        if self.current_month_year == -1 {
            self.current_month_year = current_month_year;
        }

        if self.current_month_year != current_month_year {
            self.current_month_year = current_month_year;
            self.total_contracts_executed_current_month = 0;
        }
    }

    fn open_new_position(&mut self, entry_price: f64, side: Side, datapoint: Box<&dyn DataPoint>) {
        assert!(self.current_position.is_none(), "sanity check, should not try open a position without closing the existing one");
        let fee_open_position = self.calculate_fees(self.base_qty_per_order as u64);
        let new_position = Position {
            side,
            avg_entry_price: entry_price,
            avg_exit_price: 0.0,
            date: datapoint.date(),
            entry_time: datapoint.exit_time(),
            exit_time: -1,
            start_datapoint_number: self.total_candlesticks_processed as i32,
            end_datapoint_number: 0,
            total_qty: self.base_qty_per_order as i32,
            open_qty: self.base_qty_per_order as i32,
            gross_pnl: 0.0,
            liquid_pnl: 0.0,
            total_paid_in_fees: fee_open_position,
            quantity_of_contracts: self.base_qty_per_order,
            is_closed: false,
            sum_per_candle_total_log_returns: 0.0,
            sum_per_candle_log_win_returns: 0.0,
            sum_per_candle_log_lose_returns: 0.0,
        };
        self.current_position = Some(new_position);
        self.current_balance -= fee_open_position;
        self.total_contracts_executed_current_month += self.base_qty_per_order as u64;
    }

    fn close_position(&mut self, datapoint: Box<&dyn DataPoint>) {
        assert!(self.current_position.is_some(), "sanity check, should not try to close a position that doesnt exit");
        let mut closed_position = self.current_position.as_ref().unwrap().clone();
        assert_eq!(0, closed_position.open_qty, "cant close a position that has open contracts/shares in the market");

        closed_position.is_closed = true;
        closed_position.exit_time = datapoint.exit_time();
        closed_position.gross_pnl = Self::calculate_gross_realized_pnl(&closed_position, self.asset);
        closed_position.liquid_pnl = closed_position.gross_pnl - closed_position.total_paid_in_fees;
        closed_position.end_datapoint_number = self.total_candlesticks_processed as i32;
        closed_position.open_qty = 0;

        self.current_balance += closed_position.gross_pnl;
        self.positions.push(closed_position);
        self.current_position = None;
        // println!("candle: {}-{} balance_after_position: {}", candlestick.date, candlestick.start_time, self.current_balance);
    }

    fn calculate_gross_realized_pnl(position: &Position, asset: &Asset) -> f64 {
        let price_diff = match position.side {
            Side::BUY => position.avg_exit_price - position.avg_entry_price,
            Side::SELL => position.avg_entry_price - position.avg_exit_price
        };
        price_diff * (position.total_qty - position.open_qty) as f64 * asset.value_per_variation
    }

    fn calculate_gross_open_pnl(position: &Position, asset: &Asset, current_price: f64) -> f64 {
        let price_diff = match position.side {
            Side::BUY => current_price - position.avg_entry_price,
            Side::SELL => position.avg_entry_price - current_price
        };
        (price_diff / asset.min_variation) * position.open_qty as f64 * asset.value_per_variation
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

        self.calculate_backtest_results(&event_handler)
    }

    pub fn run_backtest(&self, mut strategy: Box<dyn CandlestickTradingStrategy>, base_qty_to_trade: u32, fixed_traded_qty: bool) -> BacktestMetrics {
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

        self.calculate_backtest_results(&event_handler)
    }

    /// Initializes the TradingBacktester
    /// The historical data will be loaded from the local disk or from AWS S3 if not available locally
    pub fn new(
        asset: &Asset,
        start_date: u32,
        end_date: u32,
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

        if let Some(candle_generator) =  candlestick_generator {
            files_to_parse
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
            files_to_parse
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

        Ok(TradingBacktester {
            asset,
            qty_history_trading_days: trading_days,
            qty_history_candles: candlesticks.len() as u64,
            qty_history_trades: candlesticks.iter()
                .fold(0, |total, candle| total + candle.trades),
            data_provider: DataProvider::new(candlesticks, trades),
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

    fn calculate_backtest_results(&self, event_handler: &ExchangeEventHandler) -> BacktestMetrics {
        let mut total_trades = 0;
        let mut total_contracts: u64 = 0;
        let mut total_fees_paid = 0.0;
        let mut total_liquid_pnl = 0.0;
        let mut total_gross_pnl = 0.0;
        let mut total_wins = 0;
        let mut total_loses = 0;

        let mut total_long_trades = 0;
        let mut total_short_trades = 0;
        let mut avg_long_trade_pnl = 0.0;
        let mut avg_short_trade_pnl = 0.0;

        let mut max_win_streak = 0;
        let mut max_lose_streak = 0;
        let mut current_win_lose_streak: i32 = 0;

        let mut max_draw_down_percentage: f64 = 0.0;
        let mut local_max_balance: f64 = INITIAL_BALANCE;
        let mut local_min_balance_after_local_max = INITIAL_BALANCE;
        let mut current_balance = INITIAL_BALANCE;

        let mut daily_start_balance: Vec<f64> = vec![INITIAL_BALANCE];

        let mut liquid_profit_per_day = vec![0.0];
        let mut liquid_profit_best_day = 0.0;
        let mut liquid_profit_worst_day = 0.0;
        let mut current_day = if event_handler.positions.len() > 0 {
            event_handler.positions.first().unwrap().date
        } else {
            -1
        };
        let mut total_trading_days = 1;
        let mut profit_factor_log_sum = 0.0;
        let mut sum_winning_trades: f64 = 0.0;
        let mut sum_losing_trades: f64 = 0.0;

        for position in &event_handler.positions {
            profit_factor_log_sum += position.sum_per_candle_total_log_returns;

            if position.date != current_day {
                liquid_profit_per_day.push(0.0);
                total_trading_days += 1;
                daily_start_balance.push(current_balance);
            }

            let index_liquid_profit = liquid_profit_per_day.len() - 1;
            let liquid_rolling_profit_current_day = *liquid_profit_per_day.last().unwrap() + position.liquid_pnl;
            liquid_profit_per_day[index_liquid_profit] = liquid_rolling_profit_current_day;

            total_trades += 1;
            total_liquid_pnl += position.liquid_pnl;
            total_gross_pnl += position.gross_pnl;
            total_contracts += position.quantity_of_contracts as u64;
            total_fees_paid += position.total_paid_in_fees;
            current_day = position.date;


            if position.side == Side::BUY {
                total_long_trades+=1;
                avg_long_trade_pnl+=position.liquid_pnl;
            }else{
                total_short_trades+=1;
                avg_short_trade_pnl+=position.liquid_pnl;
            }

            if position.liquid_pnl > 0.0 {
                total_wins += 1;
                sum_winning_trades+= position.liquid_pnl;
                if current_win_lose_streak < 0 {
                    max_lose_streak = cmp::max(max_lose_streak, -current_win_lose_streak);
                    current_win_lose_streak = 1;
                } else {
                    current_win_lose_streak += 1;
                    max_win_streak = cmp::max(max_win_streak, current_win_lose_streak);
                }
            } else {
                total_loses += 1;
                sum_losing_trades+= position.liquid_pnl;
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


            current_balance += position.liquid_pnl;
            if current_balance > local_max_balance {
                local_max_balance = current_balance;
                local_min_balance_after_local_max = current_balance;
            }

            if current_balance < local_min_balance_after_local_max {
                local_min_balance_after_local_max = current_balance;
                let current_draw_down = (local_max_balance - local_min_balance_after_local_max) / local_max_balance;
                if max_draw_down_percentage < current_draw_down {
                    max_draw_down_percentage = current_draw_down;
                }
            }
        }

        let daily_percentage_returns = get_daily_percentage_returns(liquid_profit_per_day, daily_start_balance);
        let sum_daily_excess_return_percentage: f64 = daily_percentage_returns.iter()
            .map(|daily_percentage_return| *daily_percentage_return - DAILY_RISK_FREE_RATE)
            .sum();
        let avg_daily_excess_return_percentage = sum_daily_excess_return_percentage / daily_percentage_returns.len() as f64;
        let standard_deviation_percentage_returns = math::standard_deviation(daily_percentage_returns);
        let sharp_ratio = avg_daily_excess_return_percentage / standard_deviation_percentage_returns;

        BacktestMetrics {
            final_balance: event_handler.current_balance,
            initial_balance: INITIAL_BALANCE,
            total_return_percentage: ((event_handler.current_balance - INITIAL_BALANCE) / INITIAL_BALANCE) * 100.0,
            total_gross_profit: total_gross_pnl,
            total_liquid_profit: total_liquid_pnl,
            avg_trade_liquid_pnl: total_liquid_pnl / total_trades as f64,
            avg_daily_liquid_profit: total_liquid_pnl / total_trading_days as f64,
            worse_day_liquid_profit: liquid_profit_worst_day,
            avg_long_trade_liquid_pnl: avg_long_trade_pnl / (total_long_trades as f64 + 1e-30),
            avg_long_short_liquid_pnl: avg_short_trade_pnl / (total_short_trades as f64 + 1e-30),
            best_day_liquid_profit: liquid_profit_best_day,
            median_day_liquid_profit: 0.0,
            median_minutes_in_the_market_per_day: 0,
            total_fees_paid,
            minutes_in_the_market: 0,
            win_rate_percentage: 100.0 * (total_wins as f64 / total_trades as f64),
            total_trades,
            total_long_trades,
            total_short_trades,
            quantity_of_contracts: total_contracts,
            total_wins,
            total_loses,
            avg_risk_reward: 0.0,
            max_win_streak,
            max_lose_streak,
            total_trading_days,
            max_draw_down_percentage: max_draw_down_percentage * 100.0,
            sharp_ratio,
            avg_winning_trade: sum_winning_trades / total_wins as f64,
            avg_losing_trade: sum_losing_trades / total_loses as f64,
            profit_factor: sum_winning_trades/(-1.0*sum_losing_trades),
            profit_factor_log_sum
        }
    }
}


fn get_daily_percentage_returns(liquid_profit_per_day: Vec<f64>, daily_start_balances: Vec<f64>) -> Vec<f64> {
    liquid_profit_per_day.iter().zip(daily_start_balances.iter())
        .map(|(returns_current_day, start_balance_current_day)| 100.0 * (*returns_current_day / *start_balance_current_day))
        .collect()
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_calculate_daily_return_percentages_correctly() {
        let start_of_day_balances = vec![10_000.0, 10_132.0, 9279.0];
        let result_each_day = vec![132.0, -853.0, 2714.0];

        let expected_percentage_returns: Vec<f64> = vec![1.32, -8.418870904066326, 29.248841469985994];
        let percentage_returns = get_daily_percentage_returns(result_each_day, start_of_day_balances);
        assert_eq!(expected_percentage_returns, percentage_returns);
    }

    // todo: commented function as reference for future test implementations
    // fn should_calculate_avg_entry_price_correctly_after_increase_position(){
    //
    // }
    //
    // fn should_calculate_avg_exit_price_correctly_after_decrease_position(){
    //
    // }
}
