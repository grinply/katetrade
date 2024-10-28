use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickType};
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

    fn init_trading_day(_: &Asset, _: &Vec<Trade>) {
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
    fn new() -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn init_trading_day(_: &Asset, _: &Vec<Candlestick>) {
        // Initialize trading day logic
    }

    fn candlestick_type(&self) -> CandlestickType {
        CandlestickType::TICK
    }

    fn candlestick_configuration(&self) -> u64 {
        50
    }

    fn process_closed_candlestick(&mut self, _: Candlestick) -> TradeAction {
        self.counter += 1;

        if self.counter == 10 {
            return TradeAction::send_order(MarketLong);
        }

        if self.counter == 20 {
            return TradeAction::send_order(MarketShort);
        }

        if self.counter > 20 {
            self.counter = 0;
        }

        TradeAction::do_nothing()
    }
}