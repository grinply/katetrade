use std::io::{self, ErrorKind};
use std::thread;
use std::time::Duration;
use ureq::Response;
use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::side::Side;


pub struct MarketConnectorClient<'a> {
    host: &'a str,
    exchange: Exchange,
}

impl<'a> MarketConnectorClient<'a> {
    pub fn new(host: &'a str, exchange: Exchange) -> Self {
        Self {
            host,
            exchange,
        }
    }
    pub fn send_market_order(&self,
                             ticker: &str,
                             market: Market,
                             side: Side,
                             quantity: u64,
                             account_id: &str,
                             strategy: &str) -> io::Result<i64> {
        let exchange_str = self.exchange.to_string();
        let url = format!("{}/api/{exchange_str}/order/create", self.host);
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

    pub fn send_limit_order(&self,
                            ticker: &str,
                            market: Market,
                            side: Side,
                            price: f64,
                            quantity: u64,
                            account_id: &str,
                            strategy: &str,
    ) -> io::Result<i64> {
        let exchange_str = self.exchange.to_string();
        let url = format!("{}/api/{exchange_str}/order/create", self.host);
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
        let url = format!("{}/{}", self.host, endpoint_path);

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

    pub fn health_status(&self) -> io::Result<bool> {
        let url = format!("{}/actuator/health", self.host);
        let thread_handle = thread::spawn(move || {
            return ureq::get(url.as_str())
                .timeout(Duration::from_secs(2))
                .call();
        });

        let mut current_wait = 0;
        let max_wait = 2500;

        while current_wait < max_wait && !thread_handle.is_finished() {
            thread::sleep(Duration::from_millis(100));
            current_wait+=100;
        }

        if !thread_handle.is_finished() {
            return Err(io::Error::new(ErrorKind::Other, "timeout, the server is unresponsive"));
        }

        let response = thread_handle.join().unwrap();

        if response.is_err() {
            return Err(io::Error::new(ErrorKind::InvalidData, response.err().unwrap()));
        }
        let content = response.unwrap().into_string()?;
        Ok(content.contains("UP"))
    }
}









