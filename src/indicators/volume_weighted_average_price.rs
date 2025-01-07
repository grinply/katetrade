

/// The volume-weighted average price (VWAP) is a technical analysis indicator used on intraday
/// charts that resets at the start of every new trading session. It's the average price a security
/// has traded at throughout the day, based on both volume and price.
/// Calculation:
/// Typical Price = ((High + Low + Close)/3)
/// Cumulative = Total trades since the trading session opened
/// VolumeWeightedAveragePrice (VWAP) = Cumulative Typical Price x Volume / Cumulative Volume


use std::fmt;
use ta::{Close, High, Low, Next, Volume};

#[derive(Clone, Debug)]
pub struct VolumeWeightedAveragePrice {
    index: usize,
    current_value: f64,
    cumulative_price_volume: f64,
    cumulative_volume: f64
}

#[allow(unused)]
impl VolumeWeightedAveragePrice {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            index: 0,
            current_value: 0.0,
            cumulative_price_volume: 0.0,
            cumulative_volume: 0.0
        })
    }

    pub fn value(&self) -> f64 {
        self.current_value
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.current_value = 0.0;
        self.cumulative_price_volume = 0.0;
        self.cumulative_volume = 0.0;
    }
}

impl<T: High + Low + Close + Volume> Next<&T> for VolumeWeightedAveragePrice {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        let typical_price_volume = ((input.high() + input.low() + input.close()) / 3.0) * input.volume();
        self.cumulative_price_volume += typical_price_volume;
        self.cumulative_volume += input.volume();

        self.current_value = self.cumulative_price_volume / self.cumulative_volume;

        self.index += 1;
        self.current_value
    }
}

impl fmt::Display for VolumeWeightedAveragePrice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VolumeWeightedAveragePrice(current_value {})", self.current_value)
    }
}

#[cfg(test)]
mod tests {
    use ta::Next;
    use crate::indicators::volume_weighted_average_price::VolumeWeightedAveragePrice;
    use crate::models::candlestick::Candlestick;

    #[test]
    fn test_new() {
        let vwap = VolumeWeightedAveragePrice::new().unwrap();
        assert_eq!(vwap.index, 0);
        assert_eq!(vwap.cumulative_price_volume, 0.0);
        assert_eq!(vwap.cumulative_volume, 0.0);
    }

    #[test]
    fn test_next() {
        let test_data = vec![
            // high, low , close, volume, expected
            (150.5, 145.0, 148.0, 5000, 147.83),
            (155.0, 147.5, 153.0, 4000, 149.61),
            (160.0, 150.0, 155.0, 6000, 151.77)
        ];
        let mut expected_index = 0usize;

        let mut vwap = VolumeWeightedAveragePrice::new().unwrap();

        for (high, low, close, volume, expected) in test_data {
            expected_index += 1;
            let mut candle = Candlestick::default();
            candle.high = high;
            candle.low = low;
            candle.close = close;
            candle.volume = volume;
            assert_eq!(((vwap.next(&candle) * 100.0).round() / 100.0), expected);
            assert_eq!(vwap.get_index(), expected_index);
        }
    }

    #[test]
    fn test_reset() {
        let mut vwap = VolumeWeightedAveragePrice::new().unwrap();
        let mut candle = Candlestick::default();
        candle.high = 150.0;
        candle.low = 100.0;
        candle.close = 130.0;
        candle.volume = 100;
        assert_eq!(vwap.next(&candle).round(), 127.0);

        vwap.reset();

        assert_eq!(vwap.current_value, 0.0);
        assert_eq!(vwap.cumulative_price_volume, 0.0);
        assert_eq!(vwap.cumulative_volume, 0.0);
    }
}