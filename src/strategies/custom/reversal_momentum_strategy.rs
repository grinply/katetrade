use std::collections::HashMap;
use chrono::NaiveTime;
use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::models::action::{TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TickCandlestickGenerator};
use crate::strategies::strategy::{CandlestickTradingStrategy};


#[derive(Debug, PartialEq)]
enum Signal {
    SHORT,
    LONG
}


#[derive(Debug)]
#[allow(unused)]
pub struct ReversalMomentumCandlestickStrategy {
    signal: Option<Signal>,
    order_active: bool,
    stop_loss_percentage: f64,
    take_profit_percentage: f64,
    entry_price_range: (f64, f64),
    entry_price_range_percentage: (f64, f64),
    last_buy_price: f64
}

impl ReversalMomentumCandlestickStrategy {
    fn reset_order_state(&mut self) {
        self.order_active = false;
        self.last_buy_price = 0.0;
    }

    fn check_and_place_new_order(&mut self, closed_candle: &Candlestick) -> TradeAction {
        assert_eq!(self.last_buy_price, 0.0);
        assert_eq!(self.order_active, false);

        match &self.signal {
            None => TradeAction::do_nothing(),
            Some(Signal::LONG) => {
                self.check_entry_condition(closed_candle, self.entry_price_range.0, self.entry_price_range.1, TradeAction::send_order(MarketLong))
            }
            Some(Signal::SHORT) => {
                self.check_entry_condition(closed_candle, self.entry_price_range.1, self.entry_price_range.0, TradeAction::send_order(MarketShort))
            }
        }
    }

    fn check_entry_condition(&mut self, closed_candle: &Candlestick, lower_bound: f64, upper_bound: f64, action: TradeAction) -> TradeAction {
        assert_ne!(self.entry_price_range, (0.0, 0.0));

        if closed_candle.close >= lower_bound && closed_candle.close <= upper_bound {
            self.order_active = true;
            self.last_buy_price = closed_candle.close;
            return action;
        }

        TradeAction::do_nothing()
    }

    fn handle_active_order(&mut self, actual_time: &NaiveTime, closed_candle: &Candlestick) -> TradeAction {
        assert_ne!(self.last_buy_price, 0.0);

        if actual_time.gt(&NaiveTime::from_hms_opt(17, 0, 0).unwrap()) {
            if actual_time.le(&NaiveTime::from_hms_opt(17, 30, 0).unwrap()) {
                return self.check_profit_loss(closed_candle);
            } else {
                return self.close_order_on_end_of_day();
            }
        } else {
            let current_profit_loss = get_current_profit(&self.signal.as_ref().unwrap(), &self.last_buy_price, &closed_candle.close);
            let take_stop_signal = take_profit_or_stop_loss(current_profit_loss, self.take_profit_percentage, self.stop_loss_percentage);

            if take_stop_signal.0 || take_stop_signal.1 {
                return self.close_order_on_profit_loss(take_stop_signal);
            }
        }

        TradeAction::do_nothing()
    }

    fn check_profit_loss(&mut self, closed_candle: &Candlestick) -> TradeAction {
        match self.signal.as_ref().unwrap() {
            Signal::LONG if self.last_buy_price <= closed_candle.close => {
                self.reset_order_state();
                TradeAction::send_order(MarketShort)
            }
            Signal::SHORT if self.last_buy_price >= closed_candle.close => {
                self.reset_order_state();
                TradeAction::send_order(MarketLong)
            }
            _ => TradeAction::do_nothing(),
        }
    }

    fn close_order_on_profit_loss(&mut self, take_stop_signal: (bool, bool)) -> TradeAction {
        take_stop_signal.0.then(|| assert_eq!(take_stop_signal.1, false));
        take_stop_signal.1.then(|| assert_eq!(take_stop_signal.0, false));

        self.reset_order_state();
        match self.signal.as_ref().unwrap() {
            Signal::LONG => TradeAction::send_order(MarketShort),
            Signal::SHORT => TradeAction::send_order(MarketLong),
        }
    }

    fn close_order_on_end_of_day(&mut self) -> TradeAction {
        self.reset_order_state();
        match self.signal.as_ref().unwrap() {
            Signal::LONG => TradeAction::send_order(MarketShort),
            Signal::SHORT => TradeAction::send_order(MarketLong),
        }
    }
}

impl CandlestickTradingStrategy for ReversalMomentumCandlestickStrategy {
    fn new() -> Self {
        ReversalMomentumCandlestickStrategy{
            signal: None,
            order_active: false,
            take_profit_percentage: 0.04,
            stop_loss_percentage: 0.04,
            entry_price_range_percentage: (1.0, 1.01),
            entry_price_range: (0.0, 0.0),
            last_buy_price: 0.0
        }
    }

