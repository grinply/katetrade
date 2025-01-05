use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::trade::Trade;
use crate::strategies::strategy::{TradingStrategy};

#[derive(Debug)]
pub struct DummyHighResolutionStrategy {
    counter: usize,
}

impl TradingStrategy for DummyHighResolutionStrategy {
    fn new() -> Self {
        DummyHighResolutionStrategy {
            counter: 0
        }
    }

    fn name(&self) -> &str {
        "dummy_high_resolution_strategy"
    }

    fn init_trading_day(&mut self, _: &Asset, _: &[Trade]) {
        // Initialize trading day logic
    }

    fn process_new_trade(&mut self, _: &Trade) -> TradeAction {
        self.counter += 1;

        if self.counter == 50 {
            return TradeAction::send_order(MarketLong(12));
        }

        if self.counter == 100 {
            return TradeAction::send_order(MarketShort(12));
        }

        if self.counter > 100 {
            self.counter = 0;
        }

        TradeAction::do_nothing()
    }
}