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
    pub trades: u64,
}

#[allow(unused)]
pub trait CandlestickGenerator {
    fn description(&self) -> String;
    fn reset_candle_when_day_changes() -> bool;

    /// generates a brand-new Candlestick for every new trade, representing the still open candle
    /// it and also the closed when they finally do close.
    fn parse_with_open_candles(&mut self, new_trade: &Trade) -> Candlestick;


    /// generates a new Candlestick only the candlestick is closed
    /// it will only update its internal state with the new_trade in the mean time
    fn parse_only_closed_candles(&mut self, new_trade: &Trade) -> Option<Candlestick>;
}

#[allow(unused)]
pub struct TimeCandlestickGenerator {
    last_day: i32,
    last_price: f64,
    period_minutes: i32,
    last_candle: Option<Candlestick>,
}

#[allow(unused)]
pub struct TickCandlestickGenerator {
    ticks_per_candle: u64,
    last_candle: Option<Candlestick>,
}

#[allow(unused)]
pub struct VolumeCandlestickGenerator {
    volume_per_candle: f64,
    last_candle: Option<Candlestick>,
}


impl Candlestick {
    fn new(first_trade: &Trade) -> Candlestick {
        Candlestick {
            open: first_trade.price,
            high: first_trade.price,
            low: first_trade.price,
            close: first_trade.price,
            volume: first_trade.volume,
            date: first_trade.date,
            start_time: first_trade.time,
            is_closed: false,
            trades: 1,
        }
    }
}