    fn name(&self) -> &'static str {
        "reversal_momentum_candlestick_strategy"
    }
    fn days_of_context(&self) -> u32 {
        2
    }

    fn init_trading_day(&mut self, _: &Asset, context_candles: &[Candlestick]) {
        let candles = aggregate_daily_candles(context_candles);
        assert_eq!(candles.len(), 2);

        let last_but_one_candlestick = candles.first().unwrap();
        let last_candlestick = candles.last().unwrap();

        self.signal = signal_calculation(last_but_one_candlestick, last_candlestick);

        if !self.signal.is_none() {
            self.entry_price_range = get_entry_price(last_candlestick, self.signal.as_ref().unwrap(),self.entry_price_range_percentage, true);
            // println!("DEBUG: Day {} with Signal LONG {}", last_candlestick.date, self.signal == Some(Signal::LONG));
        }
    }


    fn process_closed_candlestick(&mut self, closed_candle: &Candlestick) -> TradeAction {
        let actual_time = parse_time(closed_candle.close_time);

        if self.order_active {
            return self.handle_active_order(&actual_time, &closed_candle);
        } else {
            if actual_time.gt(&NaiveTime::from_hms_opt(17, 0, 0).unwrap()) {
                return TradeAction::do_nothing();
            }
            self.check_and_place_new_order(closed_candle)
        }
    }

    fn candlestick_generator(&self) -> Box<dyn CandlestickGenerator> {
        Box::new(TickCandlestickGenerator::new(1000))
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct ClassicReversalMomentumCandlestickStrategy {
    signal: Option<Signal>,
    order_active: bool,
    stop_loss_percentage: f64,
    take_profit_percentage: f64,
    entry_price_range: (f64, f64),
    entry_price_range_percentage: (f64, f64),
    last_buy_price: f64
}

impl ClassicReversalMomentumCandlestickStrategy {
    fn reset_order_state(&mut self) {
        self.order_active = false;
        self.last_buy_price = 0.0;
    }

    fn check_and_place_new_order(&mut self, closed_candle: &Candlestick) -> TradeAction {
        assert_eq!(self.last_buy_price, 0.0);
        assert_eq!(self.order_active, false);

        match &self.signal {
            None => TradeAction::do_nothing(),
            Some(Signal::LONG) => {
                self.check_entry_condition(closed_candle, self.entry_price_range.0, self.entry_price_range.1, TradeAction::send_order(MarketLong))
            }
            Some(Signal::SHORT) => {
                self.check_entry_condition(closed_candle, self.entry_price_range.1, self.entry_price_range.0, TradeAction::send_order(MarketShort))
            }
        }
    }

    fn check_entry_condition(&mut self, closed_candle: &Candlestick, lower_bound: f64, upper_bound: f64, action: TradeAction) -> TradeAction {
        assert_ne!(self.entry_price_range, (0.0, 0.0));

        if closed_candle.close >= lower_bound && closed_candle.close <= upper_bound {
            self.order_active = true;
            self.last_buy_price = closed_candle.close;
            return action;
        }

        TradeAction::do_nothing()
    }

    fn handle_active_order(&mut self, actual_time: &NaiveTime, closed_candle: &Candlestick) -> TradeAction {
        assert_ne!(self.last_buy_price, 0.0);

        if actual_time.gt(&NaiveTime::from_hms_opt(17, 0, 0).unwrap()) {
            if actual_time.le(&NaiveTime::from_hms_opt(17, 50, 0).unwrap()) {
                return self.check_profit_loss(closed_candle);
            } else {
                return self.close_order_on_end_of_day();
            }
        } else {
            let current_profit_loss = get_current_profit(&self.signal.as_ref().unwrap(), &self.last_buy_price, &closed_candle.close);
            let take_stop_signal = take_profit_or_stop_loss(current_profit_loss, self.take_profit_percentage, self.stop_loss_percentage);

            if take_stop_signal.0 || take_stop_signal.1 {
                return self.close_order_on_profit_loss(take_stop_signal);
            }
        }

        TradeAction::do_nothing()
    }

    fn check_profit_loss(&mut self, closed_candle: &Candlestick) -> TradeAction {
        match self.signal.as_ref().unwrap() {
            Signal::LONG if self.last_buy_price <= closed_candle.close => {
                self.reset_order_state();
                TradeAction::send_order(MarketShort)
            }
            Signal::SHORT if self.last_buy_price >= closed_candle.close => {
                self.reset_order_state();
                TradeAction::send_order(MarketLong)
            }
            _ => TradeAction::do_nothing(),
        }
    }

    fn close_order_on_profit_loss(&mut self, take_stop_signal: (bool, bool)) -> TradeAction {
        take_stop_signal.0.then(|| assert_eq!(take_stop_signal.1, false));
        take_stop_signal.1.then(|| assert_eq!(take_stop_signal.0, false));

        self.reset_order_state();
        match self.signal.as_ref().unwrap() {
            Signal::LONG => TradeAction::send_order(MarketShort),
            Signal::SHORT => TradeAction::send_order(MarketLong),
        }
    }

    fn close_order_on_end_of_day(&mut self) -> TradeAction {
        self.reset_order_state();
        match self.signal.as_ref().unwrap() {
            Signal::LONG => TradeAction::send_order(MarketShort),
            Signal::SHORT => TradeAction::send_order(MarketLong),
        }
    }
}
impl CandlestickTradingStrategy for ClassicReversalMomentumCandlestickStrategy {
    fn new() -> Self {
        ClassicReversalMomentumCandlestickStrategy{
            signal: None,
            order_active: false,
            stop_loss_percentage: 0.04,
            take_profit_percentage: 0.04,
            entry_price_range: (0.0, 0.0),
            entry_price_range_percentage: (1.0, 1.01),
            last_buy_price: 0.0
        }
    }

    fn name(&self) -> &'static str {
        "classic_reversal_momentum_candlestick_strategy"
    }

    fn days_of_context(&self) -> u32 {
        2
    }

    fn init_trading_day(&mut self, _: &Asset, context_candle: &[Candlestick]) {
        let initial_candlesticks = aggregate_daily_candles(context_candle);
        let last_candlestick = initial_candlesticks.last().unwrap();

        self.signal = classic_signal_calculation(last_candlestick);

        if !self.signal.is_none() {
            self.entry_price_range = get_entry_price(last_candlestick, self.signal.as_ref().unwrap(),self.entry_price_range_percentage, false);
        }
    }

    fn candlestick_generator(&self) -> Box<dyn CandlestickGenerator> {
        Box::new(TickCandlestickGenerator::new(1000))
    }

    fn process_closed_candlestick(&mut self, closed_candle: &Candlestick) -> TradeAction {
        let actual_time = parse_time(closed_candle.close_time);

        if self.order_active {
            self.handle_active_order(&actual_time, &closed_candle)
        } else {
            if actual_time.gt(&NaiveTime::from_hms_opt(17, 0, 0).unwrap()) {
                return TradeAction::do_nothing();
            }
            self.check_and_place_new_order(closed_candle)
        }
    }
}

