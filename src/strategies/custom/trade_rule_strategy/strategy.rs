use std::f32;

use crate::constants::CHROMOSOME_SIZE_TRADE_RULE;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::side::Side;
use crate::strategies::custom::trade_rule_strategy::trade_rule::{TradeRule};
use crate::strategies::strategy::{CandlestickTradingStrategy};
use crate::utils::date::is_time_after;
use crate::utils::math::value_on_range;

/// this is strategy is based upon the papers:
/// Trade Optimizer and Optimization of the Trading Rule in Foreign Exchange using Genetic Algorithm
pub struct TradeRuleStrategy {
    quantity_to_trade: u32,
    long_rule: TradeRule,
    short_rule: TradeRule,
    is_long_position_open: bool,
    is_short_position_open: bool,
    stop_loss_price: f64,
    take_profit_price: f64,
}

impl TradeRuleStrategy {
    pub fn is_valid(&self) -> bool{
        self.long_rule.is_valid() && self.short_rule.is_valid()
    }

    pub fn decode(chromosome: &[f32]) -> Self {
        assert_eq!(CHROMOSOME_SIZE_TRADE_RULE, chromosome.len(), "the chromosome must have length {}, but it has {}", CHROMOSOME_SIZE_TRADE_RULE, chromosome.len());

        for i in 0..chromosome.len() {
            if chromosome[i] < 0.0 || chromosome[i] > 1.0 {
                panic!("the chromosome must have genes between 0.0 and 1.0, but it had {}", chromosome[i]);
            }
        }

        // you may want give control to the strategy for how many contracts it wants to trade
        let quantity_to_trade = value_on_range((1.0, 20.0), chromosome[0] as f64) as u32;

        TradeRuleStrategy {
            quantity_to_trade,
            long_rule: TradeRule::new(Side::BUY, &chromosome[1..=18]),
            short_rule: TradeRule::new(Side::SELL, &chromosome[18..=36]),
            is_long_position_open: false,
            is_short_position_open: false,
            take_profit_price: 0.0,
            stop_loss_price: 0.0,
        }
    }

    pub fn quantity_to_trade(&self) -> u32 {
        self.quantity_to_trade
    }
}

impl CandlestickTradingStrategy for TradeRuleStrategy {
    fn new() -> Self {
        panic!("TradeRuleStrategy should not initialized from here")
    }

    fn name(&self) -> &'static str {
        "trade_rule_strategy"
    }

    fn days_of_context(&self) -> u32 {
        1
    }

    fn init_trading_day(&mut self, _: &Asset, candles: &[Candlestick]) {
        self.long_rule.reset();
        self.short_rule.reset();
        self.is_short_position_open = false;
        self.is_long_position_open = false;

        for candle in candles {
            self.long_rule.evaluate(candle);
            self.short_rule.evaluate(candle);
        }
    }

    fn candlestick_generator(&self) -> Box<dyn CandlestickGenerator> {
        Box::new(TimeCandlestickGenerator::new(TimeUnit::Minutes, 1))
    }

    fn process_closed_candlestick(&mut self, new_candle: &Candlestick) -> TradeAction {
        // evaluate() will internally update indicators with the new candle, should always run first
        let start_long = self.long_rule.evaluate(new_candle);
        let start_short = self.short_rule.evaluate(new_candle);

        if !self.is_short_position_open &&
            !self.is_long_position_open &&
            is_time_after(new_candle.start_time, 17, 25, 0) {
            TradeAction::do_nothing();
        }

        // close long position
        if self.is_long_position_open {
            return if is_time_after(new_candle.start_time, 17, 25, 0) ||
                self.take_profit_price <= new_candle.high ||
                self.stop_loss_price >= new_candle.low {
                self.is_long_position_open = false;
                TradeAction::send_order(OrderAction::MarketShort)
            } else {
                TradeAction::do_nothing()
            };
        }

        // close short position
        if self.is_short_position_open {
            return if is_time_after(new_candle.start_time, 17, 25, 0)  ||
                self.take_profit_price >= new_candle.low ||
                self.stop_loss_price <= new_candle.high {
                self.is_short_position_open = false;
                TradeAction::send_order(OrderAction::MarketLong)
            } else {
                TradeAction::do_nothing()
            };
        }

        // if we have a signal for both sides don't do anything
        if start_long && start_short {
            return TradeAction::do_nothing();
        }

        // cant send a new order if an existing position is open
        // increasing position size is not allowed
        if self.is_long_position_open || self.is_short_position_open {
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
            return TradeAction::send_order(OrderAction::MarketShort);
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
        TradeRuleStrategy::decode(vec![0.3;CHROMOSOME_SIZE_TRADE_RULE].as_slice());
    }
}
