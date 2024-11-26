use crate::models::candlestick::Candlestick;
use crate::models::trade::Trade;

/// abstraction over price data used when running backtests
pub(crate) trait DataPoint {
    fn date(&self) -> i32;

    #[allow(unused)]
    fn time(&self) -> i32;

    fn exit_time(&self) -> i32;

    fn current_price(&self) -> f64;
}

impl DataPoint for Candlestick {
    fn date(&self) -> i32 {
        self.date
    }

    fn time(&self) -> i32 {
        self.start_time
    }

    fn exit_time(&self) -> i32 {
        self.close_time
    }

    fn current_price(&self) -> f64 {
        self.close
    }
}

impl DataPoint for Trade {
    fn date(&self) -> i32 {
        self.date
    }

    fn time(&self) -> i32 {
        self.time
    }

    fn exit_time(&self) -> i32 {
        self.time
    }

    fn current_price(&self) -> f64 {
        self.price
    }
}