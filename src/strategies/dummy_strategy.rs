use crate::models::action::OrderAction::{MarketLong, MarketShort};
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::trade::Trade;
use crate::strategies::strategy::TradingStrategy;

#[derive(Debug)]
pub struct DummyStrategy {
    counter: usize,
}

impl TradingStrategy for DummyStrategy {
    fn new() -> Self {
        DummyStrategy{
            counter: 0
        }
    }

    fn init_trading_day(_: &Asset, _: &Vec<Trade>) {
        // Initialize trading day logic
    }

    fn process_new_trade(&mut self, new_trade: Trade) -> TradeAction {
        println!("new trade processed by dummy strategy: {:?}", new_trade);
        self.counter+=1;
        println!("internal state is {}", self.counter);

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