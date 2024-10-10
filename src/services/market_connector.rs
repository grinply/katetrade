use std::io::{self, ErrorKind};
use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::side::Side;

pub static MARKET_CONNECTOR_HOST: &str = "market-connector.grinply.com";

struct MarketConnectorClient {
    host: String,
    exchange: Exchange,
}

impl MarketConnectorClient {
    fn send_market_order(&self,
                         ticker: &str,
                         market: Market,
                         side: Side,
                         quantity: u64,
                         account_id: &str,
                         strategy: &str) -> io::Result<i64> {
        let exchange_str = self.exchange.to_string();
        let url = format!("{MARKET_CONNECTOR_HOST}/api/{exchange_str}/order/create");
        let response = ureq::post(url.as_str())
            .send_json(ureq::json!({
                "orderType": "MARKET",
                "category": market.to_string(),
                "ticker": ticker,
                "quantity": quantity,
                "side": side.to_string(),
                "accountId": account_id,
                "strategyId": strategy
            }));

        if response.is_err() {
            return Err(io::Error::new(ErrorKind::InvalidData, response.err().unwrap()));
        }

        let response_value = response.unwrap();

        if response_value.status() != 200 {
            return Err(io::Error::new(ErrorKind::Other, "failed to send market order"));
        }

        Ok(response_value.into_string()?.parse::<i64>().unwrap())
    }

    fn send_limit_order(&self,
                        ticker: &str,
                        market: Market,
                        side: Side,
                        price: f64,
                        quantity: u64,
                        account_id: &str,
                        strategy: &str,
    ) -> io::Result<i64> {
        let exchange_str = self.exchange.to_string();
        let url = format!("{MARKET_CONNECTOR_HOST}/api/{exchange_str}/order/create");
        let response = ureq::post(url.as_str())
            .send_json(ureq::json!({
                "orderType": "LIMIT",
                "category": market.to_string(),
                "ticker": ticker,
                "quantity": quantity,
                "side": side.to_string(),
                "price": price,
                "accountId": account_id,
                "strategyId": strategy
            }));

        if response.is_err() {
            return Err(io::Error::new(ErrorKind::InvalidData, response.err().unwrap()));
        }

        let response_value = response.unwrap();

        if response_value.status() != 200 {
            return Err(io::Error::new(ErrorKind::Other, "failed to send limit order"));
        }

        Ok(response_value.into_string()?.parse::<i64>().unwrap())
    }

    pub fn get_trade_feed_key(&self, ticker: String, market: Market) -> io::Result<String> {
        let exchange_str = self.exchange.to_string();
        let endpoint_path = format!("/api/{exchange_str}/market/feed/trades/{market}/{ticker}/subscribe");
        let url = format!("{}/{}", MARKET_CONNECTOR_HOST, endpoint_path);

        let response = ureq::get(url.as_str())
            .call();

        if response.is_err() {
            return Err(io::Error::new(ErrorKind::InvalidData, response.err().unwrap()));
        }

        let response_value = response.unwrap();

        if response_value.status() != 200 {
            return Err(io::Error::new(ErrorKind::Other, "failed to send limit order"));
        }

        Ok(response_value.into_string()?)
    }
}









