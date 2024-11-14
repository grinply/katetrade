use std::collections::HashMap;
use std::convert::TryInto;
use anyhow;
use anyhow::bail;
use std::fs::{File};
use std::io::{self, BufRead, BufReader, Read, Write};
use lazy_static::lazy_static;
use crate::constants;
const TRADES_CSV_HEADER_LINE: &str = "<ticker>,<date>,<time>,<trade_number>,<price>,<qty>,<vol>,<buy_agent>,<sell_agent>,<trade_type>,<aft>";

/// Trade struct to match the Java data sent from market-connector
#[derive(Debug, PartialEq)]
#[allow(unused)]
pub struct Trade {
    pub trade_number: i32,
    pub date: i32,
    pub time: i32,
    pub qty: i32,
    pub buy_agent: i32,
    pub sell_agent: i32,
    pub trade_type: i32,
    pub price: f64,
    pub volume: f64,
}
pub struct TradeFile {
    reader: BufReader<File>,
    trades_read: usize,
    total_trades: usize,
}

impl TradeFile {
    pub fn new_from_binary_file(full_file_path: &str) -> Result<Self, anyhow::Error> {
        let file = File::open(full_file_path)?;
        file.sync_all()?;
        let file_size_bytes = file.metadata()?.len();

        Ok(
            TradeFile {
                reader: BufReader::new(file),
                total_trades: (file_size_bytes / Trade::size_in_bytes() as u64) as usize,
                trades_read: 0,
            }
        )
    }
}

impl Iterator for TradeFile {
    type Item = Trade;

    fn next(&mut self) -> Option<Trade> {
        if self.trades_read >= self.total_trades {
            return None;
        }
        let mut buffer: [u8; Trade::size_in_bytes()] = [0; Trade::size_in_bytes()];
        if self.reader.read_exact(&mut buffer).is_err() {
            return None;
        }
        let current_trade = Trade::from_bytes(buffer.as_slice()).unwrap();
        self.trades_read += 1;
        Some(current_trade)
    }
}


impl Trade {
    pub const fn size_in_bytes() -> usize {
        44
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut buffer: Vec<u8> = vec![];
        buffer.write_all(&self.trade_number.to_be_bytes())?;
        buffer.write_all(&self.qty.to_be_bytes())?;
        buffer.write_all(&self.buy_agent.to_be_bytes())?;
        buffer.write_all(&self.sell_agent.to_be_bytes())?;
        buffer.write_all(&self.trade_type.to_be_bytes())?;
        buffer.write_all(&self.date.to_be_bytes())?;
        buffer.write_all(&self.time.to_be_bytes())?;
        buffer.write_all(&self.price.to_be_bytes())?;
        buffer.write_all(&self.volume.to_be_bytes())?;
        Ok(buffer)
    }

    /// Function to parse a byte array into a Trade struct
    pub fn from_bytes(data: &[u8]) -> Result<Self, anyhow::Error> {
        if data.len() != Self::size_in_bytes() {
            anyhow::bail!(format!("the required size for a trade is {} bytes, received {}",
                                              Trade::size_in_bytes(), data.len()));
        }
        let trade_number = i32::from_be_bytes(data[0..4].try_into()?);
        let qty = i32::from_be_bytes(data[4..8].try_into()?);
        let buy_agent = i32::from_be_bytes(data[8..12].try_into()?);
        let sell_agent = i32::from_be_bytes(data[12..16].try_into()?);
        let trade_type = i32::from_be_bytes(data[16..20].try_into()?);
        let date = i32::from_be_bytes(data[20..24].try_into()?);
        let time = i32::from_be_bytes(data[24..28].try_into()?);
        let price = f64::from_be_bytes(data[28..36].try_into()?);
        let volume = f64::from_be_bytes(data[36..44].try_into()?);

        Ok(Trade {
            trade_number,
            price,
            volume,
            date,
            time,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
        })
    }

