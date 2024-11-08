use crate::models::side::Side;

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct Position {
    pub side: Side,
    pub avg_entry_price: f64,
    pub avg_exit_price: f64,
    pub date: i32,
    pub entry_time: i32,
    pub exit_time: i32,
    pub start_datapoint_number: i32,
    pub end_datapoint_number: i32,
    pub qty: i32,
    pub gross_pnl: f64,
    pub liquid_pnl: f64,
    pub total_paid_in_fees: f64,
    pub quantity_of_orders: u32,
    pub is_closed: bool,
}

#[allow(unused)]
pub struct TradePnl {
    pub result: f64,
}