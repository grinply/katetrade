use crate::models::trade::{Trade};
use std::io::Read;
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

/// Struct to manage data feed reading from a socket
pub struct TradeFeedStream {
    pub running: Arc<AtomicBool>,

}

impl TradeFeedStream {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Starts reading from the given socket stream in a separate thread
    pub fn start_reading(&self, stream: TcpStream) -> thread::JoinHandle<()> {
        let running_clone = Arc::clone(&self.running);
        let mut stream_clone = stream.try_clone().expect("Failed to clone stream");

        thread::spawn(move || {
            let mut buffer = [0; 8192];

            // Loop indefinitely until the flag is set to false
            while running_clone.load(Ordering::SeqCst){
                match stream_clone.read(&mut buffer) {
                    Ok(0) => {
                        println!("Connection closed by the server.");
                        break;
                    }
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            let trade = Trade::from_bytes(&buffer[..bytes_read]);

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
}
