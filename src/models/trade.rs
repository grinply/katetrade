use std::convert::TryInto;

/// Trade struct to match the Java data sent from market-connector
#[derive(Clone, Debug, Copy)]
pub struct Trade {
    pub trade_number: i32,
    pub price: f64,
    pub volume: f32,
    pub date: i32,
    pub time: i32,
    pub qty: i32,
    pub buy_agent: i8,
    pub sell_agent: i8,
    pub trade_type: i8,
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

impl Trade {
    pub fn size_in_bytes() -> usize {
        return 32;
    }

    pub fn to_bytes(&self) -> &[u8] {
        return unsafe { any_as_u8_slice(self) };
    }

    /// Function to parse a byte array into a Trade struct
    pub fn from_bytes(data: &[u8]) -> Self {
        let price = f64::from_be_bytes(data[0..8].try_into().unwrap());
        let trade_number = i32::from_be_bytes(data[8..12].try_into().unwrap());
        let qty = i32::from_be_bytes(data[12..16].try_into().unwrap());
        let date = i32::from_be_bytes(data[16..20].try_into().unwrap());
        let time = i32::from_be_bytes(data[20..24].try_into().unwrap());
        let volume = f32::from_be_bytes(data[24..28].try_into().unwrap());
        let buy_agent = i8::from_be_bytes(data[28..29].try_into().unwrap());
        let sell_agent = i8::from_be_bytes(data[29..30].try_into().unwrap());
        let trade_type = i8::from_be_bytes(data[30..31].try_into().unwrap());

        Trade {
            trade_number,
            price,
            volume,
            date,
            time,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
        }
    }

    /// Parses a csv row containing a single trade. The csv row should respect the following order/format of values:
    ///
    /// **ticker, date, time, trade_number, price, qty, vol, buy_agent, sell_agent, trade_type, aft**
    ///
    ///  WINFUT,20190102,090034,10,87800.00000000,2,40000.0000,Modal,XP,Auction,N
    pub fn from_csv_row(csv_line: &mut String) -> Self {
        let mut line = csv_line.split(",");
        line.next();

        let date: i32 = line.next().unwrap().to_ascii_uppercase().parse().unwrap();
        let time: i32 = line.next().unwrap().to_ascii_uppercase().parse().unwrap();
        let trade_number: i32 = line.next().unwrap().to_ascii_uppercase().parse().unwrap();
        let price: f64 = line.next().unwrap().to_ascii_uppercase().parse().unwrap();
        let qty: i32 = line.next().unwrap().to_ascii_uppercase().parse().unwrap();
        let volume: f32 = line.next().unwrap().to_ascii_uppercase().parse().unwrap();
        // let buyer_agent = line.next().unwrap().to_ascii_uppercase();
        // let sell_agent = line.next().unwrap().to_ascii_uppercase();
        // let trade_type = line.next().unwrap().to_ascii_uppercase();
        let buy_agent: i8 = 19;
        let sell_agent: i8 = 19;
        let trade_type: i8 = 19;

        Trade {
            trade_number,
            price,
            volume,
            date,
            time,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
        }
    }

    pub fn to_csv_row(&self, ticker: &str) -> String {
        // WINFUT,20190102,090034,10,87800.00000000,2,40000.0000,Modal,XP,Auction,N
        format!(
            "{},{},{},{},{},{},{},Modal,XP,Auction,N",
            ticker.to_ascii_uppercase(),
            self.date,
            self.time,
            self.trade_number,
            self.price,
            self.qty,
            self.volume
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_struct_should_have_exactly_32_bytes_of_size() {
        const EXPECTED_SIZE: usize = 32;
        const TEST_CHECKER: [u8; EXPECTED_SIZE] = [0; std::mem::size_of::<Trade>()];
        assert_eq!(EXPECTED_SIZE, Trade::size_in_bytes());
    }
}
