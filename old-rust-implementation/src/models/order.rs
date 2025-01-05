use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::order_status::OrderStatus;
use crate::models::side::Side;

#[derive(Debug)]
#[allow(unused)]
pub struct Order<'a> {
    pub account_id: &'a str,
    pub session_id: i64,
    pub exchange: Exchange,
    pub market_type: Market,
    pub side: Side,
    pub ticker: &'a str,
    pub status: OrderStatus,
    pub cl_ord_id: &'a str,
    pub requested_price: f64,
    pub execution_price: f64,
    pub qty_send: i32,
    pub qty_executed: i32,
    pub executed_date: &'a str,
    pub executed_time: i32, // in the format: HHmmssSSS
    pub strategy_id: &'a str,
}

