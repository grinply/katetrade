use ta::{Close, High, Low, Next, Reset};
use crate::indicators::average_true_range::AverageTrueRange;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::indicators::true_range::TrueRange;

/// https://en.wikipedia.org/wiki/Average_directional_movement_index
pub struct AverageDirectionalIndex {
    is_init: bool,
    period: usize,
    prev_close: f64,
    prev_high: f64,
    prev_low: f64,
    sma_positive_directional_move: SimpleMovingAverage,
    sma_negative_directional_move: SimpleMovingAverage,
    sma_adx_factor: SimpleMovingAverage,
    average_true_range: AverageTrueRange,
}

impl AverageDirectionalIndex {
    pub fn new(period: usize) -> Self {
        AverageDirectionalIndex {
            is_init: false,
            prev_close: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            sma_positive_directional_move: SimpleMovingAverage::new(period).unwrap(),
            sma_negative_directional_move: SimpleMovingAverage::new(period).unwrap(),
            sma_adx_factor: SimpleMovingAverage::new(period).unwrap(),
            average_true_range: AverageTrueRange::new(period),
            period,
        }
    }
}

impl<T: High + Low + Close> Next<&T> for AverageDirectionalIndex {
    type Output = AverageDirectionalIndexOutput;

    fn next(&mut self, input: &T) ->  AverageDirectionalIndexOutput{
        if !self.is_init {
            self.prev_high = input.high();
            self.prev_low = input.low();
            self.prev_close = input.close();

            if self.average_true_range.next(input) > 0.0 {
                self.is_init = true;
            }

            return AverageDirectionalIndexOutput {
                positive: 0.0,
                negative: 0.0,
                adx: 0.0,
            };
        }

        let dm_plus = if input.high() > self.prev_high {
            input.high() - self.prev_high
        }else{
            0.0
        };

        let dm_minus = if self.prev_low > input.low() {
            self.prev_low - input.low()
        }else {
            0.0
        };

        let sma_dm_plus = self.sma_positive_directional_move.next(dm_plus);
        let sma_dm_minus = self.sma_negative_directional_move.next(dm_minus);
        let atr = self.average_true_range.next(input.close());

        let di_plus = 100.0 * (sma_dm_plus / atr);
        let di_minus = 100.0 * (sma_dm_minus / atr);

        let dx = ((di_plus - di_minus).abs() / (di_plus + di_minus).abs());

        self.prev_close = input.close();
        self.prev_high = input.high();
        self.prev_low = input.low();

        let adx = self.sma_adx_factor.next(dx);

        AverageDirectionalIndexOutput {
            positive: di_plus,
            negative: di_minus,
            adx,
        }
    }
}

impl Reset for AverageDirectionalIndex {
    fn reset(&mut self) {
        self.sma_adx_factor.reset();
        self.sma_positive_directional_move.reset();
        self.sma_negative_directional_move.reset();
        self.average_true_range.reset();
        self.prev_low = 0.0;
        self.prev_high = 0.0;
        self.prev_close = 0.0;
    }
}

impl Default for AverageDirectionalIndex {
    fn default() -> Self {
        Self::new(14)
    }
}

pub struct AverageDirectionalIndexOutput {
    pub positive: f64,
    pub negative: f64,
    pub adx: f64,
}