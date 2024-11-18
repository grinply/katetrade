use strum_macros::EnumString;
use ta::{Close, High, Low, Open, Volume};
use crate::models::trade::Trade;
use crate::strategies::strategy::CandlestickConfiguration;

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
    pub close_time: i32,
    pub is_closed: bool,
    pub trades: u64,
}

#[derive(Debug, PartialEq, Eq, Clone, EnumString)]
#[allow(unused)]
pub enum CandlestickType {
    TIME,
    TICK,
    VOLUME,
}

#[allow(unused)]
pub trait CandlestickGenerator {
    fn description(&self) -> String;

    /// generates a brand-new Candlestick for every new trade, representing the still open candle
    /// it and also the closed when they finally do close.
    fn parse_with_open_candles(&mut self, new_trade: &Trade) -> Candlestick;

    /// generates a new Candlestick only the candlestick is closed
    /// it will only update its internal state with the new_trade in the meantime
    fn parse_only_closed_candles(&mut self, new_trade: &Trade) -> Option<Candlestick>;
}

#[allow(unused)]
pub struct TimeCandlestickGenerator {
    last_day: i32,
    last_price: f64,
    period_minutes: u8,
    last_candle: Option<Candlestick>,
}

#[allow(unused)]
pub struct TickCandlestickGenerator {
    ticks_per_candle: u64,
    last_candle: Option<Candlestick>,
    date_last_trade: i32,
}

#[allow(unused)]
pub struct VolumeCandlestickGenerator {
    volume_per_candle: f64,
    last_candle: Option<Candlestick>,
}

pub fn select_candlestick_generator(configuration: CandlestickConfiguration) -> Box<dyn CandlestickGenerator> {
    match configuration.candle_type {
        CandlestickType::TIME => Box::new(TimeCandlestickGenerator::new(configuration.candle_input_parameter)),
        CandlestickType::TICK => Box::new(TickCandlestickGenerator::new(configuration.candle_input_parameter)),
        CandlestickType::VOLUME => Box::new(TickCandlestickGenerator::new(configuration.candle_input_parameter))
    }
}

// implementing OHLCV trait from technical analysis library to use existing simple_moving_average
impl Open for Candlestick {
    fn open(&self) -> f64 {
        self.open
    }
}

impl High for Candlestick {
    fn high(&self) -> f64 {
        self.high
    }
}

impl Low for Candlestick {
    fn low(&self) -> f64 {
        self.low
    }
}

impl Close for Candlestick {
    fn close(&self) -> f64 {
        self.close
    }
}

impl Volume for Candlestick {
    fn volume(&self) -> f64 {
        self.volume
    }
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
            close_time: -1,
            is_closed: false,
            trades: 1,
        }
    }
}

impl TickCandlestickGenerator {
    fn new(input_configuration: u64) -> impl CandlestickGenerator {
        TickCandlestickGenerator {
            ticks_per_candle: input_configuration,
            last_candle: None,
            date_last_trade: -1,
        }
    }
}

impl CandlestickGenerator for TickCandlestickGenerator {
    fn description(&self) -> String {
        "Candlestick grouped by quantity of trades".to_string()
    }

    fn parse_with_open_candles(&mut self, new_trade: &Trade) -> Candlestick {
        if self.last_candle.is_none() {
            self.last_candle = Some(Candlestick::new(new_trade));
            self.date_last_trade = new_trade.date;
            return self.last_candle.as_ref().unwrap().clone();
        }

        if new_trade.date != self.date_last_trade {
            let result_candle = self.last_candle.as_ref().unwrap().clone();
            self.last_candle = Some(Candlestick::new(new_trade));
            return result_candle.clone();
        }

        let existing_candle = self.last_candle.as_mut().unwrap();
        existing_candle.close_time = new_trade.time;

        existing_candle.volume += new_trade.volume;

        if new_trade.price < existing_candle.low {
            existing_candle.low = new_trade.price;
        }
        if new_trade.price > existing_candle.high {
            existing_candle.high = new_trade.price;
        }

        existing_candle.close = new_trade.price;
        existing_candle.trades += 1;

        let mut result_candle = existing_candle.clone();

        if existing_candle.trades < self.ticks_per_candle {
            result_candle.is_closed = false;
        } else {
            result_candle.is_closed = true;
            self.last_candle = None;
        }

        result_candle
    }