fn parse_time(time: i32) -> NaiveTime {
    let time_str = time.to_string();
    let len = time_str.len();

    let (hour, minute, second, millisecond) = match len {
        5 => { // Format: HMMSS
            (
                time_str[0..1].parse().unwrap_or(0),
                time_str[1..3].parse().unwrap_or(0),
                time_str[3..].parse().unwrap_or(0),
                0,
            )
        }
        6 => { // Format: HHMMSS
            (
                time_str[0..2].parse().unwrap_or(0),
                time_str[2..4].parse().unwrap_or(0),
                time_str[4..].parse().unwrap_or(0),
                0,
            )
        }
        8 => { // Format: HMMSSmmm
            (
                time_str[0..1].parse().unwrap_or(0),
                time_str[1..3].parse().unwrap_or(0),
                time_str[3..5].parse().unwrap_or(0),
                time_str[5..].parse().unwrap_or(0),
            )
        }
        9 => { // Format: HHMMSSmmm
            (
                time_str[0..2].parse().unwrap_or(0),
                time_str[2..4].parse().unwrap_or(0),
                time_str[4..6].parse().unwrap_or(0),
                time_str[6..].parse().unwrap_or(0),
            )
        }
        _ => panic!("Invalid time format: {}", time),
    };

    NaiveTime::from_hms_milli_opt(hour, minute, second, millisecond)
        .unwrap_or_else(|| panic!("Could not parse time: {}", time))
}

fn take_profit_or_stop_loss(profit: f64, take_profit_percentage: f64, stop_loss_percentage: f64) -> (bool, bool) {
    if profit >= take_profit_percentage {
        return (true, false);
    }

    if profit <= -stop_loss_percentage {
        return (false, true);
    }

    (false, false)
}
fn get_current_profit(signal: &Signal, entry_price: &f64, current_price: &f64) -> f64 {
    match signal {
        Signal::LONG => (current_price - entry_price) / entry_price,
        Signal::SHORT => -((current_price - entry_price) / entry_price)
    }
}


fn get_entry_price(candlestick: &Candlestick, signal: &Signal, entry_price_range_percentage: (f64, f64), is_updated_version: bool) -> (f64,f64) {
    match signal {
        Signal::LONG => {
            if is_updated_version {
                (candlestick.low*entry_price_range_percentage.0,candlestick.low*entry_price_range_percentage.1)
            } else {
                (candlestick.high*entry_price_range_percentage.0,candlestick.high*entry_price_range_percentage.1)
            }
        }
        Signal::SHORT => {
            if is_updated_version {
                (candlestick.high - (entry_price_range_percentage.0 - 1.0)*candlestick.high,
                candlestick.high - (entry_price_range_percentage.1 - 1.0)*candlestick.high)
            } else {
                (candlestick.low - (entry_price_range_percentage.0 - 1.0)*candlestick.low,
                 candlestick.low - (entry_price_range_percentage.1 - 1.0)*candlestick.low)
            }
        }
    }
}
fn aggregate_daily_candles(context_candles: &[Candlestick]) -> Vec<Candlestick> {
    let mut daily_map: HashMap<i32, Candlestick> = HashMap::new();

    for candle in context_candles.iter() {
        let date = candle.date;

        if let Some(existing_candle) = daily_map.get_mut(&date) {
            existing_candle.high = existing_candle.high.max(candle.high);
            existing_candle.low = existing_candle.low.min(candle.low);
            existing_candle.close = candle.close;
            existing_candle.close_time = candle.close_time;
            existing_candle.volume += candle.volume;
            existing_candle.trades += candle.trades;
        } else {
            daily_map.insert(
                date,
                Candlestick {
                    date,
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: candle.volume,
                    start_time: candle.start_time,
                    close_time: candle.close_time,
                    is_closed: true,
                    trades: candle.trades,
                },
            );
        }
    }

    let mut aggregated_candles: Vec<Candlestick> = daily_map.into_values().collect();

    aggregated_candles.sort_by(|a, b| a.date.cmp(&b.date));

    aggregated_candles
}

fn classic_signal_calculation(last_candlestick: &Candlestick) -> Option<Signal> {
    if last_candlestick.open > last_candlestick.close {
        return Some(Signal::LONG);
    } else if last_candlestick.open < last_candlestick.close {
        return Some(Signal::SHORT);
    }
    None

}
fn signal_calculation(last_but_one_candlestick: &Candlestick, last_candlestick: &Candlestick) -> Option<Signal> {

    // Long Conditions
    // c0 = is red candle?
    // Below conditions are from updated version
    // c1 = High is higher than the last candle high?
    // c2 = Low is lower than the last candle low?
    // c3 = Close is lower than the last candle low?

    let long_condition0 = last_candlestick.open > last_candlestick.close;
    let long_condition1 = last_candlestick.high > last_but_one_candlestick.high;
    let long_condition2 = last_candlestick.low < last_but_one_candlestick.low;
    let long_condition3 = last_candlestick.close < last_but_one_candlestick.low;

    if long_condition0 && long_condition1 && long_condition2 && long_condition3 {
        return Some(Signal::LONG)
    }

    // Short Conditions
    // c0 = is green candle?
    // Below conditions are from updated version
    // c1 = Low is lower than the last candle low?
    // c2 = High is higher than the last candle high?
    // c3 = Close is higher than the last candle high?

    let short_condition0 = last_candlestick.open < last_candlestick.close;
    let short_condition1 = last_candlestick.low < last_but_one_candlestick.low;
    let short_condition2 = last_candlestick.high > last_but_one_candlestick.high;
    let short_condition3 = last_candlestick.close > last_but_one_candlestick.high;

    if short_condition0 && short_condition1 && short_condition2 && short_condition3 {
        return Some(Signal::SHORT)
    }

    None
}








