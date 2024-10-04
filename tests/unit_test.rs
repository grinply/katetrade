

#[cfg(test)]
mod tests {

    #[derive(Clone, Debug, Copy)]
    pub struct Trade {
        pub trade_number: i32,
        pub price: f64,
        pub volume: f32,
        pub date: i32,
        pub time: i32,
        pub qty: i32,
        pub buy_agent: i8,
        pub sell_agent: i8,
        pub trade_type: i8,
    }

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

    fn harmonic_patterns() {}

    #[test]
    fn test_directional_change_percentage() {
        let trade_feed = vec![
            Trade { trade_number: 1, price: 100.0, volume: 1.0, date: 20231001, time: 90000, qty: 10, buy_agent: 1, sell_agent: 2, trade_type: 0 },
            Trade { trade_number: 2, price: 101.0, volume: 1.0, date: 20231001, time: 90100, qty: 15, buy_agent: 2, sell_agent: 1, trade_type: 0 },
            Trade { trade_number: 3, price: 102.0, volume: 1.0, date: 20231001, time: 90200, qty: 12, buy_agent: 1, sell_agent: 2, trade_type: 0 },
            Trade { trade_number: 4, price: 98.0, volume: 1.0, date: 20231001, time: 90300, qty: 18, buy_agent: 2, sell_agent: 1, trade_type: 0 },
            Trade { trade_number: 5, price: 99.0, volume: 1.0, date: 20231001, time: 90400, qty: 11, buy_agent: 1, sell_agent: 2, trade_type: 0 },
            Trade { trade_number: 6, price: 100.0, volume: 1.0, date: 20231001, time: 90500, qty: 11, buy_agent: 1, sell_agent: 2, trade_type: 0 },
        ];

        let threshold = 0.02;
        let tops_and_bottoms = directional_change_percentage(trade_feed, threshold);

        assert_eq!(tops_and_bottoms.len(), 3);

        assert_eq!(tops_and_bottoms[0].0, "Bottom");
        assert_eq!(tops_and_bottoms[0].1.price, 100.0);
        assert_eq!(tops_and_bottoms[0].2, 0);

        assert_eq!(tops_and_bottoms[1].0, "Top");
        assert_eq!(tops_and_bottoms[1].1.price, 102.0);
        assert_eq!(tops_and_bottoms[1].2, 2);

        assert_eq!(tops_and_bottoms[2].0, "Bottom");
        assert_eq!(tops_and_bottoms[2].1.price, 98.0);
        assert_eq!(tops_and_bottoms[2].2, 3);

    }


}