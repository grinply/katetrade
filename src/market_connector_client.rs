use std::io::{self, Read, Write};
use std::net::TcpStream;

pub fn get_trade_feed_key(host: &str) -> io::Result<String> {
    let mut http_stream = TcpStream::connect((host, 80))?;

    let http_request = format!(
        "POST {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Length: 0\r\n\
         Connection: close\r\n\
         \r\n",
        "/api/B3/market/feed/trades/FUTURES/WIN/subscribe", host
    );

    http_stream.write_all(http_request.as_bytes())?;

    let mut response = String::new();
    http_stream.read_to_string(&mut response)?;

    Ok(extract_plain_text_from_http_response(&response))
}

fn extract_plain_text_from_http_response(response: &str) -> String {
    if let Some(body_start) = response.find("\r\n\r\n") {
        return response[(body_start + 4)..].trim().to_string();
    }
    String::new()
}