// Scenario 1.0:
    // Morning
    // Last Day Short Signal
    // No Order Active
    // Current Candle reach Entry Price

// Scenario 1.1:
    // Morning
    // Last Day Short Signal
    // No Order Active
    // Current Candle do not reached Entry Price

//Scenario 2.0:
    // Morning
    // Last Day Short Signal
    // Order Active
    // Current Candle do not Reached Take Profit

//Scenario 2.1:
    // Morning
    // Last Day Short Signal
    // Order Active
    // Current Candle reached Take Profit

//Scenario 2.2:
    // Morning
    // Last Day Short Signal
    // Order Active
    // Current Candle reached Stop Loss

//Scenario 3.0:
    // Morning
    // Last Day Long Signal
    // Order Not Active
    // Current Candle reached entry price

//Scenario 3.1:
    // Morning
    // Last Day Long Signal
    // Order Not Active
    // Current Candle do not reached entry price

//Scenario 4.0:
    // Morning
    // Last Day Long Signal
    // Order Active
    // Current Candle do not Reached Take Profit

//Scenario 4.1:
    // Morning
    // Last Day Long Signal
    // Order Active
    // Current Candle do Reached Take Profit

//Scenario 4.2:
    // Morning
    // Last Day Long Signal
    // Order Active
    // Current Candle do Reached Stop Loss

//Scenario 5.0
    // Almost End of the day (17_00_00 -> 17_50_00)
    // Last Day Short/Long Signal
    // Order Not Active
    // Current Candle Reached/DoNotReached Entry Price

//Scenario 5.1
    // Almost End of the day (17_00_00 -> 17_50_00)
    // Last Day Short Signal
    // Order Active
    // Current Candle have Profit

//Scenario 5.2
    // Almost End of the day (17_00_00 -> 17_50_00)
    // Last Day Short Signal
    // Order Active
    // Current Candle do not have profit
#[cfg(test)]
mod classic_reversal_momentum_strategy_tests {
    use crate::models::exchange::Exchange;
    use crate::models::market::Market;
    use super::*;

    #[test]
    fn test_classic_reversal_momentum_candlestick_strategy_init_trading_days() {
        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let initial_candlesticks = vec![
            Candlestick { date: 20231110, open: 130821.754224, high: 132715.797145, low: 130758.852429, close: 132541.069938, volume: 1500.0, start_time: 93000, close_time: 93100, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 132792.677116, high: 132932.458881, low: 131059.383225, close: 132205.593701, volume: 1500.0, start_time: 93100, close_time: 93200, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 132513.113585, high: 132513.113585, low: 131464.750344, close: 132506.124497, volume: 1500.0, start_time: 93200, close_time: 93300, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 131967.964700, high: 134323.287447, low: 131765.281140, close: 133868.996709, volume: 1500.0, start_time: 93300, close_time: 93400, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 133715.236767, high: 135301.759804, low: 133610.400443, close: 134581.883712, volume: 1500.0, start_time: 93400, close_time: 93500, is_closed: true, trades: 300 },
        ];

        let aggregated_candle = aggregate_daily_candles(&initial_candlesticks);

        assert_eq!(aggregated_candle.len(), 1);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*initial_candlesticks);

        let expected_low = 130758.852429;
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(aggregated_candle.get(0).unwrap().low, expected_low);

