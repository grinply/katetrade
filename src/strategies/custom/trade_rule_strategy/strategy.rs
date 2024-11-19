use std::f32;

use ta::{Next, Reset};
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::Candlestick;
use crate::models::side::Side;
use crate::strategies::custom::trade_rule_strategy::trade_rule::{TradeRule, RULE_CHROMOSOME_SIZE};
use crate::strategies::strategy::{CandlestickConfiguration, CandlestickTradingStrategy};

pub struct TradeRuleStrategy {
    long_rule: TradeRule,
    short_rule: TradeRule,
    is_long_position_open: bool,
    is_short_position_open: bool,
    stop_loss_price: f64,
    take_profit_price: f64
}

impl TradeRuleStrategy {
    pub fn is_valid(&self) -> bool{
        self.long_rule.is_valid() && self.short_rule.is_valid()
    }

    pub fn decode(chromosome: &[f32]) -> Self {
        assert_eq!(35, chromosome.len(), "the chromosome must have length 35, but it has {}", chromosome.len());

        for i in 0..chromosome.len() {
            if chromosome[i] < 0.0 || chromosome[i] > 1.0 {
                panic!("the chromosome must have genes between 0.0 and 1.0, but it had {}", chromosome[i]);
            }
        }

        TradeRuleStrategy {
            long_rule: TradeRule::new(Side::BUY, &chromosome[..=RULE_CHROMOSOME_SIZE]),
            short_rule: TradeRule::new(Side::SELL, &chromosome.iter().skip(RULE_CHROMOSOME_SIZE)[..=RULE_CHROMOSOME_SIZE]),
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

    fn init_trading_day(&mut self, _: &Asset, _: &[Candlestick]) {
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
        let start_short = self.short_rule.evaluate(new_candle);

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
        if self.is_short_position_open {
            return if new_candle.start_time >= 17_25_00 ||
                self.take_profit_price >= new_candle.low ||
                self.stop_loss_price <= new_candle.high {
                self.is_short_position_open = false;
                TradeAction::send_order(OrderAction::MarketLong)
            }else{
                TradeAction::do_nothing()
            };
        }

        // if we have a signal for both sides don't do anything
        if start_long && start_short {
            return TradeAction::do_nothing();
        }

        // open long position
        if start_long {
            self.is_long_position_open = true;
            self.take_profit_price = self.long_rule.take_profit_price();
            self.stop_loss_price = self.long_rule.stop_loss_price();
            return TradeAction::send_order(OrderAction::MarketLong);
        }

        // open short position
        if start_short {
            self.is_short_position_open = true;
            self.take_profit_price = self.short_rule.take_profit_price();
            self.stop_loss_price = self.short_rule.stop_loss_price();
            return TradeAction::send_order(OrderAction::MarketLong);
        }

        TradeAction::do_nothing()
    }
}

#[cfg(test)]
mod tests {
    use crate::strategies::custom::trade_rule_strategy::strategy::TradeRuleStrategy;
    use super::*;

    #[test]
    fn test_decode_single_strategy() {
        let strategy = TradeRuleStrategy::decode(vec![0.3;35].as_slice());
        assert_eq!(strategy.long_rule.ema_min_percentage_diff_from_price.round(), 30.0);
        assert_eq!(strategy.short_rule.ema_min_percentage_diff_from_price.round(), 30.0);
    }
}
