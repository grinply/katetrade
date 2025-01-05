use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::trade::Trade;
use crate::strategies::custom::dummy_strategy::{DummyHighResolutionStrategy};
use crate::strategies::custom::stormer_scalper_strategy::StormerScalperStrategy;

/// algorithmic trading strategy to operate in the capital markets using raw Trade data
pub trait TradingStrategy {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> &str;

    /// amount of data ( in days ) that will be provided by init_trading_day
    fn days_of_context(&self) -> u32 {
        0
    }

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(&mut self, asset: &Asset, context_trades: &[Trade]);

    /// process a new trade/datapoint that occurred in the market.
    /// the strategy can make an action to decide to buy, exit or do nothing
    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction;
}

pub fn list_all_trading_strategies() -> Vec<Box<dyn TradingStrategy>> {
    vec![
        Box::new(DummyHighResolutionStrategy::new()),
        Box::new(StormerScalperStrategy::new()),
    ]
}

pub fn get_strategy_by_name(name: &str) -> Option<Box<dyn TradingStrategy>> {
    let name = name.to_lowercase();
    for strategy in list_all_trading_strategies() {
        if strategy.name().to_lowercase()  == name {
            return Some(strategy);
        }
    }
    None
}