        assert_eq!(classic_strategy.entry_price_range, (130758.852429, 129451.26390471001));
    }
    #[test]
    // Scenario 1.0
    fn test_classic_reversal_momentum_candlestick_strategy_morning_short_entry_no_active_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 100.0,
                high: 104.0,
                low: 98.0,
                close: 102.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.5,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.entry_price_range, (98.0, 97.02));

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.order_active, false);
        assert!(current_candlestick.close < classic_strategy.entry_price_range.0);

        // Run Scenario 1.0
        let scenario1_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario1_short_trade_action.enter_market_action, Some(MarketShort));
        assert_eq!(classic_strategy.order_active, true);
    }
    #[test]
    // Scenario 1.1
    fn test_classic_reversal_momentum_candlestick_strategy_morning_short_none_no_active_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 100.0,
                high: 104.0,
                low: 98.0,
                close: 102.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 98.1,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.entry_price_range, (98.0, 97.02));

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.order_active, false);
        assert!(current_candlestick.close > classic_strategy.entry_price_range.0);

        // Run Scenario 1.1
        let scenario1_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario1_short_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, false);
    }
    #[test]
    // Scenario 2.0
    fn test_classic_reversal_momentum_candlestick_strategy_morning_short_keep_with_active_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 100.0,
                high: 104.0,
                low: 98.0,
                close: 102.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.entry_price_range, (98.0, 97.02));


        classic_strategy.order_active = true;
        current_candlestick.close = last_day_candlestick[0].low - last_day_candlestick[0].low*(classic_strategy.take_profit_percentage-0.01);
        classic_strategy.last_buy_price = current_candlestick.close;
        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.order_active, true);
        assert!(current_candlestick.close < classic_strategy.entry_price_range.0);

        let scenario2_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario2_short_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, true);
    }
    #[test]
    // Scenario 2.1
    fn test_classic_reversal_momentum_candlestick_strategy_morning_short_exit_take_profit_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 100.0,
                high: 104.0,
                low: 98.0,
                close: 102.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.entry_price_range, (98.0, 97.02));


        classic_strategy.order_active = true;
        current_candlestick.close = last_day_candlestick[0].low - last_day_candlestick[0].low*(classic_strategy.take_profit_percentage+0.01);
        classic_strategy.last_buy_price = classic_strategy.entry_price_range.0;

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.order_active, true);

        let current_profit = get_current_profit(&Signal::SHORT, &classic_strategy.last_buy_price, &current_candlestick.close);
        assert_eq!(take_profit_or_stop_loss(current_profit, classic_strategy.take_profit_percentage, classic_strategy.stop_loss_percentage), (true, false));

        let scenario2_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario2_short_trade_action.enter_market_action, Some(MarketLong));
        assert_eq!(classic_strategy.order_active, false);
    }
    #[test]
    // Scenario 2.2
    fn test_classic_reversal_momentum_candlestick_strategy_morning_short_exit_stop_loss_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 100.0,
                high: 104.0,
                low: 98.0,
                close: 102.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.entry_price_range, (98.0, 97.02));


        classic_strategy.order_active = true;
        current_candlestick.close = last_day_candlestick[0].low + last_day_candlestick[0].low*(classic_strategy.stop_loss_percentage+0.01);
        classic_strategy.last_buy_price = last_day_candlestick[0].low;

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.order_active, true);

        let current_profit = get_current_profit(&Signal::SHORT, &classic_strategy.last_buy_price, &current_candlestick.close);
        assert_eq!(take_profit_or_stop_loss(current_profit, classic_strategy.take_profit_percentage, classic_strategy.stop_loss_percentage), (false, true));

        let scenario2_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario2_short_trade_action.enter_market_action, Some(MarketLong));
        assert_eq!(classic_strategy.order_active, false);
    }
    #[test]
    //Scenario 3.0
    fn test_classic_reversal_momentum_candlestick_strategy_morning_long_entry_no_active_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 102.0,
                high: 102.0,
                low: 98.0,
                close: 100.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let current_candlestick = Candlestick{
            date: 20231111,
            open: 100.0,
            high: 105.0,
            low: 96.0,
            close: 103.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.entry_price_range, (102.0, 103.02));

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.order_active, false);

        assert!(current_candlestick.close >= classic_strategy.entry_price_range.0 && current_candlestick.close <= classic_strategy.entry_price_range.1);

        // Run Scenario 3.0
        let scenario3_long_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario3_long_trade_action.enter_market_action, Some(MarketLong));
        assert_eq!(classic_strategy.order_active, true);
    }
    #[test]
    //Scenario 3.1
    fn test_classic_reversal_momentum_candlestick_strategy_morning_long_none_no_active_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 102.0,
                high: 102.0,
                low: 98.0,
                close: 100.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let current_candlestick = Candlestick{
            date: 20231111,
            open: 100.0,
            high: 105.0,
            low: 96.0,
            close: 101.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.entry_price_range, (102.0, 103.02));

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.order_active, false);
        assert!(current_candlestick.close <= classic_strategy.entry_price_range.0 || current_candlestick.close >= classic_strategy.entry_price_range.1);

        // Run Scenario 3.1
        let scenario3_long_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario3_long_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, false);
    }
    #[test]
    //Scenario 4.0
    fn test_classic_reversal_momentum_candlestick_strategy_morning_long_keep_with_active_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 102.0,
                high: 102.0,
                low: 98.0,
                close: 100.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.entry_price_range, (102.0, 103.02));


        classic_strategy.order_active = true;
        current_candlestick.close = last_day_candlestick[0].high + last_day_candlestick[0].high*(classic_strategy.take_profit_percentage-0.01); // 1% profit
        classic_strategy.last_buy_price = classic_strategy.entry_price_range.0;

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.order_active, true);

        let current_profit = get_current_profit(&Signal::LONG, &classic_strategy.last_buy_price, &current_candlestick.close);
        assert_eq!(take_profit_or_stop_loss(current_profit, classic_strategy.take_profit_percentage, classic_strategy.stop_loss_percentage), (false, false));

        let scenario4_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario4_short_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, true);
    }
    #[test]
    //Scenario 4.1
    fn test_classic_reversal_momentum_candlestick_strategy_morning_long_exit_take_profit_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 102.0,
                high: 102.0,
                low: 98.0,
                close: 100.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.entry_price_range, (102.0, 103.02));


        classic_strategy.order_active = true;
        classic_strategy.last_buy_price = last_day_candlestick[0].high;
        current_candlestick.close = last_day_candlestick[0].high + last_day_candlestick[0].high*0.05; // 4% profit

        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.order_active, true);

        let current_profit = get_current_profit(&Signal::LONG, &classic_strategy.last_buy_price, &current_candlestick.close);
        assert_eq!(take_profit_or_stop_loss(current_profit, classic_strategy.take_profit_percentage, classic_strategy.stop_loss_percentage), (true, false));

        let scenario4_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario4_short_trade_action.enter_market_action, Some(MarketShort));
        assert_eq!(classic_strategy.order_active, false);
    }
    #[test]
    //Scenario 4.2
    fn test_classic_reversal_momentum_candlestick_strategy_morning_long_exit_stop_loss_order() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 102.0,
                high: 102.0,
                low: 98.0,
                close: 100.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.0,
            volume: 3000.0,
            start_time: 9_33_00,
            close_time: 9_34_00,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3).unwrap();
        classic_strategy.init_trading_day(&asset, &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.entry_price_range, (102.0, 103.02));


        classic_strategy.order_active = true;
        current_candlestick.close = last_day_candlestick[0].high - last_day_candlestick[0].high*0.041; // 4% loss
        classic_strategy.last_buy_price = last_day_candlestick[0].high;
        assert!(current_candlestick.close_time < 17_00_00);
        assert_eq!(classic_strategy.signal, Some(Signal::LONG));
        assert_eq!(classic_strategy.order_active, true);
        let current_profit = get_current_profit(&Signal::LONG, &classic_strategy.last_buy_price, &current_candlestick.close);
        assert_eq!(take_profit_or_stop_loss(current_profit, classic_strategy.take_profit_percentage, classic_strategy.stop_loss_percentage), (false, true));

        let scenario4_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario4_short_trade_action.enter_market_action, Some(MarketShort));
        assert_eq!(classic_strategy.order_active, false);
    }

    #[test]
    //Scenario 5.0
    fn test_classic_reversal_momentum_candlestick_strategy_almost_ending_day_no_order_active() {
        let last_day_candlestick = vec![
            Candlestick{
                date: 20231110,
                open: 100.0,
                high: 104.0,
                low: 98.0,
                close: 102.0,
                volume: 1500.0,
                start_time: 9_00_00,
                close_time: 18_00_00,
                is_closed: true,
                trades: 300
            }];
        let mut current_candlestick = Candlestick{
            date: 20231111,
            open: 102.0,
            high: 103.0,
            low: 96.0,
            close: 97.5,
            volume: 3000.0,
            start_time: 17_30_00,
            close_time: 17_30_01,
            is_closed: true,
            trades: 3000,
        };

        let mut classic_strategy = ClassicReversalMomentumCandlestickStrategy::new();
        classic_strategy.take_profit_percentage= 0.04;
        classic_strategy.stop_loss_percentage= 0.04;
        classic_strategy.entry_price_range_percentage= (1.0, 1.01);

        let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3);

        classic_strategy.init_trading_day(&asset.as_ref().unwrap(), &*last_day_candlestick);

        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.entry_price_range, (98.0, 97.02));

        assert!(current_candlestick.close_time > 17_00_00 && current_candlestick.close_time < 17_50_00);
        assert_eq!(classic_strategy.signal, Some(Signal::SHORT));
        assert_eq!(classic_strategy.order_active, false);
        assert!(current_candlestick.close < classic_strategy.entry_price_range.0);
        assert!(current_candlestick.close > classic_strategy.entry_price_range.1);

        // Short Reached Price
        let scenario5_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario5_short_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, false);

        // Short  Do not Reached Price
        current_candlestick.close = classic_strategy.entry_price_range.0 + 1.0;
        assert!(current_candlestick.close > classic_strategy.entry_price_range.0);

        let scenario5_short_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario5_short_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, false);

        // Long Reached Price
        classic_strategy.signal = Some(Signal::LONG);
        classic_strategy.entry_price_range.0 = last_day_candlestick[0].high;

        current_candlestick.close = last_day_candlestick[0].high + 1.0;
        assert!(current_candlestick.close > classic_strategy.entry_price_range.0);

        let scenario5_long_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario5_long_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, false);

        // Long do not Reached Price
        classic_strategy.signal = Some(Signal::LONG);
        classic_strategy.entry_price_range.0 = last_day_candlestick[0].high;

        current_candlestick.close = last_day_candlestick[0].high - 1.0;
        assert!(current_candlestick.close < classic_strategy.entry_price_range.0);

        let scenario5_long_trade_action = classic_strategy.process_closed_candlestick(&current_candlestick);
        assert_eq!(scenario5_long_trade_action.enter_market_action, None);
        assert_eq!(classic_strategy.order_active, false);
    }


    // Almost End of the day (17_00_00 -> 17_50_00)
    // Last Day Short Signal
    // Order Active
    // Current Candle have Profit
    #[test]
    //Scenario 5.1
    fn test_classic_reversal_momentum_candlestick_strategy_almost_ending_day_short_with_order_active_take_profit() {

    }

    #[test]
    //Scenario 5.2
    fn test_classic_reversal_momentum_candlestick_strategy_almost_ending_day_short_with_order_active_no_profit() {

    }
    // Almost End of the day (17_00_00 -> 17_50_00)
    // Last Day Short Signal
    // Order Active
    // Current Candle do not have profit

}

