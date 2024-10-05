

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::iter::Map;

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

    #[derive(Debug, Clone)]
    struct Extreme {
        pub confirmation_index: i32, // Index that this extreme point was confirmed using directional change
        pub price_index: i32, // Index of the extreme point is inside the Vec<Trade>
        pub trade: Trade,
        pub height: Option<f64>, // The height of the segment between two consecutive extreme points (absolute).
        pub retrace_ratio: Option<f64>, // The ratio of the current segment's height to the previous segment's height.
        pub is_top: bool
    }


    impl Extreme {

        // Maybe generate these values in the for inside directional change for performance?
        fn generate_retracement_ratios(extremes: &mut Vec<Extreme>) {
            // Temporários para armazenar os novos valores
            let mut heights: Vec<Option<f64>> = vec![None; extremes.len()];
            let mut retrace_ratios: Vec<Option<f64>> = vec![None; extremes.len()];

            // Primeiro passo: calcular heights e retrace_ratios
            for i in 1..extremes.len() {
                let current = &extremes[i];
                let previous = &extremes[i - 1];

                // Calcular height como a diferença absoluta nos preços
                let height = (current.trade.price - previous.trade.price).abs() as f64;
                heights[i] = Some(height);

                // Calcular retrace_ratio se o height anterior existir
                if let Some(prev_height) = heights[i - 1] {
                    retrace_ratios[i] = Some(height / prev_height);
                }
            }

            // Segundo passo: aplicar os novos valores de volta ao vetor `extremes`
            for i in 1..extremes.len() {
                extremes[i].height = heights[i];
                extremes[i].retrace_ratio = retrace_ratios[i];
            }
        }
    }

    #[derive()]
    pub struct TriangleDistances {
        pub xb_distance: [f64; 2],
        pub ac_distance: [f64; 2],
        pub bd_distance: [f64; 2],
        pub xd_distance: [f64; 2],
        name: String
    }

    pub struct HarmonicPatterns {
        pub gartley: TriangleDistances,
        pub bat: TriangleDistances,
        pub butterfly: TriangleDistances,
        pub crab: TriangleDistances,
        pub deep_crab: TriangleDistances,
        pub cypher: TriangleDistances,
        pub shark: TriangleDistances,
    }

    impl HarmonicPatterns {
        pub fn new() -> HarmonicPatterns {
            HarmonicPatterns {
                gartley: TriangleDistances {
                    xb_distance: [0.618, -1.0],
                    ac_distance: [0.382, 0.886],
                    bd_distance: [1.13, 1.618],
                    xd_distance: [0.786, -1.0],
                    name: "Gartley".to_string(),
                },
                bat: TriangleDistances {
                    xb_distance: [0.382, 0.50],
                    ac_distance: [0.382, 0.886],
                    bd_distance: [1.618, 2.618],
                    xd_distance: [0.886, -1.0],
                    name: "Bat".to_string(),
                },
                butterfly: TriangleDistances {
                    xb_distance: [0.786, -1.0],
                    ac_distance: [0.382, 0.886],
                    bd_distance: [1.618, 2.24],
                    xd_distance: [1.27, 1.41],
                    name: "Butterfly".to_string(),
                },
                crab: TriangleDistances {
                    xb_distance: [0.382, 0.618],
                    ac_distance: [0.382, 0.886],
                    bd_distance: [2.618, 3.618],
                    xd_distance: [1.618, -1.0],
                    name: "Crab".to_string(),
                },
                deep_crab: TriangleDistances {
                    xb_distance: [0.886, -1.0],
                    ac_distance: [0.382, 0.886],
                    bd_distance: [2.0, 3.618],
                    xd_distance: [1.618, -1.0],
                    name: "DeepCrab".to_string(),
                },
                cypher: TriangleDistances {
                    xb_distance: [0.382, 0.618],
                    ac_distance: [1.13, 1.41],
                    bd_distance: [1.27, 2.00],
                    xd_distance: [0.786, -1.0],
                    name: "Cypher".to_string(),
                },
                shark: TriangleDistances {
                    xb_distance: [-1.0, -1.0],
                    ac_distance: [1.13, 1.618],
                    bd_distance: [1.618, 2.24],
                    xd_distance: [0.886, 1.13],
                    name: "Shark".to_string(),
                },
            }
        }

        pub fn iter(&self) -> Vec<&TriangleDistances> {
            vec![
                &self.gartley,
                &self.bat,
                &self.butterfly,
                &self.crab,
                &self.deep_crab,
                &self.cypher,
                &self.shark,
            ]
        }
    }

    fn directional_change_percentage(sorted_trade_feed: Vec<Trade>, threshold: f64) -> Vec<Extreme> {
        let mut is_high_flow = true;
        let mut tmp_trade_min = sorted_trade_feed[0].clone();
        let mut tmp_trade_max = sorted_trade_feed[0].clone();
        let mut tmp_trade_min_index = 0;
        let mut tmp_trade_max_index = 0;

        let mut tops_and_bottoms: Vec<Extreme> = vec![];

        for (i, trade) in sorted_trade_feed.iter().enumerate() {
            let current_trade = trade;

            if is_high_flow {
                if current_trade.price <= tmp_trade_min.price {
                    tmp_trade_min = *current_trade;
                    tmp_trade_min_index = i;
                } else if current_trade.price >= tmp_trade_min.price * (1.0 + threshold) {
                    tops_and_bottoms.push(
                        Extreme {
                            confirmation_index: i as i32,
                            price_index: tmp_trade_min_index as i32,
                            trade: tmp_trade_min,
                            height: Option::None,
                            retrace_ratio: Option::None,
                            is_top: false}
                    );

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
                    tops_and_bottoms.push(
                        Extreme {
                            confirmation_index: i as i32,
                            price_index: tmp_trade_max_index as i32,
                            trade: tmp_trade_max,
                            height: Option::None,
                            retrace_ratio: Option::None,
                            is_top: true}
                    );

                    // Switch to high flow
                    is_high_flow = true;
                    tmp_trade_min = *current_trade;
                    tmp_trade_min_index = i;
                }
            }
        }

        tops_and_bottoms
    }

    fn get_error(value1, value2) -> f64 {
        return 0.0
    }
    fn find_harmonic_patterns(sorted_trade_feed: Vec<Trade>, sorted_extremes: Vec<Extreme>, error_threshold: f64) {
    // fn find_harmonic_patterns() {
        let harmonic_patterns = HarmonicPatterns::new();
        let mut output: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();

        for &pattern in harmonic_patterns.iter() {
            println!("{:?}: xb_distance {:?} {:?} ac_distance {:?} {:?} bd_distance {:?} {:?} xd_distance {:?} {:?}",
                     pattern.name,
                     pattern.xb_distance[0],
                     pattern.xb_distance[1],
                     pattern.ac_distance[0],
                     pattern.ac_distance[1],
                     pattern.bd_distance[0],
                     pattern.bd_distance[1],
                     pattern.xd_distance[0],
                     pattern.xd_distance[1]);

            let mut data: HashMap<String, Vec<f64>> = HashMap::new();

            data.insert("bull_signal".to_string(), vec![0.0; sorted_trade_feed.len()]);
            data.insert("bull_patterns".to_string(), Vec::new());
            data.insert("bear_signal".to_string(), vec![0.0; sorted_trade_feed.len()]);
            data.insert("bear_patterns".to_string(), Vec::new());

            // Insert the pattern data into the output HashMap
            output.insert(pattern.name.clone(), data);

        }

        // Get the first extreme to iterate
        let mut current_extreme = sorted_extremes.get(0).unwrap();
        let mut current_extreme_index = 0;

        let mut bull_signal = false;
        let mut bear_signal = false;
        let mut pattern_used = None;

        for i in current_extreme_index..sorted_trade_feed.len() {
            if (current_extreme_index + 1) == i {
                bull_signal = false;
                bear_signal = false;
                current_extreme_index += 1;
            }

            if bull_signal {
                pattern_used
                    .as_ref()
                    .and_then(|pattern| output.get_mut(pattern))
                    .and_then(|pat_data| pat_data.get_mut("bull_signal"))
                    .map(|bull_signal| bull_signal[i] = 1.0); // Set the bull_signal at index i to 1
            }

            if bear_signal {
                pattern_used
                    .as_ref()
                    .and_then(|pattern| output.get_mut(pattern))
                    .and_then(|pat_data| pat_data.get_mut("bear_signal"))
                    .map(|bear_signal| bear_signal[i] = -1.0); // Set the bear_signal at index i to -1
            }

            if current_extreme_index + 1 >= sorted_extremes.len() {
                break
            }

            if current_extreme_index < 3 {
                continue;
            }

            if sorted_extremes.get(current_extreme_index).unwrap().is_top {
                // Calculate D_Price using sorted_trade_feed
                // Verify if the minimum value of sorted_trade_feed between current_extreme_index and i is lower than D_Price.
                // That will confirm the lowest since last confirmed top
            } else {
                // Calculate D_Price using sorted_trade_feed
                // Verify if the maximum value of sorted_trade_feed between current_extreme_index and i is higher than D_Price.
                // That will confirm the highest since confirmed bottom
            }

            let dc_retrace = 0.0; // abs(D_price - extremes.iloc[extreme_i]['ext_p']) / extremes.iloc[extreme_i]['seg_height']
            let xd_distance_retrace = 0.0 // abs(D_price - extremes.iloc[extreme_i - 2]['ext_p']) / extremes.iloc[extreme_i - 2]['seg_height']

            let mut best_error_ratio = 0.0;
            let mut best_pattern: Option<TriangleDistances> = None;

            for &pattern in harmonic_patterns.iter() {
                let mut error_ratio = 0.0;
                error_ratio += get_error(sorted_extremes.get(current_extreme_index).unwrap().retrace_ratio, pattern.ac_distance);
                error_ratio += get_error(sorted_extremes.get(current_extreme_index-1).unwrap().retrace_ratio, pattern.xb_distance);
                error_ratio += get_error(dc_retrace, pattern.bd_distance);
                error_ratio += get_error(xd_distance_retrace, pattern.xd_distance);
                if error_ratio < best_error_ratio {
                    best_error_ratio = error_ratio;
                    best_pattern = Some(pattern);
                }
            }

            if best_error_ratio <= error_threshold {
                // set best pattern used
                // set map of indexes
                // set if it's a bull or a bear
                // make market calls
            }
        }
    }

    #[test]
    fn test_find_harmonic_patterns() {
        let trade_feed = vec![
            Trade { trade_number: 1, price: 100.0, volume: 1.0, date: 20231001, time: 90000, qty: 10, buy_agent: 1, sell_agent: 2, trade_type: 0 },
            Trade { trade_number: 2, price: 101.0, volume: 1.0, date: 20231001, time: 90100, qty: 15, buy_agent: 2, sell_agent: 1, trade_type: 0 },
            Trade { trade_number: 3, price: 102.0, volume: 1.0, date: 20231001, time: 90200, qty: 12, buy_agent: 1, sell_agent: 2, trade_type: 0 },
            Trade { trade_number: 4, price: 98.0, volume: 1.0, date: 20231001, time: 90300, qty: 18, buy_agent: 2, sell_agent: 1, trade_type: 0 },
            Trade { trade_number: 5, price: 99.0, volume: 1.0, date: 20231001, time: 90400, qty: 11, buy_agent: 1, sell_agent: 2, trade_type: 0 },
            Trade { trade_number: 6, price: 100.0, volume: 1.0, date: 20231001, time: 90500, qty: 11, buy_agent: 1, sell_agent: 2, trade_type: 0 },
        ];

        let threshold = 0.02;
        let mut extremes = directional_change_percentage(trade_feed, threshold);
        Extreme::generate_retracement_ratios(&mut extremes);

        // find_harmonic_patterns(trade_feed, extremes, 0.01);
    }
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
        let mut tops_and_bottoms = directional_change_percentage(trade_feed, threshold);
        Extreme::generate_retracement_ratios(&mut tops_and_bottoms);

        assert_eq!(tops_and_bottoms.len(), 3);

        assert_eq!(tops_and_bottoms[0].is_top, false);
        assert_eq!(tops_and_bottoms[0].trade.price, 100.0);
        assert_eq!(tops_and_bottoms[0].confirmation_index, 2);
        assert_eq!(tops_and_bottoms[0].price_index, 0);
        assert_eq!(tops_and_bottoms[0].height, None);
        assert_eq!(tops_and_bottoms[0].retrace_ratio, None);

        assert_eq!(tops_and_bottoms[1].is_top, true);
        assert_eq!(tops_and_bottoms[1].trade.price, 102.0);
        assert_eq!(tops_and_bottoms[1].confirmation_index, 3);
        assert_eq!(tops_and_bottoms[1].price_index, 2);
        assert_eq!(tops_and_bottoms[1].height, Some(2.0));
        assert_eq!(tops_and_bottoms[1].retrace_ratio, None);

        assert_eq!(tops_and_bottoms[2].is_top, false);
        assert_eq!(tops_and_bottoms[2].trade.price, 98.0);
        assert_eq!(tops_and_bottoms[2].confirmation_index, 5);
        assert_eq!(tops_and_bottoms[2].price_index, 3);
        assert_eq!(tops_and_bottoms[2].height, Some(4.0));
        assert_eq!(tops_and_bottoms[2].retrace_ratio, Some(2.0));

    }


}