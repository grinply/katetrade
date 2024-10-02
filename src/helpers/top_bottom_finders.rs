use crate::models::trade::Trade;

fn directional_change_percentage(trade_feed: Vec<Trade>, threshold: f64) -> Vec<(String, f64, usize)> {
    let mut is_high_flow = true;
    let mut tmp_min = trade_feed[0].price;
    let mut tmp_max = trade_feed[0].price;
    let mut tmp_min_index = 0;
    let mut tmp_max_index = 0;
    
    let mut tops_and_bottoms: Vec<(String, f64, usize)> = Vec::new();

    for (i, trade) in trade_feed.iter().enumerate() {
        let current_price = trade.price;

        if is_high_flow {
            if current_price <= tmp_min {
                tmp_min = current_price;
                tmp_min_index = i;
            } else if current_price >= tmp_min * (1.0 + threshold) {
                // Confirmed bottom
                tops_and_bottoms.push(("Bottom".to_string(), tmp_min, tmp_min_index));

                // Switch to low flow
                is_high_flow = false;
                tmp_max = current_price;
                tmp_max_index = i;
            }
        } else {
            if current_price >= tmp_max {
                tmp_max = current_price;
                tmp_max_index = i;
            } else if current_price <= tmp_max * (1.0 - threshold) {
                // Confirmed top
                tops_and_bottoms.push(("Top".to_string(), tmp_max, tmp_max_index));

                // Switch to high flow
                is_high_flow = true;
                tmp_min = current_price;
                tmp_min_index = i;
            }
        }
    }

    tops_and_bottoms
}

fn directional_change_atr(trade_feed: Vec<Trade>, threshold: f64) -> Vec<(String, f64, usize)> {
    unimplemented!();
}