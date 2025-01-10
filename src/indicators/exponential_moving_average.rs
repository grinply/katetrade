use std::fmt;
use ta::{Close, Next};
use crate::models::candlestick::Candlestick;

pub struct ExponentialMovingAverage {
    period: usize,
    count: usize,
    k: f64,
    current: f64,
    is_new: bool,
    results: Box<[f64]>,
}

#[allow(unused)]
impl ExponentialMovingAverage {
    pub fn new(period: usize, weight: f64) -> Self {
        assert!(weight >= 0.0 && weight <= 1.0, "weight should be a value between 0 and 1");
        Self {
            period,
            k: weight,
            current: 0.0,
            count: 0,
            is_new: true,
            results: vec![0.0; period].into_boxed_slice(),
        }
    }

    pub fn default(period: usize) -> Self {
        Self {
            period,
            k: 2.0 / (period + 1) as f64,
            current: 0.0,
            count: 0,
            is_new: true,
            results: vec![0.0; period].into_boxed_slice(),
        }
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn value(&self) -> f64 {
        self.current
    }

    pub fn reset(&mut self) {
        self.current = 0.0;
        self.count = 0;
        self.is_new = true;

        for i in 0..self.period {
            self.results[i] = 0.0;
        }
    }

    pub fn turns_up(&self) -> bool {
        if self.period < 3 {
            unimplemented!();
        }

        assert!(self.count >= 3, "The ExponentialMovingAverage turns_up method needs to have at least 3 candle period.");

        let last_but_one_value = self.results[2];
        let last_value = self.results[1];
        let current_value = self.results[0];

        if self.count >= 3 && last_but_one_value >= last_value && current_value > last_value {
            return true;
        }
        false
    }

    pub fn turns_down(&self) -> bool {
        if self.period < 3 {
            unimplemented!();
        }

        assert!(self.count >= 3, "The ExponentialMovingAverage turns_down method needs to have at least 3 candle period");

        let last_but_one_value = self.results[2];
        let last_value = self.results[1];
        let current_value = self.results[0];

        if self.count >= 3 && last_but_one_value <= last_value && current_value < last_value {
            return true;
        }
        false
    }

}

impl Next<f64> for ExponentialMovingAverage {
    type Output = f64;

    fn next(&mut self, close_value: f64) -> Self::Output {

        if self.count < self.period {
            self.count += 1;
        }

        if self.is_new {
            self.is_new = false;
            self.current = close_value;
        } else {
            self.current = self.k * close_value + (1.0 - self.k) * self.current;
        }

        self.results.rotate_right(1);
        self.results[0] = self.current;

        self.current
    }
}

impl Next<Candlestick> for ExponentialMovingAverage {
    type Output = f64;

    fn next(&mut self, candle: Candlestick) -> Self::Output {
        self.next(candle.close)
    }
}

impl<T: Close> Next<&T> for ExponentialMovingAverage {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        self.next(input.close())
    }
}

impl fmt::Display for ExponentialMovingAverage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EMA({})", self.period)
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::date;
    use super::*;

    #[test]
    fn test_new() {
        let ema = ExponentialMovingAverage::default(5);
        assert_eq!(ema.period, 5);

        let ema = ExponentialMovingAverage::default(0);
        assert_eq!(ema.period, 0);
    }

    #[test]
    fn test_next() {
        let mut ema = ExponentialMovingAverage::default(3);

        assert_eq!(ema.next(&new_candle(2.0)), 2.0);
        assert_eq!(ema.next(&new_candle(5.0)), 3.5);
        assert_eq!(ema.next(&new_candle(1.0)), 2.25);
        assert_eq!(ema.next(&new_candle(6.25)), 4.25);

        let mut ema = ExponentialMovingAverage::default(3);
        let candle1 = new_candle(2.0);
        let candle2 = new_candle(5.0);
        assert_eq!(ema.next(&candle1), 2.0);
        assert_eq!(ema.next(&candle2), 3.5);
    }

    #[test]
    fn test_display() {
        let ema = ExponentialMovingAverage::default(7);
        assert_eq!(format!("{}", ema), "EMA(7)");
    }

    fn new_candle(close: f64) -> Candlestick {
        Candlestick{
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close,
            volume: 0,
            buy_volume: 0,
            sell_volume: 0,
            amount_traded: 0.0,
            date: 0,
            start_time: date::convert_to_naive_time(09_00_00).unwrap(),
            close_time: None,
            is_closed: false,
            trades: 0,
            buy_trades: 0,
            sell_trades: 0,
            trade_imbalance: 0,
            directional_strength: 0.0,
            cumulative_volume_delta: 0,
        }
    }
}