    fn parse_only_closed_candles(&mut self, new_trade: &Trade) -> Option<Candlestick> {
        let candle = self.parse_with_open_candles(new_trade);
        if candle.is_closed {
            Some(candle)
        } else {
            None
        }
    }
}

impl CandlestickGenerator for TimeCandlestickGenerator {


    fn description(&self) -> String {
        String::from(format!("candlesticks based on time slices of {} minutes", self.period_minutes))
    }

    fn parse_with_open_candles(&mut self, new_trade: &Trade) -> Candlestick {
        TimeCandlestickGenerator::parse(self, new_trade, false).unwrap()
    }

    fn parse_only_closed_candles(&mut self, new_trade: &Trade) -> Option<Candlestick> {
        TimeCandlestickGenerator::parse(self, new_trade, true)
    }
}


impl TimeCandlestickGenerator {
    fn new(input_configuration: u64) -> impl CandlestickGenerator {
        if input_configuration == 0 || input_configuration > 60 {
            panic!("Invalid candlestick period, the period must be between 1 and 60 minutes");
        }
        TimeCandlestickGenerator {
            last_price: 0.0,
            last_day: 0,
            period_minutes: input_configuration as u8,
            last_candle: None,
        }
    }

    fn parse(&mut self, new_trade: &Trade, only_closed_candles: bool) -> Option<Candlestick> {
        if new_trade.price < 0.0 {
            panic!("Trade price should be greater than 0, received {}", new_trade.price);
        }

        // if the day rolls over we close the current candle
        if self.last_day != 0 && self.last_day != new_trade.date {
            self.last_day = new_trade.date;
            self.last_price = new_trade.price;
            let mut new_candle = Candlestick::new(new_trade);
            self.add_close_time_to_time_based_candlestick(&mut new_candle);
            self.remove_seconds_from_start_time(&mut new_candle);
            self.last_candle = Some(new_candle);
            return Some(self.last_candle.clone()?);
        }

        self.last_day = new_trade.date;

        if self.last_candle.is_none() { // the very start when no trade was parsed yet
            self.last_price = new_trade.price;
            let mut new_candle = Candlestick::new(new_trade);
            self.remove_seconds_from_start_time(&mut new_candle);
            self.add_close_time_to_time_based_candlestick(&mut new_candle);
            self.last_candle = Some(new_candle.clone());

            return if only_closed_candles {
                return None;
            } else {
                Some(new_candle)
            };
        }
        let existing_candle = self.last_candle.as_mut()?;

        // time is represented as a number with format HHmmSS or HHmmSSsss
        // we do some math to calculate to make all values beyond the minute zero values to better
        // calculate the candlestick close
        let digits_to_remove_current_time: u32 = if new_trade.time < 99_99_99 {
            2
        } else {
            5
        };

        let digits_to_remove_start_time: u32 = if existing_candle.start_time < 99_99_99 {
            2
        } else {
            5
        };

        let shift_current_time: i32 = 10i32.pow(digits_to_remove_current_time);
        let shift_start_time: i32 = 10i32.pow(digits_to_remove_start_time);

        let is_candle_closed = (new_trade.time / shift_current_time) >= (existing_candle.start_time / shift_start_time) + (self.period_minutes as i32);

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
            let mut new_candle = Candlestick::new(new_trade);
            self.add_close_time_to_time_based_candlestick(&mut new_candle);
            self.remove_seconds_from_start_time(&mut new_candle);
            self.last_candle = Some(new_candle);
        }
        Some(result_candle)
    }

    fn remove_seconds_from_start_time(&self, mut candlestick: &mut Candlestick){
        if candlestick.start_time < 99_99_99 {
            candlestick.start_time = (candlestick.start_time / 100) * 100;
        }else{
            candlestick.start_time = (candlestick.start_time / 100000) * 100000;
        }
    }

    fn add_close_time_to_time_based_candlestick(&self, candlestick: &mut Candlestick){
        if candlestick.start_time < 99_99_99 {
            candlestick.close_time = candlestick.start_time + self.period_minutes as i32 * 100;
        }else{
            candlestick.close_time = candlestick.start_time + self.period_minutes as i32 * 100000;
        }
    }
}





