use crate::models::candlestick::Candlestick;

#[derive(Debug)]
#[allow(unused)]
pub struct Extreme {
    pub confirmation_index: i32, // Index that this extreme point was confirmed using directional change
    pub price_index: i32,        // Index of the extreme point is inside the Vec<Trade>
    pub price: f64,
    pub height: Option<f64>, // The height of the segment between two consecutive extreme points (absolute).
    pub retrace_ratio: Option<f64>, // The ratio of the current segment's height to the previous segment's height.
    pub is_top: bool,
}

#[allow(unused)]
impl Extreme {
    // Maybe generate these values in the for inside directional change for performance?
    fn generate_retracement_ratios(extremes: &mut Vec<Extreme>) {
        // temp to store new values
        let mut heights: Vec<Option<f64>> = vec![None; extremes.len()];
        let mut retrace_ratios: Vec<Option<f64>> = vec![None; extremes.len()];

        // first step: calculate heights e retrace_ratios
        for i in 1..extremes.len() {
            let current = &extremes[i];
            let previous = &extremes[i - 1];

            // calculate height with absolute diffrent between prices
            let height = (current.price - previous.price).abs();
            heights[i] = Some(height);

            // calculate retrace_ratio if earlier height exists
            if let Some(prev_height) = heights[i - 1] {
                retrace_ratios[i] = Some(height / prev_height);
            }
        }

        // second step: apply new values back to `extremes` vec
        for i in 1..extremes.len() {
            extremes[i].height = heights[i];
            extremes[i].retrace_ratio = retrace_ratios[i];
        }
    }
}


#[allow(unused)]
fn directional_change_percentage(candle_feed: Vec<Candlestick>, threshold: f64) -> Vec<Extreme> {
    let mut is_high_flow = true;
    let mut tmp_price_min = candle_feed[0].low;
    let mut tmp_price_max = candle_feed[0].high;
    let mut tmp_price_min_index = 0;
    let mut tmp_price_max_index = 0;

    let mut tops_and_bottoms: Vec<Extreme> = vec![];

    for (i, candle) in candle_feed.iter().enumerate() {
        let current_candle = candle;

        if is_high_flow {
            if current_candle.high > tmp_price_max {
                tmp_price_max = current_candle.high;
                tmp_price_max_index = i;
            } else if current_candle.close < (tmp_price_max - (tmp_price_max * threshold)) {
                tops_and_bottoms.push(Extreme {
                    confirmation_index: i as i32,
                    price_index: tmp_price_max_index as i32,
                    price: tmp_price_max,
                    height: None,
                    retrace_ratio: None,
                    is_top: true,
                });

                is_high_flow = false;
                tmp_price_min = current_candle.low;
                tmp_price_min_index = i;
            }
        } else {
            if current_candle.low < tmp_price_min {
                tmp_price_min = current_candle.low;
                tmp_price_min_index = i;
            } else if current_candle.close > (tmp_price_min + (tmp_price_min * threshold)) {
                tops_and_bottoms.push(Extreme {
                    confirmation_index: i as i32,
                    price_index: tmp_price_min_index as i32,
                    price: tmp_price_min,
                    height: None,
                    retrace_ratio: None,
                    is_top: false,
                });
                is_high_flow = true;
                tmp_price_min = current_candle.high;
                tmp_price_min_index = i;
            }
        }
    }

    tops_and_bottoms
}
