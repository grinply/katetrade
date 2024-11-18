use std::f32;


use indicators::simple_moving_average::SimpleMovingAverage as SMA;
use ta::indicators::MovingAverageConvergenceDivergence as MACD;
use ta::indicators::ExponentialMovingAverage as EMA;
use ta::indicators::RelativeStrengthIndex as RSI;
use ta::indicators::AverageTrueRange as ATR;

use ta::{Next, Reset};
use crate::indicators;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::Candlestick;
use crate::models::side::Side;
use crate::strategies::strategy::{CandlestickConfiguration, CandlestickTradingStrategy};
use crate::utils::math::{percentage_difference, value_on_range};

pub struct TradeRuleStrategy {
    long_rule: TradeRule,
    short_rule: TradeRule,
    is_long_position_open: bool,
    is_short_position_open: bool,
    quantity_to_trade: u8,
    stop_loss_price: f64,
    take_profit_price: f64
}

impl TradeRuleStrategy {
    pub fn decode(chromosome: &[f32]) -> Self {
        assert_eq!(18, chromosome.len(), "the chromosome must have length 36, but it has {}", chromosome.len());

        for i in 0..chromosome.len() {
            if chromosome[i] < 0.0 || chromosome[i] > 1.0 {
                panic!("the chromosome must have genes between 0.0 and 1.0, but it had {}", chromosome[i]);
            }
        }

        TradeRuleStrategy {
            quantity_to_trade: value_on_range((1.0, 10.0), chromosome[0] as f64)  as u8,
            long_rule: TradeRule::new(Side::BUY, &chromosome[1..=17]),
            //short_rule: TradeRule::new(Side::SELL, &chromosome[18..=35]),
            short_rule: TradeRule::new(Side::SELL, &chromosome[1..=17]),
            is_long_position_open: false,
            is_short_position_open: false,
            take_profit_price: 0.0,
            stop_loss_price: 0.0,
        }
    }
}

impl CandlestickTradingStrategy for TradeRuleStrategy {
    fn new() -> Self {
        panic!("TradeRuleStrategy should not initialized from here")
    }

    fn name(&self) -> &str {
        "trade_rule_strategy"
    }

    fn days_of_context(&self) -> u32 {
        0
    }

    fn init_trading_day(&mut self, asset: &Asset, context_candles: &[Candlestick]) {
        self.long_rule.reset();
        self.short_rule.reset();
        self.is_short_position_open = false;
        self.is_long_position_open = false;
    }

    fn candlestick_configuration(&self) -> CandlestickConfiguration {
        todo!()
    }

    fn process_closed_candlestick(&mut self, new_candle: &Candlestick) -> TradeAction {
        // do not open new trades after 17:25
        if !self.is_short_position_open && !self.is_long_position_open && new_candle.start_time >= 17_25_00 {
            TradeAction::do_nothing();
        }

        let start_long = self.long_rule.evaluate(new_candle);
        //let start_short = self.short_rule.evaluate(new_candle);

        // close long position
        if self.is_long_position_open {
            return if new_candle.start_time >= 17_25_00 ||
                self.take_profit_price <= new_candle.high ||
                self.stop_loss_price >= new_candle.low {
                self.is_long_position_open = false;
                TradeAction::send_order(OrderAction::MarketShort)
            }else{
                TradeAction::do_nothing()
            };
        }

        // close short position
        // if self.is_short_position_open {
        //     return if new_candle.start_time >= 17_25_00 ||
        //         self.take_profit_price >= new_candle.low ||
        //         self.stop_loss_price <= new_candle.high {
        //         self.is_short_position_open = false;
        //         TradeAction::send_order(OrderAction::MarketLong)
        //     }else{
        //         TradeAction::do_nothing()
        //     };
        // }

        // if we have a signal for both sides don't do anything
        // if start_long && start_short {
        //     return TradeAction::do_nothing();
        // }

        // open long position
        if start_long {
            self.is_long_position_open = true;
            self.take_profit_price = self.long_rule.take_profit_price();
            self.stop_loss_price = self.long_rule.stop_loss_price();
            return TradeAction::send_order(OrderAction::MarketLong);
        }

        // open short position
        // if start_short {
        //     self.is_short_position_open = true;
        //     self.take_profit_price = self.short_rule.take_profit_price();
        //     self.stop_loss_price = self.short_rule.stop_loss_price();
        //     return TradeAction::send_order(OrderAction::MarketLong);
        // }

        TradeAction::do_nothing()
    }
}


