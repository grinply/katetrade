use std::fmt;
use ta::{Close, Next};
use crate::models::candlestick::Candlestick;

pub struct ExponentialMovingAverage {
    period: usize,
    k: f64,
    current: f64,
    is_new: bool,
}

#[allow(unused)]
impl ExponentialMovingAverage {
    pub fn new(period: usize, weight: f64) -> Self {
        assert!(weight >= 0.0 && weight <= 1.0, "weight should be a value between 0 and 1");
        Self {
            period,
            k: weight,
            current: 0.0,
            is_new: true,
        }
    }

    pub fn default(period: usize) -> Self {
        Self {
            period,
            k: 2.0 / (period + 1) as f64,
            current: 0.0,
            is_new: true,
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
        self.is_new = true;
    }
}

impl Next<f64> for ExponentialMovingAverage {
    type Output = f64;

    fn next(&mut self, close_value: f64) -> Self::Output {
        if self.is_new {
            self.is_new = false;
            self.current = close_value;
        } else {
            self.current = self.k * close_value + (1.0 - self.k) * self.current;
        }
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
            volume: 0.0,
            date: 0,
            start_time: 0,
            close_time: 0,
            is_closed: false,
            trades: 0,
        }
    }
}