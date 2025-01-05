use crate::models::trade::Trade;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use crate::models::market::Market;
use crate::services::market_connector::MarketConnectorClient;
use crossbeam_channel::{unbounded, Receiver, Sender};

/// Struct to manage data feed reading from a socket
pub struct TradeFeedStream<'a> {
    socket_url: &'a str,
    ticker: &'a str,
    client: &'a MarketConnectorClient,
    market: Market,
    running: Arc<AtomicBool>,
    sender_channel: Sender<Trade>,
    consumer_channel: Receiver<Trade>,
}

#[allow(unused)]
impl <'a>TradeFeedStream<'a> {
    pub fn new(socket_url: &'a str, ticker: &'a str, market: Market, client: &'a MarketConnectorClient) -> Self {
        let channel: (Sender<Trade>, Receiver<Trade>) = unbounded();
        Self {
            ticker,
            market,
            client,
            socket_url,
            sender_channel: channel.0,
            consumer_channel: channel.1,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_reading(&self) -> anyhow::Result<Receiver<Trade>> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(self.consumer_channel.clone());
        }

        let mut stream = self.establish_socket_connection()?;
        let running_clone = Arc::clone(&self.running);
        let sender_handle = self.sender_channel.clone();

        thread::spawn(move || {
            let mut buffer_current_trades = [0; 16384];
            let mut buffer_last_incomplete_trade = [0; 44];
            let mut left_over_bytes: usize = 0;

            // Loop indefinitely until the flag is set to false
            while running_clone.load(Ordering::Relaxed) {
                match stream.read(&mut buffer_current_trades) {
                    Ok(0) => {
                        println!("Connection closed by the server. lets reconnect");
                        break;
                    }
                    Ok(size_bytes_read) => {
                        let offset_current_buffer: usize = if left_over_bytes > 0 {
                            // if we have left over bytes from the last read how many bytes from the
                            // start of the current buffer should we take in order to complete the trade
                            Trade::size_in_bytes() - left_over_bytes
                        } else{
                            0
                        };

                        let bytes_from_incomplete_trade = (left_over_bytes + size_bytes_read) % Trade::size_in_bytes();
                        if left_over_bytes > 0 {
                            buffer_last_incomplete_trade[left_over_bytes..].copy_from_slice(&buffer_current_trades[..offset_current_buffer]);
                            let trade = Trade::from_bytes(&buffer_last_incomplete_trade);
                            if trade.is_ok() {
                                sender_handle.send(trade.unwrap());
                            }else{
                                println!("failed to parse trade {}", trade.err().unwrap());
                            }
                        }

                        let qty_valid_trades_current_buffer = (size_bytes_read - offset_current_buffer) / Trade::size_in_bytes();
                        for i in 0..qty_valid_trades_current_buffer {
                            let pos_first_byte =  offset_current_buffer + i * Trade::size_in_bytes();
                            let pos_last_byte = offset_current_buffer + Trade::size_in_bytes() + (i * Trade::size_in_bytes());
                            let trade = Trade::from_bytes(&buffer_current_trades[pos_first_byte..pos_last_byte]);
                            if trade.is_ok() {
                                sender_handle.send(trade.unwrap());
                            }else{
                                println!("failed to parse trade {}", trade.err().unwrap());
                            }

                        }

                        left_over_bytes = (size_bytes_read - offset_current_buffer) % Trade::size_in_bytes();
                        if left_over_bytes > 0 {
                            let start_offset_incomplete_trade = size_bytes_read - left_over_bytes;
                            buffer_last_incomplete_trade[..left_over_bytes]
                                .copy_from_slice(&buffer_current_trades[start_offset_incomplete_trade..size_bytes_read]);
                        }
                    }
                    Err(e) => {
                        // try to auto reconnect
                        eprintln!("Failed to read from server: {}", e);
                        break;
                    }
                }
            }
            println!("Reading thread has stopped.");
        });

        self.running.store(true, Ordering::SeqCst);
        Ok(self.consumer_channel.clone())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn establish_socket_connection(&self) -> anyhow::Result<TcpStream> {
        let feed_key = self.client.get_trade_feed_key(self.ticker, self.market)? + "\n";
        let mut stream = TcpStream::connect(self.socket_url)?;

        if stream.write_all(feed_key.as_bytes()).is_err() {
            eprintln!("Failed key '{}' to connect in the trade feed for {}", feed_key, self.ticker);
            return anyhow::bail!("Failed to connect to the trade feed server");
        }
        Ok(stream)
    }
}
