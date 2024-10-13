use crate::models::candle::Candle;
use crate::models::trade::Trade;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

fn to_naive_datetime(date: i32, time: i32) -> NaiveDateTime {
    NaiveDateTime::new(
        NaiveDate::parse_from_str(format!("{:}", date).as_str(), "%Y%m%d").unwrap(),
        NaiveTime::parse_from_str(format!("{:06}", time).as_str(), "%H%M%S").unwrap(),
    )
}

fn calculate_candles_by_period(trade_feed: Vec<Trade>, period: i64) -> Vec<Candle> {
    let mut first_trade = trade_feed[0];
    let mut first_candle_datetime = to_naive_datetime(first_trade.date, first_trade.time);

    let mut lowest_candle_price: f64 = first_trade.price;
    let mut highest_candle_price: f64 = first_trade.price;
    let mut volume_candle: f32 = 0.0;
    let mut current_price: f64 = first_trade.price;

    let mut candles: Vec<Candle> = Vec::new();

    for (i, trade) in trade_feed.iter().enumerate() {
        let current_datetime = to_naive_datetime(trade.date, trade.time);

        if current_datetime
            .signed_duration_since(first_candle_datetime)
            .num_seconds()
            >= period
        {
            // Candle closed, push it.
            candles.push(Candle {
                lowest_price: lowest_candle_price,
                highest_price: highest_candle_price,
                volume: volume_candle,
                closed_price: current_price,
                initial_date: first_trade.date,
                initial_time: first_trade.time,
                period,
                closed: true,
            });

            // Reset variables for new candle
            first_trade = *trade;
            first_candle_datetime = to_naive_datetime(first_trade.date, first_trade.time);
            lowest_candle_price = trade.price;
            highest_candle_price = trade.price;
            volume_candle = 0.0;
        }

        current_price = trade.price;
        volume_candle += trade.volume;

        if current_price < lowest_candle_price {
            lowest_candle_price = current_price;
        }

        if current_price > highest_candle_price {
            highest_candle_price = current_price;
        }

        // in case this is the last trade, closes the candlestick
        if trade_feed.len() == (i + 1) {
            candles.push(Candle {
                lowest_price: lowest_candle_price,
                highest_price: highest_candle_price,
                volume: volume_candle,
                closed_price: current_price,
                initial_date: first_trade.date,
                initial_time: first_trade.time,
                period,
                closed: false,
            });
        }
    }

    candles
}