#[cfg(test)]
mod reversal_momentum_common_tests {
    use super::*;


    #[test]
    fn test_parse_time() {
        let times = [91000, 131030, 91033223, 111033223];

        assert_eq!(parse_time(times[0]), NaiveTime::from_hms_milli_opt(9,10,0,0).unwrap());
        assert_eq!(parse_time(times[1]), NaiveTime::from_hms_milli_opt(13,10,30,0).unwrap());
        assert_eq!(parse_time(times[2]), NaiveTime::from_hms_milli_opt(9,10,33,223).unwrap());
        assert_eq!(parse_time(times[3]), NaiveTime::from_hms_milli_opt(11,10,33,223).unwrap());
    }
    #[test]
    fn test_take_profit_or_stop_loss() {
        let take_profit_percentage = 0.02;
        let stop_loss_percentage = 0.04;

        assert_eq!(take_profit_or_stop_loss(0.04,take_profit_percentage, stop_loss_percentage), (true, false));
        assert_eq!(take_profit_or_stop_loss(0.02,take_profit_percentage, stop_loss_percentage), (true, false));

        assert_eq!(take_profit_or_stop_loss(0.01,take_profit_percentage, stop_loss_percentage), (false, false));
        assert_eq!(take_profit_or_stop_loss(0.00,take_profit_percentage, stop_loss_percentage), (false, false));
        assert_eq!(take_profit_or_stop_loss(-0.01,take_profit_percentage, stop_loss_percentage), (false, false));
        assert_eq!(take_profit_or_stop_loss(-0.02,take_profit_percentage, stop_loss_percentage), (false, false));

        assert_eq!(take_profit_or_stop_loss(-0.04,take_profit_percentage, stop_loss_percentage), (false, true));
        assert_eq!(take_profit_or_stop_loss(-0.05,take_profit_percentage, stop_loss_percentage), (false, true));

    }