    /// Parses a csv row containing a single trade. The csv row should respect the following order/format of values:
    ///
    /// **ticker, date, time, trade_number, price, qty, vol, buy_agent, sell_agent, trade_type, aft**
    ///
    ///  WINFUT,20190102,090034,10,87800.00000000,2,40000.0000,Modal,XP,Auction,N
    pub fn from_csv_row(csv_line: &mut String) -> Result<Self, anyhow::Error> {
        let mut line = csv_line.split(",");
        line.next();

        let date: i32 = line.next().unwrap().parse()?;
        let time: i32 = line.next().unwrap().parse()?;
        let trade_number: i32 = line.next().unwrap().parse()?;
        let price: f64 = line.next().unwrap().parse()?;
        let qty: i32 = line.next().unwrap().parse()?;
        let volume: f64 = line.next().unwrap().parse()?;
        let buyer_agent_name = line.next().unwrap().to_ascii_uppercase();
        let sell_agent_name = line.next().unwrap().to_ascii_uppercase();
        let trade_type_name = line.next().unwrap().to_ascii_uppercase();

        let buy_agent = if AGENT_MAP.contains_key(buyer_agent_name.as_str()) {
            AGENT_MAP.get(buyer_agent_name.as_str()).unwrap()
        } else {
            AGENT_MAP.get(constants::UNKNOWN).unwrap()
        };

        let sell_agent = if AGENT_MAP.contains_key(sell_agent_name.as_str()) {
            AGENT_MAP.get(sell_agent_name.as_str()).unwrap()
        } else {
            AGENT_MAP.get(constants::UNKNOWN).unwrap()
        };

        let trade_type = if TRADE_TYPES_MAP.contains_key(trade_type_name.as_str()) {
            TRADE_TYPES_MAP.get(trade_type_name.as_str()).unwrap()
        } else {
            TRADE_TYPES_MAP.get(constants::UNKNOWN).unwrap()
        };

        Ok(Trade {
            trade_number,
            price,
            volume,
            date,
            time,
            qty,
            buy_agent: *buy_agent,
            sell_agent: *sell_agent,
            trade_type: *trade_type,
        })
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

lazy_static! {
    static ref TRADE_TYPES_MAP: HashMap<&'static str, i32> = vec![
        ("CrossTrade", 1),
        ("AggressorBuyer", 2),
        ("AggressorSeller", 3),
        ("Auction", 4),
        ("Expit", 6),
        ("Surveillance", 5),
        ("Options Exercise", 7),
        ("Over the counter", 8),
        ("Derivative Term", 9),
        ("Index", 10),
        ("BTC", 11),
        ("On Behalf", 12),
        ("RLP", 13),
        ("Desconhecido", 32),
        (constants::UNKNOWN, 32)
    ].into_iter()
    .map(|(k, v)| {
            let uppercase_key = k.to_uppercase();
            let static_str: &'static str = Box::leak(uppercase_key.into_boxed_str());
            (static_str, v)
        })
    .collect();

    static ref AGENT_MAP: HashMap<&'static str, i32> = vec![
        ("XP", 3),
        ("Alfa", 4),
        ("UBS", 8),
        ("Deutsche", 9),
        ("Spinelli", 10),
        ("Merrill", 13),
        ("Guide", 15),
        ("JP Morgan", 16),
        ("Votorantim", 21),
        ("Necton", 23),
        ("Santander CCVM", 27),
        ("Agora", 39),
        ("Morgan", 40),
        ("Credit CTVM", 45),
        ("Socopa", 58),
        ("Safra CVC", 59),
        ("Novinvest", 63),
        ("Bradesco", 72),
        ("Coinvalores", 74),
        ("Citigroup", 77),
        ("Corretora", 83),
        ("BTG CTVM", 85),
        ("CM Capital", 88),
        ("NuInvest", 90),
        ("Warren Rena", 92),
        ("Renascenca", 92),
        ("Nova Futura", 93),
        ("Merc.", 106),
        ("Terra", 107),
        ("Itau", 114),
        ("H.COMMCOR", 115),
        ("Commcor", 115),
        ("Genial", 120),
        ("BGC Liquidez", 122),
        ("Tullett", 127),
        ("Planner", 129),
        ("Fator", 131),
        ("Ativa", 147),
        ("Banrisul", 172),
        ("Elite", 174),
        ("Solidus", 177),
        ("Geral", 186),
        ("Sita", 187),
        ("XP Ativos Digitais", 188),
        ("Warren", 190),
        ("Senso", 191),
        ("J.P.", 206),
        ("Amaril", 226),
        ("Gradual", 227),
        ("Codepe", 234),
        ("Goldman", 238),
        ("BNP", 251),
        ("BB", 254),
        ("Mirae", 262),
        ("Citibank", 298),
        ("Safra", 304),
        ("Clear", 308),
        ("ZeroMarkets", 338),
        ("BEE4", 339),
        ("ATFX-1", 341),
        ("FX Globe", 344),
        ("ATFX", 345),
        ("BlackArrow OMS", 346),
        ("NinjaTrader", 347),
        ("BlackArrow", 348),
        ("Ylos - Swiset", 349),
        ("Ylos (Swiset)", 350),
        ("Rico", 386),
        ("Santander", 622),
        ("ABN", 688),
        ("ICAP", 735),
        ("LEV", 746),
        ("Credit", 833),
        ("BTG", 1026),
        ("BANCOSEGURO", 1081),
        ("RB Capital", 1089),
        ("Inter", 1099),
        ("Intl", 1130),
        ("Ideal", 1618),
        ("Sicredi", 1850),
        ("Vitreo DTVM S.A.", 1855),
        ("Sicoob", 1953),
        ("Modal", 1982),
        ("Unibanco", 2028),
        ("BB S.A", 2659),
        ("Orama", 3701),
        ("Toro", 4090),
        ("C6", 6003),
        ("Liga", 7029),
        ("SCOTIABANK", 7078),
        ("TC Traders Club SA", 811675),
        ("UNKNOWN", 9999999)
    ].into_iter()
    .map(|(k, v)| {
            let uppercase_key = k.to_uppercase();
            let static_str: &'static str = Box::leak(uppercase_key.into_boxed_str());
            (static_str, v)
        })
    .collect();
}

