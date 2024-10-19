use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::trade::Trade;
use crate::strategies::dummy_strategy::DummyStrategy;


#[allow(unused)]
/// TradingStrategy is a trait that defines the essential behavior that needs to met to allow an
/// algorithmic trading strategy to operate in the capital markets, one trading day at a time.
pub trait TradingStrategy: Send + Sync + 'static {
    fn new() -> Self
    where
        Self: Sized;

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(asset: &Asset, last_two_trading_days: &Vec<Trade>);

    /// process a new trade/datapoint that occurred in the market.
    /// the strategy can make an action to decide to buy, exit or do nothing
    fn process_new_trade(&mut self, new_trade: Trade) -> TradeAction;
}

// Generate create_strategy macro
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
        strategy_type: DummyStrategy,
    },
    // Add new strategies here
}
