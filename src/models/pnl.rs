pub struct DetailedTradePnl {
    pub avg_entry_price: f64,
    pub avg_exit_price: f64,
    pub date: i32,
    pub start_trade_number: i32,
    pub end_trade_number: i32,
    pub qty: i32,
    pub entry_time: i32,
    pub exit_time: i32,
    pub result: f64,
}

pub struct TradePnl {
    pub result: f64,
}
