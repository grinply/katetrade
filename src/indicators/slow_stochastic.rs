use std::fmt;
use ta::{Close, High, Low, Next, Period, Reset};
use ta::indicators::FastStochastic;
use crate::indicators::simple_moving_average::SimpleMovingAverage;

/// Slow stochastic oscillator.
///
/// Basically it is a fast stochastic oscillator smoothed with exponential moving average.
///
/// # Parameters
///
/// * _stochastic_period_ - number of periods for fast stochastic (integer greater than 0). Default is 14.
/// * _ema_period_ - period for SMA (integer greater than 0). Default is 3.
///
/// # Example
///
/// ```
/// use ta::indicators::SlowStochastic;
/// use ta::Next;
///
/// let mut stoch = SlowStochastic::new(3, 2).unwrap();
/// assert_eq!(stoch.next(10.0), 50.0);
/// assert_eq!(stoch.next(50.0).round(), 83.0);
/// assert_eq!(stoch.next(50.0).round(), 94.0);
/// assert_eq!(stoch.next(30.0).round(), 31.0);
/// assert_eq!(stoch.next(55.0).round(), 77.0);
/// ```
#[derive(Clone, Debug)]
pub struct SlowStochastic {
    fast_stochastic: FastStochastic,
    sma: SimpleMovingAverage,
}

impl SlowStochastic {
    pub fn new(stochastic_period: usize, sma_period: usize) -> Result<Self, anyhow::Error> {
        Ok(Self {
            fast_stochastic: FastStochastic::new(stochastic_period)?,
            sma: SimpleMovingAverage::new(sma_period)?,
        })
    }
}

impl Next<f64> for SlowStochastic {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        self.sma.next(self.fast_stochastic.next(input))
    }
}

impl<T: High + Low + Close> Next<&T> for SlowStochastic {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        self.sma.next(self.fast_stochastic.next(input))
    }
}

impl Reset for SlowStochastic {
    fn reset(&mut self) {
        self.fast_stochastic.reset();
        self.sma.reset();
    }
}

impl Default for SlowStochastic {
    fn default() -> Self {
        Self::new(14, 3).unwrap()
    }
}

impl fmt::Display for SlowStochastic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SLOW_STOCH({}, {})",
            self.fast_stochastic.period(),
            self.sma.period()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::models::candlestick::Candlestick;
    use super::*;

    #[test]
    fn test_new() {
        assert!(SlowStochastic::new(0, 1).is_err());
        assert!(SlowStochastic::new(1, 0).is_err());
        assert!(SlowStochastic::new(1, 1).is_ok());
    }

    #[test]
    fn test_next_with_f64() {
        let mut stoch = SlowStochastic::new(3, 2).unwrap();
        assert_eq!(stoch.next(10.0), 50.0);
        assert_eq!(stoch.next(50.0).round(), 75.0);
        assert_eq!(stoch.next(50.0).round(), 100.0);
        assert_eq!(stoch.next(30.0).round(), 50.0);
        assert_eq!(stoch.next(55.0).round(), 50.0);
    }

    #[test]
    fn test_next_with_bars() {
        let test_data = vec![
            // high, low , close, expected
            (30.0, 10.0, 25.0, 75.0),
            (20.0, 20.0, 20.0, 63.0),
            (40.0, 20.0, 16.0, 35.0),
            (35.0, 15.0, 19.0, 18.0),
            (30.0, 20.0, 25.0, 28.0),
            (35.0, 25.0, 30.0, 58.0),
        ];

        let mut stoch = SlowStochastic::new(3, 2).unwrap();

        for (high, low, close, expected) in test_data {
            let mut candle = Candlestick::default();
            candle.high = high;
            candle.low = low;
            candle.close = close;
            assert_eq!(stoch.next(&candle).round(), expected);
        }
    }

    #[test]
    fn test_reset() {
        let mut stoch = SlowStochastic::new(3, 2).unwrap();
        assert_eq!(stoch.next(10.0), 50.0);
        assert_eq!(stoch.next(50.0).round(), 75.0);
        assert_eq!(stoch.next(50.0).round(), 100.0);

        stoch.reset();
        assert_eq!(stoch.next(10.0), 50.0);
    }

    #[test]
    fn test_default() {
        SlowStochastic::default();
    }

    #[test]
    fn test_display() {
        let indicator = SlowStochastic::new(10, 2).unwrap();
        assert_eq!(format!("{}", indicator), "SLOW_STOCH(10, 2)");
    }
}
