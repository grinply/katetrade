use crate::models::trade::Trade;
fn directional_change_percentage(trade_feed: Vec<Trade>, threshold: f64) -> Vec<(String, Trade, usize)> {
    let mut is_high_flow = true;
    let mut tmp_trade_min = trade_feed[0].clone();
    let mut tmp_trade_max = trade_feed[0].clone();
    let mut tmp_trade_min_index = 0;
    let mut tmp_trade_max_index = 0;

    let mut tops_and_bottoms: Vec<(String, Trade, usize)> = Vec::new();

    for (i, trade) in trade_feed.iter().enumerate() {
        let current_trade = trade;

        if is_high_flow {
            if current_trade.price <= tmp_trade_min.price {
                tmp_trade_min = *current_trade;
                tmp_trade_min_index = i;
            } else if current_trade.price >= tmp_trade_min.price * (1.0 + threshold) {
                // Confirmed bottom
                tops_and_bottoms.push(("Bottom".to_string(), tmp_trade_min, tmp_trade_min_index));

                // Switch to low flow
                is_high_flow = false;
                tmp_trade_max = *current_trade;
                tmp_trade_max_index = i;
            }
        } else {
            if current_trade.price >= tmp_trade_max.price {
                tmp_trade_max = *current_trade;
                tmp_trade_max_index = i;
            } else if current_trade.price <= tmp_trade_max.price * (1.0 - threshold) {
                // Confirmed top
                tops_and_bottoms.push(("Top".to_string(), tmp_trade_max, tmp_trade_max_index));

                // Switch to high flow
                is_high_flow = true;
                tmp_trade_min = *current_trade;
                tmp_trade_min_index = i;
            }
        }
    }

    tops_and_bottoms
}

fn directional_change_atr(trade_feed: Vec<Trade>, threshold: f64) -> Vec<(String, f64, usize)> {
    unimplemented!();
}