use strum_macros::EnumString;
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::trade::Trade;
use crate::strategies::custom::dummy_strategy::{DummyCandlestickStrategy, DummyHighResolutionStrategy};
use crate::strategies::custom::reversal_momentum_strategy::{ClassicReversalMomentumCandlestickStrategy, ReversalMomentumCandlestickStrategy};

/// algorithmic trading strategy to operate in the capital markets using raw Trade data
#[allow(unused)]
pub trait HighResolutionTradingStrategy {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> &str;

    /// amount of data ( in days ) that will be provided by init_trading_day
    fn days_of_context(&self) -> u32 {
        1
    }

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(&mut self, asset: &Asset, context_trades: &[Trade]);

    /// process a new trade/datapoint that occurred in the market.
    /// the strategy can make an action to decide to buy, exit or do nothing
    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction;
}


/// algorithmic trading strategy to operate in the capital markets using Candlestick aggregate data
#[allow(unused)]
pub trait CandlestickTradingStrategy {
    fn new() -> Self
    where
        Self: Sized;

    fn name(&self) -> &'static str;

    /// amount of data ( in days ) that will be provided by init_trading_day
    fn days_of_context(&self) -> u32 {
        1
    }

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(&mut self, asset: &Asset, context_candles: &[Candlestick]);


    fn candlestick_generator(&self) -> Box<dyn CandlestickGenerator>;

    /// process the latest closed Candlestick.
    /// the strategy can make an action to decide to buy, exit or do nothing
    fn process_closed_candlestick(&mut self, closed_candle: &Candlestick) -> TradeAction;
}

/// Input data used in the trading strategy
#[derive(Debug, Clone, EnumString)]
pub enum StrategyInputData {
    CANDLESTICK,
    TRADE,
}


fn list_all_candlestick_strategies() -> Vec<Box<dyn CandlestickTradingStrategy>> {
    vec![
        Box::new(DummyCandlestickStrategy::new()),
        Box::new(ReversalMomentumCandlestickStrategy::new()),
        Box::new(ClassicReversalMomentumCandlestickStrategy::new())
    ]
}

pub fn list_all_high_resolution_strategies() -> Vec<Box<dyn HighResolutionTradingStrategy>> {
    vec![
        Box::new(DummyHighResolutionStrategy::new()),
    ]
}


pub fn get_candlestick_strategy_by_name(name: &str) -> Option<Box<dyn CandlestickTradingStrategy>> {
    let name = name.to_lowercase();
    for strategy in list_all_candlestick_strategies() {
        if strategy.name().to_lowercase() == name {
            return Some(strategy);
        }
    }
    None
}

pub fn get_high_resolution_strategy_by_name(name: &str) -> Option<Box<dyn HighResolutionTradingStrategy>> {
    let name = name.to_lowercase();
    for strategy in list_all_high_resolution_strategies() {
        if strategy.name().to_lowercase()  == name {
            return Some(strategy);
        }
    }
    None
}

pub fn get_candlestick_generator_for_strategy(name: &str) -> Option<Box<dyn CandlestickGenerator>> {
    let name = name.to_lowercase();
    for strategy in list_all_candlestick_strategies() {
        if strategy.name().to_lowercase() == name {
            return Some(strategy.candlestick_generator().clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_between_candlestick_and_high_resolution_strategy_should_not_conflict() {
        for candle_strategy in list_all_candlestick_strategies() {
            for high_resolution_strategy in list_all_high_resolution_strategies() {
                assert_ne!(candle_strategy.name(), high_resolution_strategy.name())
            }
        }
    }

    #[test]
    fn should_list_available_candlestick_strategies_correctly(){
        assert_eq!(3, list_all_candlestick_strategies().len());
    }
}