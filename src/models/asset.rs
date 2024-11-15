use lazy_static::lazy_static;
use crate::models::exchange::Exchange;
use crate::models::exchange::Exchange::B3;
use crate::models::market::Market;
use crate::models::market::Market::FUTURES;

#[derive(Debug)]
#[allow(unused)]
pub struct Asset {
    pub ticker: String,
    pub market: Market,
    pub exchange: Exchange,
    pub margin_per_contract: f64,
    pub min_variation: f64,
    pub value_per_variation: f64,
}

impl Asset {
    pub(crate) fn new(ticker: &str,
                      market: Market,
                      exchange: Exchange,
                      margin_per_contract: f64,
                      min_variation: f64,
                      value_per_variation: f64) -> Self {
        Asset {
            ticker: ticker.to_ascii_uppercase(),
            market,
            exchange,
            margin_per_contract,
            min_variation,
            value_per_variation,
        }
    }

    pub fn get_asset(ticker: &str, market: Market, exchange: Exchange) -> Option<&Asset>{
        let ticker = ticker.to_uppercase();
        for asset in AVAILABLE_ASSETS.iter() {
            if asset.ticker == ticker && asset.market == market && asset.exchange == exchange {
                return Some(asset);
            }
        }
        None
    }
}



lazy_static! {
    static ref AVAILABLE_ASSETS: Vec<Asset> = vec![
        Asset::new("WIN", FUTURES, B3, 200.0 ,5.0, 0.2),
    ];
}