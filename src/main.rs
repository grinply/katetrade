mod data_feed_reader;
mod market_connector_client;
mod trade_model;

use data_feed_reader::DataFeedReader;
use market_connector_client::get_trade_feed_key;
use std::io::{self, Write};
use std::net::TcpStream;

fn main() -> io::Result<()> {
    let message = get_trade_feed_key("market-connector.grinply.com")?;
    println!("{}", message);

    // set timeout to 15 seconds
    let mut stream = TcpStream::connect("market-connector.grinply.com:8022")?;
    println!("Connected to the remote socket server.");

    let key = message + "\n";

    if let Err(e) = stream.write_all(key.as_bytes()) {
        eprintln!("Failed to send message: {}", e);
        return Err(e); // Propagate the error
    }

    let data_feed_reader = DataFeedReader::new();
    let handle = data_feed_reader.start_reading(stream);

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("exit") {
            println!("Exiting program...");
            break;
        }
    }

    data_feed_reader.stop();
    handle.join().unwrap();

    data_feed_reader
        .save_trades_to_csv("trades_output.csv")
        .expect("Failed to save trades to CSV");

    println!("Main thread has finished.");
    Ok(())
}
