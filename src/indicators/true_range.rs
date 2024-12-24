use std::fmt;
use ta::{Close, High, Low, Next};

pub struct TrueRange {
    prev_close: Option<f64>,
}

impl TrueRange {
    pub fn new() -> Self {
        Self { prev_close: None }
    }

    pub fn reset(&mut self) {
        self.prev_close = None;
    }
}

impl fmt::Display for TrueRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TRUE_RANGE()")
    }
}

impl Next<f64> for TrueRange {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        let distance = match self.prev_close {
            Some(prev) => (input - prev).abs(),
            None => 0.0,
        };
        self.prev_close = Some(input);
        distance
    }
}

impl<T: High + Low + Close> Next<&T> for TrueRange {
    type Output = f64;

    fn next(&mut self, candle: &T) -> Self::Output {
        let max_dist = match self.prev_close {
            Some(prev_close) => {
                let dist1 = candle.high() - candle.low();
                let dist2 = (candle.high() - prev_close).abs();
                let dist3 = (candle.low() - prev_close).abs();
                dist1.max(dist2).max(dist3)
            }
            None => candle.high() - candle.low(),
        };
        self.prev_close = Some(candle.close());
        max_dist
    }
}

#[cfg(test)]
mod tests {
    use crate::models::candlestick::Candlestick;
    use crate::utils::date;
    use super::*;

    #[test]
    fn test_next_f64() {
        let mut tr = TrueRange::new();
        assert_eq!(tr.next(2.5), 0.0);
        assert_eq!(tr.next(3.6), 1.1);
        assert_eq!(tr.next(3.3), 0.30000000000000027);
    }

    #[test]
    fn test_next_candle() {
        let mut tr = TrueRange::new();

        let candle1 = new_candle(10.0,7.5, 9.0);
        let candle2 = new_candle(11.0,9.0, 9.5);
        let candle3 = new_candle(9.0,5.0, 8.0);

        assert_eq!(tr.next(&candle1), 2.5);
        assert_eq!(tr.next(&candle2), 2.0);
        assert_eq!(tr.next(&candle3), 4.5);
    }

    #[test]
    fn test_display() {
        let indicator = TrueRange::new();
        assert_eq!(format!("{}", indicator), "TRUE_RANGE()");
    }

    fn new_candle(high: f64, low: f64, close: f64) -> Candlestick {
        Candlestick{
            open: 0.0,
            high,
            low,
            close,
            volume: 0,
            amount_traded: 0.0,
            date: 0,
            start_time: date::convert_to_naive_time(09_00_00).unwrap(),
            close_time: None,
            is_closed: false,
            trades: 0,
        }
    }
}