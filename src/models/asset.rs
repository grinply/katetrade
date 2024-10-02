use crate::models::exchange::Exchange;
use crate::models::market::Market;
#[derive(Debug)]
pub struct Asset {
    ticker: String,
    market: Market,
    exchange: Exchange,
    min_variation: f64,
    value_per_variation: f64,
}


