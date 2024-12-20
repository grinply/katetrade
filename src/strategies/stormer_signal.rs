#![allow(unused)]
use chrono::{NaiveTime, Timelike};
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::{Next, Reset};
use crate::indicators::relative_strength_index::RelativeStrengthIndex;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::candlestick::CandleDirection::{LONG, SHORT};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::strategies::strategy::HighResolutionTradingStrategy;
const CANDLES_TO_TRACK: usize = 3;
const CANDLESTICK_PERIOD: u64 = 10;
const BOLLINGER_PERIOD: usize = 21;
const BOLLINGER_MULTIPLIER: f64 = 0.45;
const SLOW_STOCHASTIC_PERIOD: usize = 8;
const SLOW_STOCHASTIC_SMA_PERIOD: usize = 3;
const STOCHASTIC_LONG_THRESHOLD: f64 = 80.0;
const STOCHASTIC_SHORT_THRESHOLD: f64 = 20.0;
const RSI_LONG_THRESHOLD: f64 = 70.0;
const RSI_SHORT_THRESHOLD: f64 = 30.0;
const RSI_PERIOD: usize = 9;
const SIGNAL_DURATION_CANDLES: usize = 6;

pub struct StormerIndicator {
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    min_candles: usize,
    candle_count: usize,
    bollinger_bands: BollingerBands,
    stochastic: SlowStochastic,
    candle_parser: TimeCandlestickGenerator,
    latest_closed_candles: [Candlestick; CANDLES_TO_TRACK],
    rsi: RelativeStrengthIndex,
    current_signal_count: usize,
}

impl StormerIndicator {
    fn default() -> Self
    where
        Self: Sized,
    {
        StormerIndicator {
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(10, 30, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(10, 40, 00).unwrap(),
            rsi: RelativeStrengthIndex::new(RSI_PERIOD),

            candle_count: 0,
            min_candles: 21,
            bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, CANDLESTICK_PERIOD),
            latest_closed_candles: [Candlestick::default(); CANDLES_TO_TRACK],
            current_signal_count: 0,
        }
    }

    fn reset(&mut self) {
        self.candle_count = 0;
        self.current_signal_count = 0;
        self.rsi.reset();
        self.bollinger_bands.reset();
        self.stochastic.reset();
        self.latest_closed_candles = [Candlestick::default(); CANDLES_TO_TRACK];
    }

    fn next(&mut self, new_trade: &Trade) -> f64 {
        let signal = if let Some(candlestick) = self.candle_parser.parse_only_closed_candles(new_trade) {
            if self.current_signal_count > 0 {
                self.current_signal_count -= 1;
            }
            self.candle_count += 1;
            let stochastic = self.stochastic.next(candlestick.close);
            let bollinger_output = self.bollinger_bands.next(candlestick.close);
            let rsi = self.rsi.next(candlestick.close);
            self.update_candles(candlestick);

            if self.should_open_position(stochastic, bollinger_output, rsi).is_some() {
                self.current_signal_count = SIGNAL_DURATION_CANDLES;
                1.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        if self.current_signal_count > 0 {
            return 1.0;
        }

        signal
    }

    fn update_candles(&mut self, new_closed_candle: Candlestick) {
        for index in 0..CANDLES_TO_TRACK - 1 {
            self.latest_closed_candles[index] = self.latest_closed_candles[index + 1];
        }
        self.latest_closed_candles[CANDLES_TO_TRACK - 1] = new_closed_candle;
    }

    fn should_open_position(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput, rsi: f64) -> Option<Side> {
        let first_candle = self.latest_closed_candles[CANDLES_TO_TRACK - 1];
        if self.candle_count < self.min_candles || first_candle.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() >= self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        let second_candle = self.latest_closed_candles[CANDLES_TO_TRACK - 2];
        let current_price = first_candle.close;

        // "long" signals
        if current_price > bollinger_output.upper &&
            rsi < RSI_LONG_THRESHOLD &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            second_candle.direction() == SHORT &&
            first_candle.direction() == LONG {
            return Some(Side::BUY);
        }

        // "short" signals
        if current_price < bollinger_output.lower &&
            rsi > RSI_SHORT_THRESHOLD &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            second_candle.direction() == LONG &&
            first_candle.direction() == SHORT {
            return Some(Side::SELL);
        }

        None
    }
}