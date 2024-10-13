#[derive(Debug)]
pub enum OrderAction {
    MarketLong,
    MarketShort,
    LimitLong(f64),
    LimitShort(f64),
}

#[derive(Debug)]
pub struct TradeAction {
    enter_market_action: Option<OrderAction>,
    exit_market_action: Option<OrderAction>,
}
