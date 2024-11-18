use std::fmt;
use ta::{Close, Next, Period};
use crate::models::candlestick::Candlestick;

pub struct SimpleMovingAverage {
    period: usize,
    index: usize,
    count: usize,
    sum: f64,
    deque: Box<[f64]>,
    current_value: f64,
}

impl SimpleMovingAverage {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            index: 0,
            count: 0,
            sum: 0.0,
            deque: vec![0.0; period].into_boxed_slice(),
            current_value: 0.0,
        }
    }

    pub fn value(&self) -> f64 {
        self.current_value
    }

    pub fn min_value_on_period(&self) -> f64 {
        let mut min = self.deque[0];
        for i in 1..self.deque.len() {
            if self.deque[i] < min {
                min = self.deque[i];
            }
        }

        min
    }

    pub fn max_value_on_period(&self) -> f64 {
        let mut max = self.deque[0];
        for i in 1..self.deque.len() {
            if self.deque[i] > max {
                max = self.deque[i];
            }
        }

        max
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.count = 0;
        self.sum = 0.0;
        self.current_value = 0.0;
        for i in 0..self.period {
            self.deque[i] = 0.0;
        }
    }
}

impl Period for SimpleMovingAverage {
    fn period(&self) -> usize {
        self.period
    }
}

impl Next<f64> for SimpleMovingAverage {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        let old_val = self.deque[self.index];
        self.deque[self.index] = input;

        self.index = if self.index + 1 < self.period {
            self.index + 1
        } else {
            0
        };

        if self.count < self.period {
            self.count += 1;
        }

        self.sum = self.sum - old_val + input;
        self.current_value = self.sum / (self.count as f64);
        self.current_value
    }
}

impl Next<Candlestick> for SimpleMovingAverage {
    type Output = f64;

    fn next(&mut self, candle: Candlestick) -> Self::Output {
        self.next(candle.close)
    }
}


impl<T: Close> Next<&T> for SimpleMovingAverage {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        self.next(input.close())
    }
}

impl fmt::Display for SimpleMovingAverage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SMA({})", self.period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let avg = SimpleMovingAverage::new(1);
        assert_eq!(avg.period, 1);
    }

    #[test]
    fn test_next() {
        let mut sma = SimpleMovingAverage::new(4);
        assert_eq!(sma.next(4.0), 4.0);
        assert_eq!(sma.next(5.0), 4.5);
        assert_eq!(sma.next(6.0), 5.0);
        assert_eq!(sma.next(6.0), 5.25);
        assert_eq!(sma.next(6.0), 5.75);
        assert_eq!(sma.next(6.0), 6.0);
        assert_eq!(sma.next(2.0), 5.0);
    }

    #[test]
    fn test_next_with_candlesticks() {
        let mut sma = SimpleMovingAverage::new(3);

        assert_eq!(sma.next(new_candle(4.0)), 4.0);
        assert_eq!(sma.next(new_candle(4.0)), 4.0);
        assert_eq!(sma.next(new_candle(7.0)), 5.0);
        assert_eq!(sma.next(new_candle(1.0)), 4.0);
    }

    #[test]
    fn test_display() {
        let sma = SimpleMovingAverage::new(5);
        assert_eq!(format!("{}", sma), "SMA(5)");
    }

    #[test]
    fn test_min_value(){
        let mut sma = SimpleMovingAverage::new(3);
        sma.next(3.0);
        sma.next(1.0);
        sma.next(5.0);
        assert_eq!(sma.min_value_on_period(), 1.0);

        sma.next(15.0);
        sma.next(31.0);
        assert_eq!(sma.min_value_on_period(), 5.0);

        sma.next(11.0);
        assert_eq!(sma.min_value_on_period(), 11.0);
    }

    #[test]
    fn test_max_value(){
        let mut sma = SimpleMovingAverage::new(3);
        sma.next(3.0);
        sma.next(1.0);
        sma.next(5.0);
        assert_eq!(sma.max_value_on_period(), 5.0);

        sma.next(15.0);
        assert_eq!(sma.max_value_on_period(), 15.0);
        sma.next(31.0);
        assert_eq!(sma.max_value_on_period(), 31.0);

        sma.next(11.0);
        assert_eq!(sma.max_value_on_period(), 31.0);
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