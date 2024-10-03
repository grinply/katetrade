use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use crate::models::market::Market;

pub fn get_trade_feed_key(host: &str, ticker: &str, market: Market) -> io::Result<String> {
    let mut http_stream = TcpStream::connect((host, 80))?;

    let http_request = format!(
        "POST /api/B3/market/feed/trades/{}/{}/subscribe HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Length: 0\r\n\
         Connection: close\r\n\
         \r\n",
        market, ticker, host
    );

    http_stream.write_all(http_request.as_bytes())?;

    let mut response = String::new();
    http_stream.read_to_string(&mut response)?;

    if let Some(body_start) = response.find("\r\n\r\n") {
        return Ok(response[(body_start + 4)..].trim().to_string());
    }

    Err(io::Error::new(ErrorKind::InvalidData, "failed to get trade feed key"))
}

fn extract_plain_text_from_http_response(response: &str) -> String {

    String::new()
}
