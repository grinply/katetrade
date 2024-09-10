use std::fs::File;
use std::io::{self, Write};

use crate::trade_model::Trade;
use std::io::Read;
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

/// Struct to manage data feed reading from a socket
pub struct DataFeedReader {
    pub running: Arc<AtomicBool>,
    pub trades: Arc<Mutex<Vec<Trade>>>,
}

impl DataFeedReader {
    /// Creates a new DataFeedReader
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
            trades: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Starts reading from the given socket stream in a separate thread
    pub fn start_reading(&self, stream: TcpStream) -> thread::JoinHandle<()> {
        let running_clone = Arc::clone(&self.running);
        let trades_clone: Arc<Mutex<Vec<Trade>>> = Arc::clone(&self.trades);
        let mut stream_clone = stream.try_clone().expect("Failed to clone stream");

        thread::spawn(move || {
            let mut buffer = [0; 8192];

            // Loop indefinitely until the flag is set to false
            while running_clone.load(Ordering::SeqCst) {
                match stream_clone.read(&mut buffer) {
                    Ok(0) => {
                        println!("Connection closed by the server.");
                        break;
                    }
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            let trade = Trade::from_bytes(&buffer[..bytes_read]);
                            println!(
                                "new trade for {} with price {}\n",
                                trade.asset_ticker, trade.price
                            );
                            // Store the trade in the shared list
                            let mut trades = trades_clone.lock().unwrap();
                            trades.push(trade);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from server: {}", e);
                        break;
                    }
                }
            }
            println!("Reading thread has stopped.");
        })
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn get_trades(&self) -> Arc<Mutex<Vec<Trade>>> {
        Arc::clone(&self.trades)
    }

    pub fn save_trades_to_csv(&self, file_path: &str) -> io::Result<()> {
        let trades = self.trades.lock().unwrap();
        let mut file = File::create(file_path)?;

        writeln!(
                file,
                "price,volume,timestamp_millis,qty,buy_agent,sell_agent,trade_type,edit_trade,asset_ticker"
            )?;

        for trade in trades.iter() {
            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{}",
                trade.price,
                trade.volume,
                trade.timestamp_millis,
                trade.qty,
                trade.buy_agent,
                trade.sell_agent,
                trade.trade_type,
                trade.edit_trade,
                trade.asset_ticker
            )?;
        }

        Ok(())
    }
}
