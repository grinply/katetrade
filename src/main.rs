// mod data_feed_reader;
// mod market_connector_client;
// mod live_market_connection;
// mod models;
pub mod utils;
pub mod models;

use std::io;
use utils::csv_loader;

fn main() -> io::Result<()> {
    println!("hello world.");
    csv_loader::read_trades_from_csv("myfile");
    Ok(())
}
