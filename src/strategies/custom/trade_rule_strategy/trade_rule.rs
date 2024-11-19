use crate::indicators::simple_moving_average::SimpleMovingAverage as SMA;
use ta::indicators::MovingAverageConvergenceDivergence as MACD;
use ta::indicators::ExponentialMovingAverage as EMA;
use ta::indicators::RelativeStrengthIndex as RSI;
use ta::indicators::AverageTrueRange as ATR;

use ta::{Next, Reset};
use crate::utils::math::{percentage_difference, value_on_range};

use crate::models::candlestick::Candlestick;
use crate::models::side::Side;
use crate::strategies::strategy::{CandlestickTradingStrategy};

pub(crate) const RULE_CHROMOSOME_SIZE: usize = 65;

pub struct TradeRule {
    direction: Side,
    sma: SMA,
    rsi: RSI,
    macd: MACD,
    ema: EMA,
    take_profit_atr: ATR,
    stop_loss_atr: ATR,

    macd_target_value: f64,
    macd_fast_period: usize,
    macd_slow_period: usize,


    rsi_lower_bound: f64,
    rsi_upper_bound: f64,

    take_profit_atr_value: f64,
    stop_loss_atr_value: f64,

    pub ema_min_percentage_diff_from_price: f64,
    pub sma_max_percentage_diff_from_ema: f64,

    take_profit_target_atr: f64,
    stop_loss_target_atr: f64,

    sma_take_profit: SMA,
    sma_stop_loss: SMA,
}

impl TradeRule {
    pub fn reset(&mut self) {
        self.sma.reset();
        self.rsi.reset();
        self.macd.reset();
        self.ema.reset();
        self.take_profit_atr.reset();
        self.stop_loss_atr.reset();
        self.sma_take_profit.reset();
        self.sma_stop_loss.reset();
    }

    pub fn new(direction: Side, chromosome: &[f32]) -> Self {
        // first relative strength index
        let rsi1_upper_bound = value_on_range((1.0, 100.0), chromosome[0] as f64);
        let rsi1_lower_bound = value_on_range((1.0, 100.0), chromosome[1] as f64);
        let rsi1_period = value_on_range((1.0, 100.0), chromosome[2] as f64) as usize;

        //percentage difference from the last candle
        let pd_upper_bound = value_on_range((1.0, 100.0), chromosome[3] as f64);
        let pd_lower_bound = value_on_range((1.0, 100.0), chromosome[4] as f64);

        // rising and falling rate from last N period
        let rr_upper_bound = value_on_range((1.0, 100.0), chromosome[5] as f64);
        let rr_lower_bound = value_on_range((1.0, 100.0), chromosome[6] as f64);
        let rr_period = value_on_range((1.0, 100.0), chromosome[6] as f64) as usize;

        // simple moving average
        let sma_upper_bound = value_on_range((1.0, 100.0), chromosome[6] as f64);
        let sma_lower_bound = value_on_range((1.0, 100.0), chromosome[6] as f64);
        let sma_period = value_on_range((1.0, 100.0), chromosome[6] as f64) as usize;

        // second relative strength index

        // let macd_fast_period = value_on_range((2.0, 100.0), chromosome[7] as f64) as usize;
        // let macd_slow_period = value_on_range((2.0, 100.0), chromosome[8] as f64) as usize;
        // let macd_singal_period = value_on_range((2.0, 100.0), chromosome[9] as f64) as usize;
        //
        // TradeRule {
        //     direction,
        //     take_profit_target_atr: value_on_range((0.1, 20.0), chromosome[0] as f64),
        //     take_profit_atr: ATR::new(value_on_range((2.0, 100.0), chromosome[1] as f64) as usize).unwrap(),
        //     stop_loss_target_atr: value_on_range((0.1, 20.0), chromosome[2] as f64),
        //     stop_loss_atr: ATR::new(value_on_range((2.0, 50.0), chromosome[3] as f64) as usize).unwrap(),
        //     rsi: RSI::new(rsi1_period).unwrap(),
        //     rsi_lower_bound: rsi1_lower_bound,
        //     rsi_upper_bound: rsi1_upper_bound,
        //     macd_fast_period,
        //     macd_slow_period,
        //     macd: MACD::new(macd_fast_period, macd_slow_period, macd_singal_period).unwrap(),
        //     macd_target_value: value_on_range((-1000.0, 1000.0), chromosome[10] as f64),
        //     ema: EMA::new(value_on_range((2.0, 100.0), chromosome[11] as f64) as usize).unwrap(),
        //     ema_min_percentage_diff_from_price: value_on_range((0.1, 100.0), chromosome[12] as f64),
        //     sma: SMA::new(value_on_range((2.0, 100.0), chromosome[13] as f64) as usize),
        //     sma_max_percentage_diff_from_ema: value_on_range((0.1, 100.0), chromosome[14] as f64),
        //     sma_take_profit: SMA::new(value_on_range((1.0, 100.0), chromosome[15] as f64) as usize),
        //     sma_stop_loss: SMA::new(value_on_range((1.0, 100.0), chromosome[16] as f64) as usize),
        //     stop_loss_atr_value: 0.0,
        //     take_profit_atr_value: 0.0,
        // }
    }

    pub(crate) fn evaluate(&mut self, candle: &Candlestick) -> bool {
        let rsi = self.rsi.next(candle);
        let macd = self.macd.next(candle).macd;
        let ema = self.ema.next(candle);
        let sma = self.sma.next(candle);
        let pd_ema_from_close = percentage_difference(ema, candle.close);
        let pd_sma_from_ema = percentage_difference(sma, ema);

        self.sma_take_profit.next(if self.direction == Side::BUY { candle.high } else { candle.low });
        self.sma_stop_loss.next(if self.direction == Side::BUY { candle.low } else { candle.high });

        self.take_profit_atr_value = self.take_profit_atr.next(candle);
        self.stop_loss_atr_value = self.stop_loss_atr.next(candle);

        //expression that evaluates if a trade should be opened
        macd > self.macd_target_value &&
            rsi > self.rsi_upper_bound && rsi < self.rsi_lower_bound &&
            pd_ema_from_close >= self.ema_min_percentage_diff_from_price ||
            pd_sma_from_ema <= self.sma_max_percentage_diff_from_ema
    }

    pub(crate) fn take_profit_price(&mut self) -> f64 {
        let diff = self.take_profit_atr_value * self.take_profit_target_atr;
        if self.direction == Side::BUY {
            self.sma_take_profit.max_value_on_period() + diff
        }else{
            self.sma_take_profit.min_value_on_period() - diff
        }
    }

    pub(crate) fn stop_loss_price(&self) -> f64 {
        let diff = self.stop_loss_atr_value * self.stop_loss_target_atr;
        if self.direction == Side::BUY {
            self.sma_stop_loss.min_value_on_period() - diff
        }else{
            self.sma_stop_loss.max_value_on_period() + diff
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.rsi_lower_bound < self.rsi_upper_bound && self.macd_fast_period < self.macd_slow_period
    }
}