pub struct TradeRule {
    direction: Side,
    sma: SMA,
    rsi: RSI,
    macd: MACD,
    ema: EMA,
    take_profit_atr: ATR,
    stop_loss_atr: ATR,

    macd_target_value: f64,
    rsi_lower_bound: f64,
    rsi_higher_bound: f64,

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
        let macd_period1 = value_on_range((2.0, 100.0), chromosome[7] as f64) as usize;
        let macd_period2 = value_on_range((2.0, 100.0), chromosome[8] as f64) as usize;
        let macd_singal_period = value_on_range((2.0, 100.0), chromosome[9] as f64) as usize;
        let rsi_bound1 = value_on_range((1.0, 100.0), chromosome[5] as f64);
        let rsi_bound2 = value_on_range((1.0, 100.0), chromosome[6] as f64);

        TradeRule {
            direction,
            take_profit_target_atr: value_on_range((0.1, 20.0), chromosome[0] as f64),
            take_profit_atr: ATR::new(value_on_range((2.0, 100.0), chromosome[1] as f64) as usize).unwrap(),
            stop_loss_target_atr: value_on_range((0.1, 20.0), chromosome[2] as f64),
            stop_loss_atr: ATR::new(value_on_range((2.0, 50.0), chromosome[3] as f64) as usize).unwrap(),
            rsi: RSI::new(value_on_range((1.0, 100.0), chromosome[4] as f64) as usize).unwrap(),
            rsi_lower_bound: rsi_bound1.max(rsi_bound2),
            rsi_higher_bound: rsi_bound1.min(rsi_bound2),
            macd: MACD::new(macd_period1, macd_period2, macd_singal_period).unwrap(),
            macd_target_value: value_on_range((-1000.0, 1000.0), chromosome[10] as f64),
            ema: EMA::new(value_on_range((2.0, 100.0), chromosome[11] as f64) as usize).unwrap(),
            ema_min_percentage_diff_from_price: value_on_range((0.1, 100.0), chromosome[12] as f64),
            sma: SMA::new(value_on_range((2.0, 100.0), chromosome[13] as f64) as usize),
            sma_max_percentage_diff_from_ema: value_on_range((0.1, 100.0), chromosome[14] as f64),
            sma_take_profit: SMA::new(value_on_range((1.0, 100.0), chromosome[15] as f64) as usize),
            sma_stop_loss: SMA::new(value_on_range((1.0, 100.0), chromosome[16] as f64) as usize),
            stop_loss_atr_value: 0.0,
            take_profit_atr_value: 0.0,
        }
    }

    fn evaluate(&mut self, candle: &Candlestick) -> bool {
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
        rsi > self.rsi_higher_bound && rsi < self.rsi_lower_bound &&
        pd_ema_from_close >= self.ema_min_percentage_diff_from_price ||
        pd_sma_from_ema <= self.sma_max_percentage_diff_from_ema
    }

    fn take_profit_price(&mut self) -> f64 {
        let diff = self.take_profit_atr_value * self.take_profit_target_atr;
        if self.direction == Side::BUY {
            self.sma_take_profit.max_value_on_period() + diff
        }else{
            self.sma_take_profit.min_value_on_period() - diff
        }
    }

    fn stop_loss_price(&self) -> f64 {
        let diff = self.stop_loss_atr_value * self.stop_loss_target_atr;
        if self.direction == Side::BUY {
            self.sma_stop_loss.min_value_on_period() - diff
        }else{
            self.sma_stop_loss.max_value_on_period() + diff
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_strategy() {
        let strategy = TradeRuleStrategy::decode(vec![0.3;36].as_slice());
        assert_eq!(strategy.quantity_to_trade, 3);
        assert_eq!(strategy.long_rule.ema_min_percentage_diff_from_price.round(), 30.0);
        assert_eq!(strategy.short_rule.ema_min_percentage_diff_from_price.round(), 30.0);
    }
}