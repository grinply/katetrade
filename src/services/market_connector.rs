use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::side::Side;
use std::thread;
use std::time::Duration;
use crate::models::order::Order;
use crate::models::order_status::OrderStatus;

pub struct MarketConnectorClient<'a> {
    host: &'a str,
    #[allow(unused)]
    exchange: Exchange,
}

#[allow(unused)]
impl<'a> MarketConnectorClient<'a> {
    pub fn new(host: &'a str, exchange: Exchange) -> Self {
        Self { host, exchange }
    }

    pub fn get_order_details(&self, account_id: i64, order_id: i64) -> Result<Order, anyhow::Error>{
        println!("fetching order details for {} and order_id {}", account_id, order_id);
        let order = Order{
            account_id: "",
            session_id: 0,
            exchange: Exchange::B3,
            ticker: "WIN",
            status: OrderStatus::SentToExchange,
            cl_ord_id: "",
            requested_price: 0.0,
            execution_price: 0.0,
            qty_send: 0,
            qty_executed: 0,
            executed_date: "",
            executed_time: 0,
            side: Side::BUY,
            market_type: Market::SPOT,
            strategy_id: "",
        };
        Ok(order)
    }

    pub fn send_market_order(
        &self,
        ticker: &str,
        market: Market,
        side: Side,
        quantity: i32,
        account_id: &str,
        strategy: &str,
    ) -> Result<i64, anyhow::Error> {
        let exchange_str = self.exchange.to_string();
        let url = format!("{}/api/{exchange_str}/order/create", self.host);
        let response = ureq::post(url.as_str()).send_json(ureq::json!({
            "orderType": "MARKET",
            "category": market.to_string(),
            "ticker": ticker,
            "quantity": quantity,
            "side": side.to_string(),
            "accountId": account_id,
            "strategyId": strategy
        }))?;

        if response.status() != 200 {
            return anyhow::bail!("failed to send market order");
        }

        Ok(response.into_string()?.parse::<i64>()?)
    }

    pub fn send_limit_order(
        &self,
        ticker: &str,
        market: Market,
        side: Side,
        price: f64,
        quantity: i32,
        account_id: &str,
        strategy: &str,
    ) -> Result<i64, anyhow::Error> {
        let exchange_str = self.exchange.to_string();
        let url = format!("{}/api/{exchange_str}/order/create", self.host);
        let response = ureq::post(url.as_str()).send_json(ureq::json!({
            "orderType": "LIMIT",
            "category": market.to_string(),
            "ticker": ticker,
            "quantity": quantity,
            "side": side.to_string(),
            "price": price,
            "accountId": account_id,
            "strategyId": strategy
        }))?;

        if response.status() != 200 {
            return anyhow::bail!("failed to send limit order");
        }
        Ok(response.into_string()?.parse::<i64>()?)
    }

    pub fn get_trade_feed_key(&self, ticker: String, market: Market) -> Result<String, anyhow::Error> {
        let exchange_str = self.exchange.to_string();
        let endpoint_path =
            format!("/api/{exchange_str}/market/feed/trades/{market}/{ticker}/subscribe");
        let url = format!("{}/{}", self.host, endpoint_path);

        let response = ureq::get(url.as_str()).call()?;

        if response.status() != 200 {
            anyhow::bail!("failed to send limit order");
        }
        Ok(response.into_string()?)
    }

    pub fn health_status(&self) -> Result<bool, anyhow::Error> {
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
            current_wait += 100;
        }

        if !thread_handle.is_finished() {
            return anyhow::bail!("timeout, the server is unresponsive");
        }
        let response = thread_handle.join().unwrap();
        Ok(response?.into_string()?.contains("UP"))
    }
}
