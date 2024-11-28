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
    pub min_point_variation: f64,
    pub value_per_point: f64,
    pub margin_per_contract: f64,
}

impl Asset {
    fn new(ticker: &str,
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
            min_point_variation: min_variation,
            value_per_point: value_per_variation,
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
    // https://investimentos.btgpactual.com/renda-variavel/day-trade/lista-de-ativos
    static ref AVAILABLE_ASSETS: Vec<Asset> = vec![
        Asset::new("WIN", FUTURES, B3, 100.0, 5.0, 0.2),
        Asset::new("IND", FUTURES, B3, 500.0, 5.0, 1.0),
        Asset::new("WDO", FUTURES, B3, 150.0, 0.5, 10.0),
        Asset::new("DOL", FUTURES, B3, 750.0, 0.5, 50.0),
        Asset::new("BIT", FUTURES, B3, 100.0, 20.0, 0.1),
    ];
}