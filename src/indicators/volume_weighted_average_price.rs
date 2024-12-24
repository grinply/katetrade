
/***
VolumeWeightedAveragePrice = ((High + Low + Close)/3) de todos os candles desde a abertura * Volume do candle / Soma de todos os volumes desde a abertura
 */

use std::fmt;
use ta::{Close, High, Low, Next, Volume};

#[derive(Clone, Debug)]
pub struct VolumeWeightedAveragePrice {
    index: usize,
    current_value: f64,
    cumulative_typical_price: f64,
    cumulative_volume: f64
}

#[allow(unused)]
impl VolumeWeightedAveragePrice {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            index: 0,
            current_value: 0.0,
            cumulative_typical_price: 0.0,
            cumulative_volume: 0.0
        })
    }

    pub fn value(&self) -> f64 {
        self.current_value
    }


    pub fn reset(&mut self) {
        self.index = 0;
        self.current_value = 0.0;
        self.cumulative_typical_price = 0.0;
        self.cumulative_volume = 0.0;
    }
}

impl<T: High + Low + Close + Volume> Next<&T> for VolumeWeightedAveragePrice {
    type Output = f64;

    // VWAP is calculated using the following steps:
    //
    // Compute the typical price for the intraday period, which is the average of the high, low, and close: ((H+L+C)/3).
    //
    // Multiply the typical price by the period's volume.
    //
    // Create a running total of these values, known as a cumulative total.
    //
    // Create a running total of volume (cumulative volume).
    //
    // Divide the running total of price-volume by the running total of volume.
    //
    // The formula is:
    //
    // Cumulative(Volume x Typical Price)/Cumulative(Volume)
    fn next(&mut self, input: &T) -> Self::Output {
        let vwap_period = ((input.high() + input.low() + input.close()) / 3.0) * input.volume();
        self.cumulative_typical_price += vwap_period;
        self.cumulative_volume += input.volume();

        self.current_value = self.cumulative_typical_price / self.cumulative_volume;
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
        assert_eq!(vwap.cumulative_typical_price, 0.0);
        assert_eq!(vwap.cumulative_volume, 0.0);
    }

    #[test]
    fn test_next() {
        let test_data = vec![
            // high, low , close, volume, expected
            (20.0, 15.0, 16.0, 10.0, 17.0),
            (5.0, 3.0, 4.0, 100.0, 5.18),
            // (6.0, 4.0, 5.0, 150.0, 5.0),
            // (7.0, 5.0, 6.0, 200.0, 6.0),
            // (8.0, 6.0, 7.0, 120.0, 6.999999999999999),
            // (10.0, 8.0, 9.0, 180.0, 8.999999999999998),
        ];

        let mut vwap = VolumeWeightedAveragePrice::new().unwrap();

        for (high, low, close, volume, expected) in test_data {
            let mut candle = Candlestick::default();
            candle.high = high;
            candle.low = low;
            candle.close = close;
            candle.volume = volume;
            assert_eq!(((vwap.next(&candle) * 100.0).round() / 100.0), expected);
        }
    }
}