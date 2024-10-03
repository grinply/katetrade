// use std::fs::{File, remove_file};
// use std::io::{BufRead, BufReader, Write, Read};
// use std::net::TcpListener;
// use std::process::{Command, Stdio};
// use std::sync::{Arc, Mutex, Condvar};
// use std::thread;
// use std::time::Duration;

// /// Function to simulate an HTTP server that responds to a POST request.
// fn start_http_server() {
//     thread::spawn(move || {
//         let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
//         for stream in listener.incoming() {
//             let mut stream = stream.unwrap();
//             let mut buffer = [0; 512];
//             stream.read(&mut buffer).unwrap();

//             if buffer.starts_with(b"POST /feed/WIN/start HTTP/1.1") {
//                 let response = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nKEY1";
//                 stream.write_all(response.as_bytes()).unwrap();
//             }
//         }
//     });
// }

// /// Function to simulate a socket server that sends 1000 different Trade messages.
// fn start_socket_server(received_count: Arc<(Mutex<usize>, Condvar)>) {
//     thread::spawn(move || {
//         let listener = TcpListener::bind("127.0.0.1:8066").unwrap();
//         for stream in listener.incoming() {
//             let mut stream = stream.unwrap();

//             for i in 0..1000 {
//                 let trade_data = create_trade_data(
//                     100.0 + i as f64,           // Unique price
//                     50.0 + i as f64,            // Unique volume
//                     1630456789000 + i as i64,   // Unique timestamp
//                     10 + i as i32,              // Unique qty
//                     i as i32,                   // Unique buy_agent
//                     (1000 - i) as i32,          // Unique sell_agent
//                     i % 5,                      // Unique trade_type
//                     i % 2 == 0,                 // Toggle edit_trade
//                     &format!("TICKER{}", i),    // Unique asset_ticker
//                 );

//                 // Send trade data to the connected client
//                 stream.write_all(&trade_data).unwrap();

//                 let (lock, cvar) = &*received_count;
//                 let mut count = lock.lock().unwrap();
//                 *count += 1;

//                 if *count == 1000 {
//                     cvar.notify_one();
//                 }
//             }
//         }
//     });
// }

// /// Function to create mock binary data for a Trade struct
// fn create_trade_data(
//     price: f64,
//     volume: f64,
//     timestamp_millis: i64,
//     qty: i32,
//     buy_agent: i32,
//     sell_agent: i32,
//     trade_type: i32,
//     edit_trade: bool,
//     asset_ticker: &str,
// ) -> Vec<u8> {
//     let mut data = Vec::new();
//     data.extend_from_slice(&price.to_be_bytes());
//     data.extend_from_slice(&volume.to_be_bytes());
//     data.extend_from_slice(&timestamp_millis.to_be_bytes());
//     data.extend_from_slice(&qty.to_be_bytes());
//     data.extend_from_slice(&buy_agent.to_be_bytes());
//     data.extend_from_slice(&sell_agent.to_be_bytes());
//     data.extend_from_slice(&trade_type.to_be_bytes());
//     data.push(if edit_trade { 1 } else { 0 });
//     data.extend_from_slice(asset_ticker.as_bytes());
//     data
// }

// #[test]
// fn test_integration() {
//     // Shared counter and condition variable to track the number of messages received
//     let received_count = Arc::new((Mutex::new(0), Condvar::new()));
//     let received_count_clone = Arc::clone(&received_count);

//     start_http_server();
//     start_socket_server(received_count_clone);

//     thread::sleep(Duration::from_secs(1));

//     let mut child = Command::new("cargo")
//         .arg("run")
//         .stdin(Stdio::piped())  // To send commands to the application
//         .stdout(Stdio::inherit())  // To inherit the standard output
//         .stderr(Stdio::inherit())  // To inherit the standard error
//         .spawn()
//         .expect("Failed to run the application");

//     {
//         let (lock, cvar) = &*received_count;
//         let mut count = lock.lock().unwrap();
//         while *count < 1000 {
//             count = cvar.wait(count).unwrap();
//         }
//     }

//     if let Some(stdin) = child.stdin.as_mut() {
//         stdin.write_all(b"exit\n").expect("Failed to send exit command");
//     }

//     let _ = child.wait().expect("Failed to wait on child process");

//     let csv_file_path = "trades_output.csv";
//     validate_csv(csv_file_path);

//     remove_file(csv_file_path).expect("Failed to delete the CSV file");
// }

// /// Function to validate the content of the generated CSV file
// fn validate_csv(file_path: &str) {
//     let file = File::open(file_path).expect("Failed to open CSV file");
//     let reader = BufReader::new(file);

//     let mut lines = reader.lines();
//     let header = lines.next().expect("No header found").expect("Failed to read header");
//     assert_eq!(
//         header,
//         "price,volume,timestamp_millis,qty,buy_agent,sell_agent,trade_type,edit_trade,asset_ticker"
//     );

//     let mut records = std::collections::HashSet::new();
//     let mut count = 0;

//     // Count the number of records and ensure they are unique and correctly formatted
//     for line in lines {
//         let record = line.expect("Failed to read a CSV record");
//         count += 1;

//         // Check the format and uniqueness of the record
//         let fields: Vec<&str> = record.split(',').collect();
//         assert_eq!(fields.len(), 9, "Expected 9 fields but found {}", fields.len());

//         assert!(fields[0].parse::<f64>().is_ok(), "Price field is not a valid f64");
//         assert!(fields[1].parse::<f64>().is_ok(), "Volume field is not a valid f64");
//         assert!(fields[2].parse::<i64>().is_ok(), "Timestamp field is not a valid i64");
//         assert!(fields[3].parse::<i32>().is_ok(), "Qty field is not a valid i32");
//         assert!(fields[4].parse::<i32>().is_ok(), "Buy Agent field is not a valid i32");
//         assert!(fields[5].parse::<i32>().is_ok(), "Sell Agent field is not a valid i32");
//         assert!(fields[6].parse::<i32>().is_ok(), "Trade Type field is not a valid i32");
//         assert!(fields[7].parse::<bool>().is_ok(), "Edit Trade field is not a valid bool");

//         assert!(records.insert(record.clone()), "Duplicate record found: {}", record);
//     }

//     assert_eq!(count, 1000, "Expected 1000 trades, but found {}", count);
// }