    #[test]
    fn test_get_current_profit() {
        assert_eq!(get_current_profit(&Signal::LONG,&100.0, &104.0), 0.04);
        assert_eq!(get_current_profit(&Signal::LONG,&100.0, &96.0), -0.04);

        assert_eq!(get_current_profit(&Signal::SHORT,&100.0, &104.0), -0.04);
        assert_eq!(get_current_profit(&Signal::SHORT,&100.0, &96.0), 0.04);

    }

    #[test]
    fn test_return_entry_price_long_updated() {
        let last_candle = &Candlestick{
            open: 2.0,
            high: 3.0,
            low: 1.0,
            close: 2.0,
            volume: 3.0,
            date: 20201001,
            start_time: 143500,
            close_time: 143600,
            is_closed: false,
            trades: 60,
        };
        assert_eq!(get_entry_price(last_candle, &Signal::LONG, (0.9, 1.1), true), (0.9,1.1))
    }

    #[test]
    fn test_return_entry_price_long_classic() {
        let last_candle = &Candlestick{
            open: 2.0,
            high: 3.0,
            low: 1.0,
            close: 2.0,
            volume: 3.0,
            date: 20201001,
            start_time: 143500,
            close_time: 143600,
            is_closed: false,
            trades: 60,
        };
        assert_eq!(get_entry_price(last_candle, &Signal::LONG, (0.9, 1.1),false), (2.7,3.3000000000000003))
    }

    #[test]
    fn test_return_entry_price_short_updated() {
        let last_candle = &Candlestick{
            open: 2.0,
            high: 3.0,
            low: 1.0,
            close: 2.0,
            volume: 3.0,
            date: 20201001,
            start_time: 143500,
            close_time: 143600,
            is_closed: false,
            trades: 60,
        };
        assert_eq!(get_entry_price(last_candle, &Signal::SHORT, (0.9, 1.1),true), (3.3, 2.6999999999999997))
    }

    #[test]
    fn test_return_entry_price_short_classic() {
        let last_candle = &Candlestick{
            open: 2.0,
            high: 3.0,
            low: 1.0,
            close: 2.0,
            volume: 3.0,
            date: 20201001,
            start_time: 143500,
            close_time: 143600,
            is_closed: false,
            trades: 60,
        };
        assert_eq!(get_entry_price(last_candle, &Signal::SHORT,(0.9,1.1), false), (1.1, 0.8999999999999999))
    }

    #[test]
    fn test_aggregate_with_less_than_two_days() {
        let candles = vec![
            Candlestick { date: 20231110, open: 130821.754224, high: 132715.797145, low: 130758.852429, close: 132541.069938, volume: 1500.0, start_time: 93000, close_time: 93100, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 132792.677116, high: 132932.458881, low: 131059.383225, close: 132205.593701, volume: 1500.0, start_time: 93100, close_time: 93200, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 132513.113585, high: 132513.113585, low: 131464.750344, close: 132506.124497, volume: 1500.0, start_time: 93200, close_time: 93300, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 131967.964700, high: 134323.287447, low: 131765.281140, close: 133868.996709, volume: 1500.0, start_time: 93300, close_time: 93400, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 133715.236767, high: 135301.759804, low: 133610.400443, close: 134581.883712, volume: 1500.0, start_time: 93400, close_time: 93500, is_closed: true, trades: 300 },
        ];

        let candles = aggregate_daily_candles(&candles);

        assert_eq!(candles.len(), 1)

    }

