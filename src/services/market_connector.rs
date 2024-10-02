use super::clients::market_connector_client::get_trade_feed_key;
use crate::services::data_feed_reader::TradeFeedStream;
use std::io::{self, Write};
use std::net::TcpStream;
use crate::models::market::Market;


static MARKET_CONNECTOR_HOST: &str = "market-connector.grinply.com";

fn connect_to_live_markets(ticker: &str, market: Market) -> io::Result<()> {
    let message = get_trade_feed_key(MARKET_CONNECTOR_HOST, ticker, market)?;
    println!("{}", message);

    let socket_address = format!("{}:{}",MARKET_CONNECTOR_HOST,8022);
    // set timeout to 15 seconds
    let mut stream = TcpStream::connect(socket_address)?;
    println!("Connected to the remote socket server.");

    let key = message + "\n";

    if let Err(e) = stream.write_all(key.as_bytes()) {
        eprintln!("Failed to send message: {}", e);
        return Err(e); // Propagate the error
    }
    let data_feed_reader = TradeFeedStream::new();
    let handle = data_feed_reader.start_reading(stream);
    data_feed_reader.stop();
    handle.join().unwrap();
    Ok(())
}
