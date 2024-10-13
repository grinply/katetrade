use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::trade::Trade;

/// TradingStrategy is a trait that defines the essential behavior that needs to met to allow an
/// algorithmic trading strategy to operate in the capital markets, one trading day at a time.
trait TradingStrategy {
    /// unique identifier for the trading strategy
    fn identifier(&self) -> String;

    /// starts the trading day providing the context necessary to know what is going to be traded
    fn init_trading_day(asset: &Asset, last_two_trading_days: &Vec<Trade>);

    /// process a new trade/datapoint that occurred in the market.
    /// the strategy can make a action to decide to buy, exit or do nothing
    fn process_new_trade(&self, new_trade: Trade) -> TradeAction;
}