    #[test]
    fn test_aggregate_with_more_than_two_days() {
        let candles = vec![
            Candlestick { date: 20231109, open: 95.0, high: 105.0, low: 90.0, close: 100.0, volume: 1000.0, start_time: 93000, close_time: 100000, is_closed: true, trades: 200 },
            Candlestick { date: 20231110, open: 100.0, high: 110.0, low: 99.0, close: 105.0, volume: 1500.0, start_time: 93000, close_time: 100000, is_closed: true, trades: 300 },
            Candlestick { date: 20231111, open: 111.0, high: 120.0, low: 110.0, close: 119.0, volume: 1800.0, start_time: 93000, close_time: 100000, is_closed: true, trades: 350 },
        ];

        let candles = aggregate_daily_candles(&candles);

        assert_eq!(candles.len(), 3)
    }
    #[test]
    fn test_aggregate_exactly_two_days() {
        let candles = vec![
            Candlestick { date: 20231110, open: 130821.754224, high: 132715.797145, low: 130758.852429, close: 132541.069938, volume: 1500.0, start_time: 93000, close_time: 93100, is_closed: true, trades: 9548 },
            Candlestick { date: 20231110, open: 132792.677116, high: 132932.458881, low: 131059.383225, close: 132205.593701, volume: 3500.0, start_time: 93100, close_time: 93200, is_closed: true, trades: 9066 },
            Candlestick { date: 20231110, open: 132513.113585, high: 132513.113585, low: 131464.750344, close: 132506.124497, volume: 2000.0, start_time: 93200, close_time: 93300, is_closed: true, trades: 9066 },
            Candlestick { date: 20231110, open: 131967.964700, high: 134323.287447, low: 131765.281140, close: 133868.996709, volume: 1000.0, start_time: 93300, close_time: 93400, is_closed: true, trades: 4826 },
            Candlestick { date: 20231110, open: 133715.236767, high: 135301.759804, low: 133610.400443, close: 134581.883712, volume: 5000.0, start_time: 93400, close_time: 93500, is_closed: true, trades: 3495 },

            Candlestick { date: 20231111, open: 134611.883712, high: 135611.883712, low: 130758.852429, close: 132541.069938, volume: 1500.0, start_time: 93000, close_time: 93100, is_closed: true, trades: 9548 },
            Candlestick { date: 20231111, open: 132792.677116, high: 132932.458881, low: 131059.383225, close: 132205.593701, volume: 3500.0, start_time: 93100, close_time: 93200, is_closed: true, trades: 9066 },
            Candlestick { date: 20231111, open: 132513.113585, high: 132513.113585, low: 131464.750344, close: 132506.124497, volume: 2000.0, start_time: 93200, close_time: 93300, is_closed: true, trades: 9066 },
            Candlestick { date: 20231111, open: 131967.964700, high: 134323.287447, low: 101765.281140, close: 133868.996709, volume: 1000.0, start_time: 93300, close_time: 93400, is_closed: true, trades: 4826 },
            Candlestick { date: 20231111, open: 133715.236767, high: 135301.759804, low: 133610.400443, close: 134581.883712, volume: 3000.0, start_time: 93400, close_time: 93500, is_closed: true, trades: 3494 },

        ];

        let result = aggregate_daily_candles(&candles);

        assert_eq!(result.len(), 2);

        //Asserting first candle
        assert_eq!(result[0].date, 20231110, "Date does not match");
        assert_eq!(result[0].open, 130821.754224, "Open value does not match");
        assert_eq!(result[0].high, 135301.759804, "High value does not match");
        assert_eq!(result[0].low, 130758.852429, "Low value does not match");
        assert_eq!(result[0].close, 134581.883712, "Close value does not match");
        assert_eq!(result[0].volume, 13000.0, "Volume value does not match");
        assert_eq!(result[0].start_time, 93000, "Start date does not match");
        assert_eq!(result[0].close_time, 93500, "Close date does not match");
        assert_eq!(result[0].trades, 36001, "Trades value does not match");

        //Asserting second candle
        assert_eq!(result[1].date, 20231111, "Date does not match");
        assert_eq!(result[1].open, 134611.883712, "Open value does not match");
        assert_eq!(result[1].high, 135611.883712, "High value does not match");
        assert_eq!(result[1].low, 101765.281140, "Low value does not match");
        assert_eq!(result[1].close, 134581.883712, "Close value does not match");
        assert_eq!(result[1].volume, 11000.0, "Volume value does not match");
        assert_eq!(result[1].start_time, 93000, "Start date does not match");
        assert_eq!(result[1].close_time, 93500, "Close date does not match");
        assert_eq!(result[1].trades, 36000, "Trades value does not match");
    }

    #[test]
    fn test_signal_calculation_long() {
        let current_candle = Candlestick {
            date: 20231110,
            open: 153689.529445,
            high: 153926.481182,
            low: 151076.478345,
            close: 151352.922038,
            volume: 1500.0,
            start_time: 90000,
            close_time: 180000,
            is_closed: true,
            trades: 90548
        };
        let last_candle = Candlestick {
            date: 20231111,
            open: 152840.452387,
            high: 153630.291511	,
            low: 151965.047359,
            close: 152958.928256,
            volume: 1500.0,
            start_time: 90000,
            close_time: 180000,
            is_closed: true,
            trades: 90548
        };

        let signal = signal_calculation(&last_candle, &current_candle);

        assert_eq!(signal, Some(Signal::LONG))
    }
    #[test]
    fn test_signal_calculation_short() {
        let current_candle = Candlestick {
            date: 20231111,
            open: 102028.128617,
            high: 104673.594800,
            low: 99606.408330,
            close: 104509.075759,
            volume: 1500.0,
            start_time: 90000,
            close_time: 180000,
            is_closed: true,
            trades: 90548
        };
        let last_candle = Candlestick {
            date: 20231110,
            open: 103976.034065,
            high: 104193.199200,
            low: 101567.475302,
            close: 103317.957900,
            volume: 1500.0,
            start_time: 90000,
            close_time: 180000,
            is_closed: true,
            trades: 90548
        };

        let signal = signal_calculation(&last_candle, &current_candle);

        assert_eq!(signal, Some(Signal::SHORT))
    }
    #[test]
    fn test_signal_calculation_none() {
        let current_candle = Candlestick {
            date: 20231111,
            open: 131580.000000,
            high: 133040.000000,
            low: 131380.000000,
            close: 132005.000000,
            volume: 1500.0,
            start_time: 90000,
            close_time: 180000,
            is_closed: true,
            trades: 90548
        };
        let last_candle = Candlestick {
            date: 20231110,
            high: 132830.000000,
            low: 131260.000000,
            close: 132665.000000,
            open: 131695.000000,
            volume: 1500.0,
            start_time: 90000,
            close_time: 180000,
            is_closed: true,
            trades: 90548
        };

        let signal = signal_calculation(&last_candle, &current_candle);
        assert_eq!(signal, None)

    }
}