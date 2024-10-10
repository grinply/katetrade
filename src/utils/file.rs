use crate::models::trade::Trade;
use std::fs::{File};
use std::io::{self, BufRead, Read, Write};

const TRADES_CSV_HEADER_LINE: &str = "<ticker>,<date>,<time>,<trade_number>,<price>,<qty>,<vol>,<buy_agent>,<sell_agent>,<trade_type>,<aft>";

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

pub fn write_to_binary_file(trades: Vec<Trade>, output_file_path: &str) -> Result<(), io::Error> {
    let mut csv = File::create(output_file_path)?;
    for trade in trades {
        csv.write_all(trade.to_bytes())?;
    }
    Ok(())
}

pub fn read_trades_from_binary_file(file_path: &str) -> Result<Vec<Trade>, io::Error> {
    let file = File::open(file_path)?;
    file.sync_all()?;
    let file_size_bytes = file.metadata()?.len();
    //since every Trade struct has 32 bytes we can reserve the exact required space
    let trade_count = (file_size_bytes / Trade::size_in_bytes() as u64) as usize;
    let mut trades = Vec::with_capacity(trade_count);

    let mut file_reader = io::BufReader::new(File::open(file_path)?);

    loop {
        let mut buffer: [u8; 32] = [0; 32];
        if file_reader.read_exact(&mut buffer).is_err() {
            break;
        }
        let trade = Trade::from_bytes(buffer.as_slice());
        trades.push(trade);
    }

    Ok(trades)
}

pub fn read_trades_from_csv(file_path: &str) -> Result<Vec<Trade>, io::Error> {
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
            let error_msg = std::format!(
                "expected the header column {} in the {} position, but found {}",
                expected_headers[index],
                index,
                header
            );
            return Err(io::Error::new(io::ErrorKind::InvalidData, error_msg));
        }
    }

    let mut trades: Vec<Trade> = Vec::new();
    for line in file_reader.lines() {
        trades.push(Trade::from_csv_row(&mut line?));
    }
    return Ok(trades);
}

// Test module
#[cfg(test)]
mod tests {
    use std::path::Path;

    use io::Read;

    use super::*;

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
            volume: 30000.3000,
            qty: 3,
            buy_agent: 2,
            sell_agent: 2,
            trade_type: 2,
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
    }

    #[test]
    fn should_load_trades_from_example_csv_correctly() {
        let trades_file_path = "resources/example_history_trades.csv";
        let trades = read_trades_from_csv(trades_file_path);
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
        let test_binary_trades_file_path = "resources/WIN_TRADES";
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
