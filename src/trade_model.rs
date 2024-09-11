use std::convert::TryInto;

/// Trade struct to match the Java data sent from market-connector
#[derive(Debug)]
pub struct Trade {
    pub trade_number: i32,
    pub price: f64,
    pub volume: f64,
    pub timestamp_millis: i64,
    pub qty: i32,
    pub buy_agent: i32,
    pub sell_agent: i32,
    pub trade_type: i32,
    pub edit_trade: bool,
}

impl Trade {
    /// Function to parse a byte array into a Trade struct
    pub fn from_bytes(data: &[u8]) -> Self {
        let trade_number = i32::from_be_bytes(data[0..4].try_into().unwrap());
        let qty = i32::from_be_bytes(data[4..8].try_into().unwrap());
        let buy_agent = i32::from_be_bytes(data[8..12].try_into().unwrap());
        let sell_agent = i32::from_be_bytes(data[12..16].try_into().unwrap());
        let trade_type = i32::from_be_bytes(data[16..20].try_into().unwrap());
        let timestamp_millis = i64::from_be_bytes(data[20..28].try_into().unwrap());
        let price = f64::from_be_bytes(data[28..36].try_into().unwrap());
        let volume = f64::from_be_bytes(data[36..44].try_into().unwrap());
        let edit_trade = data[44] != 0;

        Trade {
            trade_number,
            price,
            volume,
            timestamp_millis,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
            edit_trade,
        }
    }
}
