use chrono::NaiveTime;
use crate::models::candlestick::Candlestick;
use crate::models::trade::Trade;
use crate::utils::date;

/// abstraction over price data used when running backtests
pub(crate) trait DataPoint {
    fn date(&self) -> i32;

    #[allow(unused)]
    fn time(&self) -> NaiveTime;

    fn exit_time(&self) -> Option<NaiveTime>;

    fn current_price(&self) -> f64;
}

impl DataPoint for Candlestick {
    fn date(&self) -> i32 {
        self.date
    }

    fn time(&self) -> NaiveTime {
        self.start_time
    }

    fn exit_time(&self) -> Option<NaiveTime> {
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

    fn time(&self) -> NaiveTime {
        let time = date::convert_to_naive_time(self.time);
        assert!(time.is_some(), "time included in the trade should be valid {}", self.time);
        time.unwrap()
    }

    fn exit_time(&self) -> Option<NaiveTime> {
        Some(self.time())
    }

    fn current_price(&self) -> f64 {
        self.price
    }
}