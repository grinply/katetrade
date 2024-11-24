use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TickCandlestickGenerator};
use crate::models::trade::Trade;
use crate::strategies::strategy::{CandlestickTradingStrategy, HighResolutionTradingStrategy};

#[derive(Debug)]
pub struct DummyHighResolutionStrategy {
    counter: usize,
}

#[derive(Debug)]
pub struct DummyCandlestickStrategy {
    counter: usize,
}

impl HighResolutionTradingStrategy for DummyHighResolutionStrategy {
    fn new() -> Self {
        DummyHighResolutionStrategy {
            counter: 0
        }
    }

    fn name(&self) -> &str {
        "dummy_high_resolution_strategy"
    }

    fn init_trading_day(&mut self, _: &Asset, _: &Vec<Trade>) {
        // Initialize trading day logic
    }

    fn process_new_trade(&mut self, _: Trade) -> TradeAction {
        self.counter += 1;

        if self.counter == 50 {
            return TradeAction::send_order(MarketLong);
        }

        if self.counter == 100 {
            return TradeAction::send_order(MarketShort);
        }

        if self.counter > 100 {
            self.counter = 0;
        }

        TradeAction::do_nothing()
    }
}

impl CandlestickTradingStrategy for DummyCandlestickStrategy {
    fn new() -> Self {
        DummyCandlestickStrategy{
            counter: 50
        }
    }

    fn name(&self) -> &'static str {
        "dummy_candlestick_strategy"
    }

    fn init_trading_day(&mut self, _: &Asset, _: &[Candlestick]) {
        // Initialize trading day logic
    }

    fn candlestick_generator(&self) -> Box<dyn CandlestickGenerator> {
        Box::new(TickCandlestickGenerator::new(50))
    }

    fn process_closed_candlestick(&mut self, _: &Candlestick) -> TradeAction {
        self.counter += 1;

        if self.counter == 1000 {
            return TradeAction::send_order(MarketLong);
        }

        if self.counter == 3000 {
            return TradeAction::send_order(MarketShort);
        }

        if self.counter > 3000 {
            self.counter = 0;
        }

        TradeAction::do_nothing()
    }
}