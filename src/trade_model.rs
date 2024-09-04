use std::convert::TryInto;

/// Trade struct to match the Java data sent from market-connector
#[derive(Debug)]
pub struct Trade {
    pub price: f64,
    pub volume: f64,
    pub timestamp_millis: i64,
    pub qty: i32,
    pub buy_agent: i32,
    pub sell_agent: i32,
    pub trade_type: i32,
    pub edit_trade: bool,
    pub asset_ticker: String,
}

impl Trade {
    /// Function to parse a byte array into a Trade struct
    pub fn from_bytes(data: &[u8]) -> Self {
        let price = f64::from_be_bytes(data[0..8].try_into().unwrap());
        let volume = f64::from_be_bytes(data[8..16].try_into().unwrap());
        let timestamp_millis = i64::from_be_bytes(data[16..24].try_into().unwrap());
        let qty = i32::from_be_bytes(data[24..28].try_into().unwrap());
        let buy_agent = i32::from_be_bytes(data[28..32].try_into().unwrap());
        let sell_agent = i32::from_be_bytes(data[32..36].try_into().unwrap());
        let trade_type = i32::from_be_bytes(data[36..40].try_into().unwrap());
        let edit_trade = data[40] != 0;
        let asset_ticker = String::from_utf8_lossy(&data[41..]).to_string();

        Trade {
            price,
            volume,
            timestamp_millis,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
            edit_trade,
            asset_ticker,
        }
    }
}
