use std::collections::HashMap;
use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::models::action::{TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickType};
use crate::strategies::strategy::{CandlestickConfiguration, CandlestickTradingStrategy};


#[derive(Debug, PartialEq)]
enum Signal {
    SHORT,
    LONG
}


#[derive(Debug)]
#[allow(unused)]
pub struct ReversalMomentumCandlestickStrategy {
    signal: Option<Signal>,
    entry_price: f64,
    order_active: bool,
    is_order_sent: bool,
    day_last_order: i32,
}

impl CandlestickTradingStrategy for ReversalMomentumCandlestickStrategy {
    fn new() -> Self {
        ReversalMomentumCandlestickStrategy{ signal: None, entry_price: 0.0, order_active: false, is_order_sent: false, day_last_order: -1 }
    }

    fn name(&self) -> &str {
        "reversal_momentum_candlestick_strategy"
    }
    fn days_of_context(&self) -> u32 {
        2
    }

    fn init_trading_day(&mut self, _: &Asset, context_candles: &[Candlestick]) {
        let candles = aggregate_daily_candles(context_candles);

        let last_but_one_candlestick = candles.first().unwrap();
        let last_candlestick = candles.last().unwrap();

        self.signal = signal_calculation(last_but_one_candlestick, last_candlestick);

        if !self.signal.is_none() {
            self.entry_price = get_entry_price(last_candlestick, self.signal.as_ref().unwrap());
        }
    }

    fn candlestick_configuration(&self) -> CandlestickConfiguration {
        CandlestickConfiguration {
            candle_type: CandlestickType::TICK,
            candle_input_parameter: 1000,
        }
    }

    fn process_closed_candlestick(&mut self, closed_candle: &Candlestick) -> TradeAction {
        // Exiting Day
        if self.is_order_sent && self.order_active {
            if (closed_candle.close_time > 17_00_00) && (closed_candle.close_time <= 17_50_00) {
                match self.signal.as_ref().unwrap() {
                    Signal::LONG => {
                        if self.entry_price < closed_candle.close {
                            self.order_active = false;
                            return TradeAction::send_order(MarketShort)
                        }
                    },
                    Signal::SHORT => {
                        if self.entry_price > closed_candle.close {
                            self.order_active = false;
                            return TradeAction::send_order(MarketLong)
                        }
                    }
                }
            } else if closed_candle.close_time > 17_50_00 {
                return match self.signal.as_ref().unwrap() {
                    Signal::LONG => {
                        self.order_active = false;
                        TradeAction::send_order(MarketShort)
                    },
                    Signal::SHORT => {
                        self.order_active = false;
                        TradeAction::send_order(MarketLong)
                    }
                }
            }
        }

        if self.day_last_order != closed_candle.date {
            self.is_order_sent = false;
            self.day_last_order = closed_candle.date;
        }

        if self.is_order_sent {
            return TradeAction::do_nothing();
        }

        // Starting Day
        match self.signal {
            None => {
                return TradeAction::do_nothing()
            }
            Some(Signal::LONG) => {
                assert_ne!(self.entry_price, 0.0);
                if !self.order_active && closed_candle.close >=  self.entry_price{
                    self.order_active = true;
                    self.is_order_sent = true;
                    return TradeAction::send_order(MarketLong)
                }
            }
            Some(Signal::SHORT) => {
                assert_ne!(self.entry_price, 0.0);
                if !self.order_active  && closed_candle.close <=  self.entry_price{
                    self.order_active = true;
                    self.is_order_sent = true;
                    return TradeAction::send_order(MarketShort)
                }
            }
        }

        TradeAction::do_nothing()
    }
}
fn get_entry_price(candlestick: &Candlestick, signal: &Signal) -> f64 {
    match signal {
        Signal::LONG => {
            candlestick.low
        }
        Signal::SHORT => {
            candlestick.high
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

    // Coletar candles agregados em um vetor
    let mut aggregated_candles: Vec<Candlestick> = daily_map.into_values().collect();

    assert_eq!(aggregated_candles.len(), 2, "Trading init days should be exactly two days");

    // Ordenar por data se necessário
    aggregated_candles.sort_by(|a, b| a.date.cmp(&b.date));

    aggregated_candles
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

#[cfg(test)]
mod reversal_momentum_tests {
    use super::*;
    #[test]
    fn test_return_entry_price_long() {
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
        assert_eq!(get_entry_price(last_candle, &Signal::LONG), 1.0)
    }

    #[test]
    fn test_return_entry_price_short() {
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
        assert_eq!(get_entry_price(last_candle, &Signal::SHORT), 3.0)
    }

    #[test]
    #[should_panic(expected = "Trading init days should be exactly two days")]
    fn test_aggregate_fails_with_less_than_two_days() {
        let candles = vec![
            Candlestick { date: 20231110, open: 130821.754224, high: 132715.797145, low: 130758.852429, close: 132541.069938, volume: 1500.0, start_time: 93000, close_time: 93100, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 132792.677116, high: 132932.458881, low: 131059.383225, close: 132205.593701, volume: 1500.0, start_time: 93100, close_time: 93200, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 132513.113585, high: 132513.113585, low: 131464.750344, close: 132506.124497, volume: 1500.0, start_time: 93200, close_time: 93300, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 131967.964700, high: 134323.287447, low: 131765.281140, close: 133868.996709, volume: 1500.0, start_time: 93300, close_time: 93400, is_closed: true, trades: 300 },
            Candlestick { date: 20231110, open: 133715.236767, high: 135301.759804, low: 133610.400443, close: 134581.883712, volume: 1500.0, start_time: 93400, close_time: 93500, is_closed: true, trades: 300 },
        ];

        aggregate_daily_candles(&candles);
    }

    #[test]
    #[should_panic(expected = "Trading init days should be exactly two days")]
    fn test_aggregate_fails_with_more_than_two_days() {
        let candles = vec![
            Candlestick { date: 20231109, open: 95.0, high: 105.0, low: 90.0, close: 100.0, volume: 1000.0, start_time: 93000, close_time: 100000, is_closed: true, trades: 200 },
            Candlestick { date: 20231110, open: 100.0, high: 110.0, low: 99.0, close: 105.0, volume: 1500.0, start_time: 93000, close_time: 100000, is_closed: true, trades: 300 },
            Candlestick { date: 20231111, open: 111.0, high: 120.0, low: 110.0, close: 119.0, volume: 1800.0, start_time: 93000, close_time: 100000, is_closed: true, trades: 350 },
        ];

        aggregate_daily_candles(&candles);
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