#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::models::trade::TradeFile;
    use super::*;

    #[test]
    fn parse_correct_amount_of_time_based_candlesticks() {
        let binary_trades_path = PathBuf::from("resources/TRADE-B3-FUTURES-WIN-20150102");

        assert!(binary_trades_path.exists());

        let trades_file = TradeFile::new_from_binary_file(binary_trades_path.as_path().to_str().unwrap())
            .unwrap();

        let mut generator = TimeCandlestickGenerator::new(1);
        let mut candlesticks: Vec<Candlestick> = vec![];

        for trade in trades_file {
            if let Some(candle) = generator.parse_only_closed_candles(&trade) {
                candlesticks.push(candle);
            }
        }
        assert_eq!(candlesticks.first().unwrap().date, 2015_01_02);
        assert_eq!(candlesticks.first().unwrap().start_time, 09_00_00);

        assert_eq!(candlesticks.last().unwrap().date, 2015_01_02);
        assert_eq!(candlesticks.last().unwrap().start_time, 17_54_02);
        assert_eq!(535, candlesticks.len());
    }

    #[test]
    fn time_candlesticks_generator_should_correctly_create_candles_of_one_minute_including_not_closed_candles() {
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

    #[test]
    fn time_candlesticks_generator_handles_day_change() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = vec![
            create_mock_trade(131175.0, 1, 200.0, 23_59_50, 2024_10_14),
            create_mock_trade(131180.0, 2, 200.0, 23_59_59, 2024_10_14),
            create_mock_trade(131185.0, 3, 200.0, 00_00_05, 2024_10_15), // Next day
        ];
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            let new_candle = generator.parse_with_open_candles(&trade);
            result_candlesticks.push(new_candle);
        }

        // Verify that a new candle started after midnight
        assert_eq!(result_candlesticks[2].date, 2024_10_15);
        assert_eq!(result_candlesticks[2].start_time, 00_00_00);
    }

    #[test]
    fn time_candlesticks_generator_handles_boundary_trades() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = vec![
            create_mock_trade(131000.0, 1, 200.0, 09_00_59, 2024_10_14),
            create_mock_trade(131005.0, 2, 200.0, 09_01_00, 2024_10_14), // Exactly at boundary
        ];
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            let new_candle = generator.parse_with_open_candles(&trade);
            result_candlesticks.push(new_candle);
        }

        // Verify if the trade at 09:01:00 starts a new candle
        assert!(result_candlesticks[1].is_closed);
        assert_eq!(result_candlesticks[1].start_time, 09_00_00);
        assert_eq!(result_candlesticks[1].close_time, 09_01_00);
    }

    #[test]
    fn time_candlesticks_generator_handles_no_trades_in_period() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = vec![
            create_mock_trade(131000.0, 1, 200.0, 09_00_00, 2024_10_14),
            // Gap of 2 minutes with no trades
            create_mock_trade(131050.0, 2, 200.0, 09_02_00, 2024_10_14),
        ];
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            if let Some(candle) = generator.parse_only_closed_candles(&trade) {
                result_candlesticks.push(candle);
            }
        }

        // Verify that the generator does not create empty candles for periods with no trades
        assert_eq!(result_candlesticks.len(), 1);
    }

    #[test]
    #[should_panic(expected = "Trade price should be greater than 0, received -131000")]
    fn time_candlesticks_generator_handles_invalid_trades() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = vec![
            create_mock_trade(-131000.0, 1, 200.0, 09_00_00, 2024_10_14), // Negative price
        ];

        for trade in &trades {
            generator.parse_with_open_candles(&trade);
        }
    }

    #[test]
    fn time_candlesticks_generator_handles_high_volume() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let mut trades = vec![];
        for i in 0..10_000 {
            trades.push(create_mock_trade(
                131000.0 + (i as f64 * 0.01),
                i,
                200.0,
                09_00_00 + (i % 60),
                2024_10_14,
            ));
        }
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            let candle = generator.parse_with_open_candles(&trade);
            result_candlesticks.push(candle);
        }
        assert_eq!(10_000, result_candlesticks.len());
    }

    #[test]
    fn time_candlesticks_generator_handles_extreme_fluctuations() {
        let mut generator = TimeCandlestickGenerator::new(1);
        let trades = vec![
            create_mock_trade(131000.0, 1, 200.0, 09_00_00, 2024_10_14),
            create_mock_trade(131500.0, 2, 200.0, 09_00_15, 2024_10_14),
            create_mock_trade(130500.0, 3, 200.0, 09_00_30, 2024_10_14),
            create_mock_trade(131200.0, 4, 200.0, 09_00_45, 2024_10_14),
        ];
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            let new_candle = generator.parse_with_open_candles(&trade);
            result_candlesticks.push(new_candle);
        }

        let candle = result_candlesticks.last().unwrap();

        assert_eq!(candle.high, 131500.0);
        assert_eq!(candle.low, 130500.0);
    }

    #[test]
    fn time_candlesticks_generator_with_five_minute_period() {
        let mut generator =TimeCandlestickGenerator {
            last_price: 0.0,
            last_day: 0,
            period_minutes: 5,
            last_candle: None,
        };

        let trades = mock_trades_list_for_five_minute_period(); // Get the mock trades data
        let mut result_candlesticks: Vec<Candlestick> = vec![];

        for trade in &trades {
            // Use parse_only_closed_candles to get only closed candles
            if let Some(candle) = generator.parse_only_closed_candles(&trade) {
                result_candlesticks.push(candle);
            }
        }

        // We expect two closed candles for the first two 5-minute periods
        assert_eq!(
            result_candlesticks.len(),
            2,
            "Should have 2 closed candles for the 5-minute periods"
        );

        // Verify the first closed candle (09:00:00 - 09:04:59)
        let first_candle = &result_candlesticks[0];
        assert_eq!(first_candle.is_closed, true);
        assert_eq!(first_candle.start_time, 09_00_00);
        assert_eq!(first_candle.open, 131000.0);
        assert_eq!(first_candle.high, 131075.0);
        assert_eq!(first_candle.low, 131000.0);
        assert_eq!(first_candle.close, 131060.0);
        assert_eq!(first_candle.trades, 5);
        assert_eq!(
            first_candle.volume,
            100.0 + 150.0 + 120.0 + 130.0 + 110.0
        );

        // Verify the second closed candle (09:05:00 - 09:09:59)
        let second_candle = &result_candlesticks[1];
        assert_eq!(second_candle.is_closed, true);
        assert_eq!(second_candle.start_time, 09_05_10);
        assert_eq!(second_candle.open, 131100.0);
        assert_eq!(second_candle.high, 131130.0);
        assert_eq!(second_candle.low, 131080.0);
        assert_eq!(second_candle.close, 131090.0);
        assert_eq!(second_candle.trades, 5);
        assert_eq!(
            second_candle.volume,
            200.0 + 220.0 + 210.0 + 230.0 + 240.0
        );

        // The third period (09:10:00 - 09:14:59) is not closed yet
        // Verify that the last candle is open and has correct data
        if let Some(last_candle) = &generator.last_candle {
            assert_eq!(last_candle.is_closed, false);
            assert_eq!(last_candle.start_time, 09_10_15);
            assert_eq!(last_candle.open, 131110.0);
            assert_eq!(last_candle.high, 131150.0);
            assert_eq!(last_candle.low, 131070.0);
            assert_eq!(last_candle.close, 131085.0);
            assert_eq!(last_candle.trades, 5);
            assert_eq!(
                last_candle.volume,
                300.0 + 310.0 + 320.0 + 330.0 + 340.0
            );
        } else {
            panic!("Expected a last open candle");
        }
    }

    // Mock trades data for the test case
    fn mock_trades_list_for_five_minute_period() -> Vec<Trade> {
        vec![
            // Trades in the first 5-minute period (09:00:00 - 09:04:59)
            create_mock_trade(131000.0, 1, 100.0, 09_00_00, 2024_10_14),
            create_mock_trade(131050.0, 2, 150.0, 09_01_00, 2024_10_14),
            create_mock_trade(131025.0, 3, 120.0, 09_02_30, 2024_10_14),
            create_mock_trade(131075.0, 4, 130.0, 09_03_45, 2024_10_14),
            create_mock_trade(131060.0, 5, 110.0, 09_04_50, 2024_10_14),

            // Trades in the second 5-minute period (09:05:00 - 09:09:59)
            create_mock_trade(131100.0, 6, 200.0, 09_05_10, 2024_10_14),
            create_mock_trade(131120.0, 7, 220.0, 09_06_20, 2024_10_14),
            create_mock_trade(131080.0, 8, 210.0, 09_07_30, 2024_10_14),
            create_mock_trade(131130.0, 9, 230.0, 09_08_40, 2024_10_14),
            create_mock_trade(131090.0, 10, 240.0, 09_09_50, 2024_10_14),

            // Trades in the third period (09:10:00 - 09:14:59), candle is still open
            create_mock_trade(131110.0, 11, 300.0, 09_10_15, 2024_10_14),
            create_mock_trade(131140.0, 12, 310.0, 09_11_25, 2024_10_14),
            create_mock_trade(131070.0, 13, 320.0, 09_12_35, 2024_10_14),
            create_mock_trade(131150.0, 14, 330.0, 09_13_45, 2024_10_14),
            create_mock_trade(131085.0, 15, 340.0, 09_14_55, 2024_10_14),
        ]
    }

    fn mock_trades_list() -> Vec<Trade> {
        vec![
            create_mock_trade(131175.0, 1, 200.0, 09_00_00, 2024_10_14), // open - first value for the first candle
            create_mock_trade(131170.0, 2, 200.0, 09_00_10, 2024_10_14),
            create_mock_trade(131010.0, 3, 200.0, 09_00_20, 2024_10_14), // min price
            create_mock_trade(131275.0, 4, 200.0, 09_00_30, 2024_10_14),
            create_mock_trade(131385.0, 5, 200.0, 09_00_40, 2024_10_14), // high price
            create_mock_trade(131095.0, 6, 200.0, 09_00_59, 2024_10_14), // close - last value for the first candle
            create_mock_trade(131000.0, 7, 200.0, 09_01_07_234, 2024_10_14), // open/low - second candle
            create_mock_trade(131085.0, 8, 200.0, 09_01_10, 2024_10_14),
            create_mock_trade(131105.0, 9, 200.0, 09_01_20, 2024_10_14),
            create_mock_trade(131115.0, 10, 200.0, 09_01_30, 2024_10_14),
            create_mock_trade(131200.0, 11, 200.0, 09_01_59, 2024_10_14),
            create_mock_trade(131210.0, 12, 200.0, 09_02_10, 2024_10_14),
            create_mock_trade(131215.0, 13, 200.0, 09_02_15, 2024_10_14),
            create_mock_trade(131225.0, 14, 200.0, 09_02_25, 2024_10_14),
        ]
    }

    fn create_mock_trade(price: f64, trade_number: i32, volume: f64, time: i32, date: i32) -> Trade {
        Trade {
            trade_number,
            date,
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

