use crate::models::candle::Candle;
use crate::models::extreme::Extreme;

fn directional_change_percentage(candle_feed: Vec<Candle>, threshold: f64) -> Vec<Extreme> {
    let mut is_high_flow = true;
    let mut tmp_price_min = candle_feed[0].lowest_price;
    let mut tmp_price_max = candle_feed[0].highest_price;
    let mut tmp_price_min_index = 0;
    let mut tmp_price_max_index = 0;

    let mut tops_and_bottoms: Vec<Extreme> = vec![];

    for (i, candle) in candle_feed.iter().enumerate() {
        let current_candle = candle;

        if is_high_flow {
            if current_candle.highest_price > tmp_price_max {
                tmp_price_max = current_candle.highest_price;
                tmp_price_max_index = i;
            } else if current_candle.closed_price < (tmp_price_max - (tmp_price_max * threshold)) {
                tops_and_bottoms.push(
                    Extreme {
                        confirmation_index: i as i32,
                        price_index: tmp_price_max_index as i32,
                        price: tmp_price_max,
                        height: None,
                        retrace_ratio: None,
                        is_top: true
                    }
                );

                is_high_flow = false;
                tmp_price_min = current_candle.lowest_price;
                tmp_price_min_index = i;
            }
        } else {
            if current_candle.lowest_price < tmp_price_min {
                tmp_price_min = current_candle.lowest_price;
                tmp_price_min_index = i;
            } else if current_candle.closed_price > (tmp_price_min + (tmp_price_min * threshold)) {
                tops_and_bottoms.push(
                    Extreme {
                        confirmation_index: i as i32,
                        price_index: tmp_price_min_index as i32,
                        price: tmp_price_min,
                        height: None,
                        retrace_ratio: None,
                        is_top: false
                    }
                );
                is_high_flow = true;
                tmp_price_min = current_candle.highest_price;
                tmp_price_min_index = i;
            }
        }
    }

    tops_and_bottoms
}
