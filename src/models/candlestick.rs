use crate::models::trade::Trade;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Candlestick {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub date: i32,
    pub start_time: i32,
    pub is_closed: bool,
    pub trades: u64
}

#[allow(unused)]
pub trait CandlestickGenerator {
    fn description(&self) -> String;
    fn reset_candle_when_day_changes() -> bool;
    fn parse(&mut self, new_trade: &Trade) -> Candlestick;
}

#[allow(unused)]
pub struct TimeCandlestickGenerator {
    last_price: f64,
    last_day: i32,
    period_minutes: u64,
    last_candle: Option<Candlestick>
}

#[allow(unused)]
pub struct TickCandlestickGenerator {
    ticks_per_candle: u64,
    last_candle: Option<Candlestick>
}

#[allow(unused)]
pub struct VolumeCandlestickGenerator {
    volume_per_candle: f64,
    last_candle: Option<Candlestick>
}


impl Candlestick {
    fn new(first_trade: &Trade) -> Candlestick{
        Candlestick{
            open: first_trade.price,
            high: first_trade.price,
            low: first_trade.price,
            close: first_trade.price,
            volume: first_trade.volume,
            date: first_trade.date,
            start_time: first_trade.time,
            is_closed: false,
            trades: 1
        }
    }
}


#[allow(unused)]
impl TimeCandlestickGenerator {
    fn new(period_minutes: u64) -> TimeCandlestickGenerator {
        TimeCandlestickGenerator{
            last_price: 0.0,
            last_day: 0,
            period_minutes,
            last_candle: None,
        }
    }
}

impl CandlestickGenerator for TimeCandlestickGenerator {

    fn description(&self) -> String {
        String::from(format!("candlesticks based on time slices of {} minutes", self.period_minutes))
    }

    fn reset_candle_when_day_changes() -> bool {
        true
    }

    fn parse(&mut self, new_trade: &Trade) -> Candlestick {
        if self.last_day != 0 && self.last_day != new_trade.date {
            let mut last_candle_from_yesterday = self.last_candle.clone().unwrap();
            last_candle_from_yesterday.close = self.last_price;

            self.last_day = new_trade.date;
            self.last_price = new_trade.price;
            self.last_candle = Some(Candlestick::new(new_trade));

            return last_candle_from_yesterday;
        }

        match &mut self.last_candle {
            Some(existing_candle) => {
                let is_candle_closed = false;
                if !is_candle_closed {
                    existing_candle.trades +=1;
                    existing_candle.volume += new_trade.volume;

                    if new_trade.price < existing_candle.low {
                        existing_candle.low = new_trade.price;
                    }
                    if new_trade.price > existing_candle.high {
                        existing_candle.high = new_trade.price;
                    }
                }else{
                    existing_candle.close = self.last_price;
                }
            }
            None => {
                self.last_candle = Some(Candlestick::new(new_trade));
            }
        }
        self.last_price = new_trade.price;
        self.last_candle.clone().unwrap()
    }
}


#[cfg(test)]
mod tests {
    // use super::*;
    // #[test]
    // fn time_candlesticks_generator_should_correctly_create_candles_of_one_minute() {
    //     let generator = TimeCandlestickGenerator::new(1);
    // }
}

