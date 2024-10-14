use crate::models::candlestick::Candlestick;
use crate::models::trade::Trade;

pub enum MovingAverageError {
    ZeroWindowSize,
    WindowSizeTooLarge,
}

impl std::fmt::Display for MovingAverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MovingAverageError::ZeroWindowSize => write!(f, "Window size cannot be zero."),
            MovingAverageError::WindowSizeTooLarge => write!(f, "Window size is larger than the prices vector."),
        }
    }
}

fn simple_moving_average_trade(trade_feed: Vec<Trade>, window: usize) -> Result<f64, MovingAverageError>{
    let prices: Vec<f64> = trade_feed.iter().map(|trade| trade.price).collect();
    calculate_sma(prices, window)
}

fn simple_moving_average_candlestick(candlestick_feed: Vec<Candlestick>, window: usize) -> Result<f64, MovingAverageError>{
    let closes: Vec<f64> = candlestick_feed.iter().map(|candle| candle.close).collect();
    calculate_sma(closes, window)
}

fn calculate_sma(data: Vec<f64>, window: usize) -> Result<f64, MovingAverageError>{
    if window == 0 {
        return Err(MovingAverageError::ZeroWindowSize);
    }
    if data.len() < window {
        return Err(MovingAverageError::WindowSizeTooLarge);
    }

    let mut average = 0.0;
    let start = data.len() - window;

    for value in &data[start..] {
        average += *value;
    }

    Ok(average/window as f64)
}