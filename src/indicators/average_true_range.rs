use crate::indicators::exponential_moving_average::ExponentialMovingAverage as Ema;
use crate::indicators::true_range::TrueRange;
use std::fmt;
use ta::{Close, High, Low, Next, Period};

pub struct AverageTrueRange {
    true_range: TrueRange,
    ema: Ema,
}
#[allow(unused)]
impl AverageTrueRange {
    pub fn new(period: usize) -> Self {
        Self {
            true_range: TrueRange::new(),
            ema: Ema::new(period),
        }
    }

    pub fn value(&self) -> f64 {
        self.ema.value()
    }

    pub fn reset(&mut self) {
        self.true_range.reset();
        self.ema.reset();
    }
}

impl Period for AverageTrueRange {
    fn period(&self) -> usize {
        self.ema.period()
    }
}

impl Next<f64> for AverageTrueRange {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        self.ema.next(self.true_range.next(input))
    }
}

impl<T: High + Low + Close> Next<&T> for AverageTrueRange {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        self.ema.next(self.true_range.next(input))
    }
}

impl fmt::Display for AverageTrueRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ATR({})", self.ema.period())
    }
}

#[cfg(test)]
mod tests {
    use crate::models::candlestick::Candlestick;
    use super::*;

    #[test]
    fn test_new() {
        let atr = AverageTrueRange::new(10);
        assert_eq!(atr.period(), 10);
    }
    #[test]
    fn test_next() {
        let mut atr = AverageTrueRange::new(3);

        let candle1 = new_candle(10.0, 7.5,9.0);
        let candle2 = new_candle(11.0, 9.0,9.5);
        let candle3 = new_candle(9.0, 5.0,8.0);

        assert_eq!(atr.next(&candle1), 2.5);
        assert_eq!(atr.next(&candle2), 2.25);
        assert_eq!(atr.next(&candle3), 3.375);
    }

    #[test]
    fn test_display() {
        let indicator = AverageTrueRange::new(8);
        assert_eq!(format!("{}", indicator), "ATR(8)");
    }

    fn new_candle(high: f64, low: f64, close: f64) -> Candlestick {
        Candlestick{
            open: 0.0,
            high,
            low,
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