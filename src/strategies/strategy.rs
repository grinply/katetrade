use strum_macros::{Display, EnumString, IntoStaticStr};
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::candlestick::{CandlestickType, Candlestick};
use crate::models::trade::Trade;
use crate::strategies::dummy_strategy::{DummyCandlestickStrategy, DummyHighResolutionStrategy};


#[allow(unused)]
/// algorithmic trading strategy to operate in the capital markets using raw Trade data
pub trait HighResolutionTradingStrategy: Send + Sync + 'static {
    fn new() -> Self
    where
        Self: Sized;

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(asset: &Asset, last_two_trading_days: &Vec<Trade>);

    /// process a new trade/datapoint that occurred in the market.
    /// the strategy can make an action to decide to buy, exit or do nothing
    fn process_new_trade(&mut self, new_trade: Trade) -> TradeAction;
}

#[allow(unused)]
/// algorithmic trading strategy to operate in the capital markets using Candlestick aggregate data
pub trait CandlestickTradingStrategy: Send + Sync + 'static {
    fn new() -> Self
    where
        Self: Sized;

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(asset: &Asset, last_two_trading_days: &Vec<Candlestick>);

    /// type of candlestick used in this strategy
    fn candlestick_type(&self) -> CandlestickType;

    /// the input parameter used for type of candlestick select
    ///
    /// for TICK: number of trades | for TIME: minutes | for VOLUME: total volume
    fn candlestick_configuration(&self) -> u64;

    /// process the latest closed Candlestick.
    /// the strategy can make an action to decide to buy, exit or do nothing
    fn process_closed_candlestick(&mut self, closed_candle: Candlestick) -> TradeAction;
}

/// Input data used in the trading strategy
#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoStaticStr, Display, EnumString)]
pub enum StrategyInputData {
    CANDLESTICK,
    TRADE,
}


/// macro for HighResolutionTradingStrategy
#[macro_export]
macro_rules! define_algorithms {
    (
        $(
            $variant:ident => {
                id: $id:expr,
                strategy_type: $strategy_type:ty,
            },
        )*
    ) => {
        // Define the AlgorithmicStrategy enum
        pub enum AlgorithmicStrategy {
            $(
                $variant($strategy_type),
            )*
        }

        // Implement the create_strategy function
        pub fn create_strategy(id: &str) -> Option<AlgorithmicStrategy> {
            match id {
                $(
                    $id => Some(AlgorithmicStrategy::$variant(<$strategy_type>::new())),
                )*
                _ => None,
            }
        }
    }
}

#[macro_export]
macro_rules! strategy_dispatch {
     // Immutable case
     ($strategy:expr, $method:ident $(, $args:expr)*) => {
        match $strategy {
            $crate::strategies::strategy::AlgorithmicStrategy::Dummy(ref mut s) => s.$method($($args),*),
        }
    };

    ($strategy:expr, $method:ident $(, $($args:tt)+)?) => {
        match $strategy {
            $(
                $crate::strategies::strategy::AlgorithmicStrategy::Dummy(ref s) => s.$method($($($args)+)?),
            )*
        }
    };
}


#[macro_export]
macro_rules! strategy_init_trading_day {
    ($strategy:expr, $asset:expr, $last_two_trading_days:expr) => {
        strategy_dispatch!($strategy, init_trading_day, $asset, $last_two_trading_days)
    };
}

#[macro_export]
macro_rules! strategy_process_new_trade {
    ($strategy:expr, $new_trade:expr) => {
        strategy_dispatch!($strategy, process_new_trade, $new_trade)
    };
}

define_algorithms! {
    Dummy => {
        id: "dummy",
        strategy_type: DummyHighResolutionStrategy,
    },
    // Add new strategies here
}

/// macros for the CandlestickTradingStrategy
#[macro_export]
macro_rules! define_candlestick_algorithms {
    (
        $(
            $variant:ident => {
                id: $id:expr,
                strategy_type: $strategy_type:ty,
            },
        )*
    ) => {
        // Define the CandlestickAlgorithmicStrategy enum
        pub enum CandlestickAlgorithmicStrategy {
            $(
                $variant($strategy_type),
            )*
        }

        // Implement the create_candlestick_strategy function
        pub fn create_candlestick_strategy(id: &str) -> Option<CandlestickAlgorithmicStrategy> {
            match id {
                $(
                    $id => Some(CandlestickAlgorithmicStrategy::$variant(<$strategy_type>::new())),
                )*
                _ => None,
            }
        }
    }
}

#[macro_export]
macro_rules! candlestick_strategy_dispatch {
    ($strategy:expr, $method:ident $(, $args:expr)*) => {
        match $strategy {
            CandlestickAlgorithmicStrategy::DummyCandlestick(ref mut s) => s.$method($($args),*),
            // Add other variants here as you define them
        }
    };
}

#[macro_export]
macro_rules! candlestick_strategy_init_trading_day {
    ($strategy:expr, $asset:expr, $last_two_trading_days:expr) => {
        candlestick_strategy_dispatch!($strategy, init_trading_day, $asset, $last_two_trading_days)
    };
}

#[macro_export]
macro_rules! strategy_process_closed_candlestick {
    ($strategy:expr, $closed_candle:expr) => {
        candlestick_strategy_dispatch!($strategy, process_closed_candlestick, $closed_candle)
    };
}

#[macro_export]
macro_rules! strategy_candlestick_type{
    ($strategy:expr) => {
        candlestick_strategy_dispatch!($strategy, candlestick_type)
    };
}

#[macro_export]
macro_rules! strategy_candlestick_configuration{
    ($strategy:expr) => {
        candlestick_strategy_dispatch!($strategy, candlestick_configuration)
    };
}

define_candlestick_algorithms! {
    DummyCandlestick => {
        id: "dummy",
        strategy_type: DummyCandlestickStrategy,
    },
    // Add new strategies here
}

