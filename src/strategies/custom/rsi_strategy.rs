
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use ta::indicators::RelativeStrengthIndex as RSI;
use ta::indicators::AverageTrueRange as ATR;
use ta::{Next, Reset};
use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::strategies::strategy::{CandlestickTradingStrategy};

#[derive(Debug, PartialEq)]
enum Signal {
    SHORT,
    LONG
}

#[derive(Debug)]
pub struct RsiOverboughtOversoldCandlestickStrategy {
    day_period: bool,
    rsi: RSI,
    atr: ATR,

    is_order_active: Option<Signal>,
    atr_multiplier: f64,
    entry_price: f64
}

impl RsiOverboughtOversoldCandlestickStrategy {
    fn reset(&mut self) {
        self.rsi.reset();
        self.atr.reset();
    }

    // (take_profit, stop_loss)
    fn calculate_stop_and_take(&self, current_atr: f64) -> (f64, f64) {
        if self.is_order_active.is_none() {
            return (0.0, 0.0)
        }

        let positive_operation = self.entry_price + (current_atr * self.atr_multiplier);
        let negative_operation = self.entry_price - (current_atr * self.atr_multiplier);

        if self.is_order_active.eq(&Some(Signal::SHORT)) {
            return (negative_operation, positive_operation);
        }

        if self.is_order_active.eq(&Some(Signal::LONG)) {
            return (positive_operation, negative_operation);
        }

        panic!("is_order_active does not match any valid value")
    }
}

impl CandlestickTradingStrategy for RsiOverboughtOversoldCandlestickStrategy {
    fn new() -> Self {
        RsiOverboughtOversoldCandlestickStrategy{
            day_period: false,
            rsi: RSI::new(14).unwrap(),
            atr: ATR::new(14).unwrap(),
            is_order_active: None,
            atr_multiplier: 2.0,
            entry_price: 0.0,
        }
    }

    fn name(&self) -> &'static str {
        "rsi_overbought_oversold_candlestick_strategy"
    }

    fn init_trading_day(&mut self, _: &Asset, candles: &[Candlestick]) {
        self.reset();
        // Initialize trading day logic
        self.day_period = false;
        self.rsi = RSI::new(14).unwrap();
        self.atr = ATR::new(14).unwrap();

        for candle in candles.iter() {
            self.rsi.next(candle.close);
        }
    }

    fn candlestick_generator(&self) -> Box<dyn CandlestickGenerator> {
        Box::new(TimeCandlestickGenerator::new(TimeUnit::Minutes, 5))
    }

    fn process_closed_candlestick(&mut self, closed_candle: &Candlestick) -> TradeAction {
        let rsi_value = self.rsi.next(closed_candle.close);
        let atr_value = self.atr.next(closed_candle.close);

        if closed_candle.start_time < 10_10_00 {
            return TradeAction::do_nothing()
        }

        let take_stop = self.calculate_stop_and_take(atr_value);

        println!("CLOSE {}, RSI {}, Datetime {}-{}", closed_candle.close, rsi_value, closed_candle.date, closed_candle.close_time);
        match self.is_order_active {
            None => {
                if rsi_value > 70.0 && closed_candle.start_time < 17_00_00_000 {
                    self.is_order_active = Some(Signal::SHORT);
                    self.entry_price = closed_candle.close;
                    println!("COMPRANDO, RSI {}, ENTRY_PRICE {}, DateTime {}-{}", rsi_value, self.entry_price, closed_candle.date, closed_candle.close_time);
                    return TradeAction::send_order(MarketShort);
                } else if rsi_value < 30.0 && closed_candle.start_time < 17_00_00_000 {
                    self.is_order_active = Some(Signal::LONG);
                    self.entry_price = closed_candle.close;
                    println!("COMPRANDO, RSI {}, ENTRY_PRICE {}, DateTime {}-{}", rsi_value, self.entry_price, closed_candle.date, closed_candle.close_time);
                    return TradeAction::send_order(MarketLong);
                }
            }
            Some(Signal::SHORT) => {
                assert_ne!(take_stop.0, 0.0);
                assert_ne!(take_stop.1, 0.0);

                if rsi_value <= 50.0
                    // || closed_candle.close <= take_stop.0
                    // || closed_candle.close >= take_stop.1
                    || closed_candle.start_time > 17_40_00_000 {
                    // println!("{} {} {} {}", rsi_value <= 50.0, closed_candle.close <= take_stop.0, closed_candle.close >= take_stop.1, closed_candle.start_time > 17_40_00);
                    println!("VENDENDO SHORT, RSI {}, BALANCE {}, DateTime {}-{}", rsi_value, closed_candle.close - self.entry_price, closed_candle.date, closed_candle.close_time);
                    self.is_order_active = None;
                    self.entry_price = 0.0;
                    return TradeAction::send_order(MarketLong);
                }
            }
            Some(Signal::LONG) => {
                assert_ne!(take_stop.0, 0.0);
                assert_ne!(take_stop.1, 0.0);

                if rsi_value >= 50.0 ||
                    // closed_candle.close >= take_stop.0 ||
                    // closed_candle.close <= take_stop.1 ||
                    closed_candle.start_time > 17_40_00 {
                    // println!("{} {} {} {}", rsi_value >= 50.0, closed_candle.close >= take_stop.0, closed_candle.close <= take_stop.1, closed_candle.start_time > 17_40_00);
                    println!("VENDENDO LONG, RSI {}, BALANCE {}, DateTime {}-{}", rsi_value, self.entry_price - closed_candle.close, closed_candle.date, closed_candle.close_time);
                    self.is_order_active = None;
                    self.entry_price = 0.0;
                    return TradeAction::send_order(MarketShort);
                }
            }
        }

        TradeAction::do_nothing()
    }
}