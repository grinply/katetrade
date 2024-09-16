use crate::models::trade::Trade;
use std::io::{self, BufRead};
use std::fs::File;

pub fn read_trades_from_csv(file_path: &str) -> Result<Vec<Trade>, io::Error> {
    let mut file_reader = io::BufReader::new(File::open(file_path)?);

    let csv_header_line = &mut String::new();
    file_reader.read_line(csv_header_line)?;
    
    let expected_headers = ["<ticker>", "<date>", "<time>", "<trade_number>", "<price>", "<qty>", "<vol>", "<buy_agent>", "<sell_agent>", "<trade_type>", "<aft>"];
    for (index, header) in csv_header_line.split(",").enumerate() {
        if expected_headers[index] != header && (index < 10 || expected_headers[index] != header.replace("\n", "")) {
            let error_msg = std::format!("expected the header column {} in the {} position, but found {}", expected_headers[index], index, header);
            return Err(io::Error::new(io::ErrorKind::InvalidData, error_msg))
        }
    }
    
    let mut trades: Vec<Trade> = Vec::new();
    for line in file_reader.lines(){
        trades.push(Trade::from_csv_row(&mut line?));
    }
    return Ok(trades);
}

// Test module
#[cfg(test)]
mod tests {
    use super::*;

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
}