#[allow(unused)]
impl TimeCandlestickGenerator {
    fn new(period_minutes: i32) -> TimeCandlestickGenerator {
        TimeCandlestickGenerator {
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

    fn parse_with_open_candles(&mut self, new_trade: &Trade) -> Candlestick {
        TimeCandlestickGenerator::parse(self, new_trade, false).unwrap()
    }

    fn parse_only_closed_candles(&mut self, new_trade: &Trade) -> Option<Candlestick> {
        TimeCandlestickGenerator::parse(self, new_trade, true)
    }
}

impl TimeCandlestickGenerator {
    fn parse(&mut self, new_trade: &Trade, only_closed_candles: bool) -> Option<Candlestick> {
        if self.last_candle.is_none() { // the very start when not trade was parsed yet
            self.last_price = new_trade.price;
            let new_candle = Candlestick::new(new_trade);
            self.last_candle = Some(new_candle.clone());

            return if only_closed_candles {
                return None;
            } else {
                Some(new_candle)
            };
        }
        // if the day rolls over we close the current candle
        if self.last_day != 0 && self.last_day != new_trade.date {
            let mut last_candle_from_yesterday = self.last_candle.clone().unwrap();
            last_candle_from_yesterday.close = self.last_price;
            last_candle_from_yesterday.is_closed = true;

            self.last_day = new_trade.date;
            self.last_price = new_trade.price;
            self.last_candle = Some(Candlestick::new(new_trade));

            return Some(last_candle_from_yesterday);
        }

        let existing_candle = self.last_candle.as_mut().unwrap();

        // time is represented as a number with format HHmmSS or HHmmSSsss
        // we do some math to calculate to make all values beyond the minute zero values to better
        // calculate the candlestick close
        let digits_to_remove_current_time: u32 = if new_trade.time < 99_99_99 {
            2
        }else{
            5
        };

        let digits_to_remove_start_time: u32 = if existing_candle.start_time < 99_99_99 {
            2
        }else{
            5
        };

        let shift_current_time: i32 = 10i32.pow(digits_to_remove_current_time);
        let shift_start_time: i32 = 10i32.pow(digits_to_remove_start_time);

        let is_candle_closed = (new_trade.time/shift_current_time) >= (existing_candle.start_time / shift_start_time) + self.period_minutes;

        if !is_candle_closed {
            existing_candle.trades += 1;
            existing_candle.volume += new_trade.volume;

            if new_trade.price < existing_candle.low {
                existing_candle.low = new_trade.price;
            }
            if new_trade.price > existing_candle.high {
                existing_candle.high = new_trade.price;
            }

            existing_candle.close = new_trade.price;
            existing_candle.is_closed = false;

            if only_closed_candles {
                self.last_price = new_trade.price;
                return None;
            }
        } else {
            existing_candle.is_closed = true;
        }

        self.last_price = new_trade.price;
        let result_candle = existing_candle.clone();

        if existing_candle.is_closed {
            self.last_candle = Some(Candlestick::new(new_trade));
        }
        Some(result_candle)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_candlesticks_generator_should_correctly_create_candles_of_one_minute_including_not_closed_candles(){
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = mock_trades_list();
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            let new_candle = generator.parse_with_open_candles(&trade);
            result_candlesticks.push(new_candle);
        }
        assert_eq!(trades.len(), result_candlesticks.len());
        assert_eq!(false, result_candlesticks.get(5).unwrap().is_closed, "the time didnt moved to the next minute, candle should not be closed");

        let first_closed_candle = result_candlesticks.get(6).unwrap();
        assert_eq!(true, first_closed_candle.is_closed);
        assert_eq!(131175.0, first_closed_candle.open);
        assert_eq!(131385.0, first_closed_candle.high);
        assert_eq!(131010.0, first_closed_candle.low);
        assert_eq!(131095.0, first_closed_candle.close);
        assert_eq!(6, first_closed_candle.trades);
        assert_eq!(09_00_00, first_closed_candle.start_time);

        let second_closed_candle = result_candlesticks.get(11).unwrap();
        assert_eq!(true, second_closed_candle.is_closed);
        assert_eq!(131000.0, second_closed_candle.open);
        assert_eq!(131200.0, second_closed_candle.high);
        assert_eq!(131000.0, second_closed_candle.low);
        assert_eq!(131200.0, second_closed_candle.close);
        assert_eq!(5, second_closed_candle.trades);
        assert_eq!(09_01_07_234, second_closed_candle.start_time);

        let last_candle = result_candlesticks.last().unwrap();

        assert_eq!(false, last_candle.is_closed);
        assert_eq!(131210.0, last_candle.open);
        assert_eq!(131225.0, last_candle.high);
        assert_eq!(131210.0, last_candle.low);
        assert_eq!(131225.0, last_candle.close);
        assert_eq!(3, last_candle.trades);
        assert_eq!(09_02_10, last_candle.start_time);
    }
    #[test]
    fn time_candlesticks_generator_should_correctly_create_closed_only_candles_of_one_minute() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = mock_trades_list();
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in trades {
            match generator.parse_only_closed_candles(&trade) {
                Some(candlestick) => {
                    result_candlesticks.push(candlestick);
                }
                None => {}
            }
        }
        assert_eq!(2, result_candlesticks.len(), "should only have {} closed candlesticks, found {}", 2, result_candlesticks.len());

        let first_candle = result_candlesticks.first().unwrap();

        assert_eq!(true, first_candle.is_closed);
        assert_eq!(131175.0, first_candle.open);
        assert_eq!(131385.0, first_candle.high);
        assert_eq!(131010.0, first_candle.low);
        assert_eq!(131095.0, first_candle.close);
        assert_eq!(6, first_candle.trades);
        assert_eq!(09_00_00, first_candle.start_time);

        let second_candle = result_candlesticks.last().unwrap();

        assert_eq!(true, second_candle.is_closed);
        assert_eq!(131000.0, second_candle.open);
        assert_eq!(131200.0, second_candle.high);
        assert_eq!(131000.0, second_candle.low);
        assert_eq!(131200.0, second_candle.close);
        assert_eq!(5, second_candle.trades);
        assert_eq!(09_01_07_234, second_candle.start_time);
    }

    fn mock_trades_list() -> Vec<Trade> {
         vec![
            create_mock_trade(131175.0, 1, 200.0, 09_00_00), // open - first value for the first candle
            create_mock_trade(131170.0, 2, 200.0, 09_00_10),
            create_mock_trade(131010.0, 3, 200.0, 09_00_20), // min price
            create_mock_trade(131275.0, 4, 200.0, 09_00_30),
            create_mock_trade(131385.0, 5, 200.0, 09_00_40), // high price
            create_mock_trade(131095.0, 6, 200.0, 09_00_59), // close - last value for the first candle
            create_mock_trade(131000.0, 7, 200.0, 09_01_07_234), // open/low - second candle
            create_mock_trade(131085.0, 8, 200.0, 09_01_10),
            create_mock_trade(131105.0, 9, 200.0, 09_01_20),
            create_mock_trade(131115.0, 10, 200.0, 09_01_30),
            create_mock_trade(131200.0, 11, 200.0, 09_01_59),
            create_mock_trade(131210.0, 12, 200.0, 09_02_10),
            create_mock_trade(131215.0, 13, 200.0, 09_02_15),
            create_mock_trade(131225.0, 14, 200.0, 09_02_25),
        ]
    }

    fn create_mock_trade(price: f64, trade_number: i32, volume: f64, time: i32) -> Trade {
        Trade {
            trade_number,
            date: 2024_10_14,
            time,
            qty: 1,
            buy_agent: 2,
            sell_agent: 2,
            trade_type: 13,
            price,
            volume,
        }
    }
}