#[allow(unused)]
pub fn write_to_csv_file(
    asset_ticker: &str,
    output_file_path: &str,
    trades: Vec<Trade>,
) -> Result<(), io::Error> {
    let mut csv = File::create(output_file_path)?;
    writeln!(csv, "{}", TRADES_CSV_HEADER_LINE)?;
    for trade in trades {
        write!(csv, "{}\n", trade.to_csv_row(asset_ticker))?;
    }
    Ok(())
}

#[allow(unused)]
pub fn write_to_binary_file(trades: Vec<Trade>, output_file_path: &str) -> Result<(), anyhow::Error> {
    let mut binary_file = File::create(output_file_path)?;
    for trade in trades {
        let trade_binary_data = trade.to_bytes()?;
        binary_file.write_all(trade_binary_data.as_slice())?;
    }
    Ok(())
}

#[allow(unused)]
pub fn read_trades_from_binary_file(file_path: &str) -> Result<Vec<Trade>, anyhow::Error> {
    let file = File::open(file_path)?;
    file.sync_all()?;
    let file_size_bytes = file.metadata()?.len();
    let trade_count = (file_size_bytes / Trade::size_in_bytes() as u64) as usize;
    let mut trades = Vec::with_capacity(trade_count);

    let mut file_reader = io::BufReader::new(File::open(file_path)?);

    loop {
        let mut buffer: [u8; Trade::size_in_bytes()] = [0; Trade::size_in_bytes()];
        if file_reader.read_exact(&mut buffer).is_err() {
            break;
        }
        let trade = Trade::from_bytes(buffer.as_slice())?;
        trades.push(trade);
    }

    Ok(trades)
}


#[allow(unused)]
pub fn read_trades_from_csv(file_path: &str) -> Result<Vec<Trade>, anyhow::Error> {
    let mut file_reader = io::BufReader::new(File::open(file_path)?);

    let csv_header_line = &mut String::new();
    file_reader.read_line(csv_header_line)?;

    let expected_headers = [
        "<ticker>",
        "<date>",
        "<time>",
        "<trade_number>",
        "<price>",
        "<qty>",
        "<vol>",
        "<buy_agent>",
        "<sell_agent>",
        "<trade_type>",
        "<aft>",
    ];
    for (index, header) in csv_header_line.split(",").enumerate() {
        if expected_headers[index] != header
            && (index < 10 || expected_headers[index] != header.replace("\n", ""))
        {
            return !bail!(format!(
                "expected the header column {} in the {} position, but found {}",
                expected_headers[index],
                index,
                header
            ));
        }
    }

    let mut trades: Vec<Trade> = Vec::new();
    for line in file_reader.lines() {
        trades.push(Trade::from_csv_row(&mut line?)?);
    }
    Ok(trades)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::fs::remove_file;

    #[test]
    fn trade_size_in_bytes_should_be_explicitly_defined_correctly() {
        assert_eq!(44, Trade::size_in_bytes());
    }

    #[test]
    fn serialize_and_deserialize_trade_to_binary_correctly() {
        let example_trade = Trade {
            trade_number: 9332,
            price: 127800.0061701,
            date: 20241013, // yyyyMMdd
            time: 090034, // HHmmSS or HHmmSSsss
            volume: 40000.321,
            qty: 4,
            buy_agent: 1,
            sell_agent: 2,
            trade_type: 13,
        };

        let trade_bytes = example_trade.to_bytes().unwrap();
        assert_eq!(Trade::size_in_bytes(), trade_bytes.len());

        let deserialized_trade_struct = Trade::from_bytes(trade_bytes.as_slice());
        assert!(deserialized_trade_struct.is_ok());

        let deserialized_trade = deserialized_trade_struct.unwrap();
        assert_eq!(example_trade, deserialized_trade);
    }

    fn mock_trades() -> Vec<Trade> {
        let mut trades: Vec<Trade> = Vec::new();

        trades.push(Trade {
            trade_number: 78231,
            price: 87800.00000020,
            date: 20190102,
            time: 090034,
            volume: 40000.3000,
            qty: 12,
            buy_agent: 1,
            sell_agent: 2,
            trade_type: 13,
        });

        trades.push(Trade {
            trade_number: 78232,
            price: 87801.00000020,
            date: 20190102,
            time: 090035,
            volume: 30000.300,
            qty: 3,
            buy_agent: 3,
            sell_agent: 3,
            trade_type: 4,
        });

        trades
    }

    #[test]
    fn should_load_trades_from_binary_file_correctly() {
        let trades_file_path = "resources/example_binary_trades";
        let trades = read_trades_from_binary_file(trades_file_path);
        let result = trades.unwrap();

        assert_eq!(
            2,
            result.len(),
            "should have read 2 trades from the file '{}'",
            trades_file_path
        );

        let first_trade = result.first().unwrap();
        assert_eq!(78231, first_trade.trade_number);
        assert_eq!(87800.00000020, first_trade.price);
        assert_eq!(20190102, first_trade.date);
        assert_eq!(090034, first_trade.time);
        assert_eq!(40000.3000, first_trade.volume);
        assert_eq!(12, first_trade.qty);
        assert_eq!(1, first_trade.buy_agent);
        assert_eq!(2, first_trade.sell_agent);
        assert_eq!(13, first_trade.trade_type);
    }

    #[test]
    fn should_load_trades_from_example_csv_correctly() {
        let trades_file_path = "resources/example_history_trades.csv";
        let trades = read_trades_from_csv(trades_file_path);

        assert!(trades.is_ok(), "trade data failed to be loaded from '{}'", trades_file_path);
        let result = trades.unwrap();

        assert_eq!(
            9,
            result.len(),
            "should have read 9 trades from the file '{}'",
            trades_file_path
        );
    }

    #[test]
    fn should_create_binary_file_from_trades() {
        let trades = mock_trades();
        let test_binary_trades_file_path = "resources/WIN_BINARY_TEST";
        if Path::new(test_binary_trades_file_path).exists() {
            remove_file(test_binary_trades_file_path).unwrap();
        }

        write_to_binary_file(trades, test_binary_trades_file_path).unwrap();
        assert!(Path::new(test_binary_trades_file_path).exists());

        remove_file(test_binary_trades_file_path).unwrap();
    }

    #[test]
    fn should_create_csv_from_trades_file() {
        let trades = mock_trades();

        let test_trades_file_path = "resources/test_trades.csv";
        if Path::new(test_trades_file_path).exists() {
            remove_file(test_trades_file_path).unwrap();
        }

        write_to_csv_file("WIN", test_trades_file_path, trades).unwrap();
        assert!(Path::new(test_trades_file_path).exists());

        let mut generated_file = io::BufReader::new(File::open(test_trades_file_path).unwrap());

        // check for the header line
        let csv_header_line = &mut String::new();
        generated_file.read_line(csv_header_line).unwrap();
        assert_eq!(TRADES_CSV_HEADER_LINE, csv_header_line.replace("\n", ""));

        let first_trade_line = &mut String::new();
        generated_file.read_line(first_trade_line).unwrap();
        assert_eq!(
            "WIN,20190102,90034,78231,87800.0000002,12,40000.3,Modal,XP,Auction,N\n",
            first_trade_line
        );

        let second_trade_line = &mut String::new();
        generated_file.read_line(second_trade_line).unwrap();
        assert_eq!(
            "WIN,20190102,90035,78232,87801.0000002,3,30000.3,Modal,XP,Auction,N\n",
            second_trade_line
        );

        remove_file(test_trades_file_path).unwrap();
    }
}
