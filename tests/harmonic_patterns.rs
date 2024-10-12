


#[cfg(test)]
mod tests {
    use std::cmp::min;
    use std::collections::HashMap;

    pub struct HarmonicFound {
        pub extreme_points: [i32; 4],
        pub total_error: f64,
        pub pattern_name: String,
        pub is_bull: bool
    }

    #[derive(Clone)]
    pub struct TriangleDistances {
        pub xb_retrace: [f64; 2],
        pub ac_retrace: [f64; 2],
        pub bd_retrace: [f64; 2],
        pub xd_retrace: [f64; 2],
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

    #[derive(Debug)]
    pub struct Candle {
        pub lowest_price: f64,
        pub highest_price: f64,
        pub closed_price: f64,
        pub volume: f32,
        pub initial_date: i32,
        pub initial_time: i32,
        pub period: i64,
        pub closed: bool
    }

    impl HarmonicPatterns {
        pub fn new() -> HarmonicPatterns {
            HarmonicPatterns {
                gartley: TriangleDistances {
                    xb_retrace: [0.618, -1.0],
                    ac_retrace: [0.382, 0.886],
                    bd_retrace: [1.13, 1.618],
                    xd_retrace: [0.786, -1.0],
                    name: "Gartley".to_string(),
                },
                bat: TriangleDistances {
                    xb_retrace: [0.382, 0.50],
                    ac_retrace: [0.382, 0.886],
                    bd_retrace: [1.618, 2.618],
                    xd_retrace: [0.886, -1.0],
                    name: "Bat".to_string(),
                },
                butterfly: TriangleDistances {
                    xb_retrace: [0.786, -1.0],
                    ac_retrace: [0.382, 0.886],
                    bd_retrace: [1.618, 2.24],
                    xd_retrace: [1.27, 1.41],
                    name: "Butterfly".to_string(),
                },
                crab: TriangleDistances {
                    xb_retrace: [0.382, 0.618],
                    ac_retrace: [0.382, 0.886],
                    bd_retrace: [2.618, 3.618],
                    xd_retrace: [1.618, -1.0],
                    name: "Crab".to_string(),
                },
                deep_crab: TriangleDistances {
                    xb_retrace: [0.886, -1.0],
                    ac_retrace: [0.382, 0.886],
                    bd_retrace: [2.0, 3.618],
                    xd_retrace: [1.618, -1.0],
                    name: "DeepCrab".to_string(),
                },
                cypher: TriangleDistances {
                    xb_retrace: [0.382, 0.618],
                    ac_retrace: [1.13, 1.41],
                    bd_retrace: [1.27, 2.00],
                    xd_retrace: [0.786, -1.0],
                    name: "Cypher".to_string(),
                },
                shark: TriangleDistances {
                    xb_retrace: [-1.0, -1.0],
                    ac_retrace: [1.13, 1.618],
                    bd_retrace: [1.618, 2.24],
                    xd_retrace: [0.886, 1.13],
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

    #[derive(Debug)]
    struct Extreme {
        pub confirmation_index: i32, // Index that this extreme point was confirmed using directional change
        pub price_index: i32, // Index of the extreme point is inside the Vec<Trade>
        pub price: f64,
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
                let height = (current.price - previous.price).abs() as f64;
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

    enum HarmonicOutput {
        Float(Vec<f64>),
        Harmonic(Vec<HarmonicFound>),
        String(Vec<String>)
    }

    fn get_error(value1: Option<f64>, value2: [f64; 2]) -> f64 {

        if value2[0] == -1.0 {
            return 0.0;
        }

        let pattern_log0 = value2[0].ln();

        if value1.unwrap() == 0.0 {
            return 10000.0;
        }

        let mut current_log = value1.unwrap().ln();
        let mut err= 0.0;

        if value2[1] != -1.0 {
            let pattern_log1 = value2[1].ln();


            assert!(pattern_log1 > pattern_log0);

            if pattern_log0 >= current_log && pattern_log1 <= current_log {
                return 0.0;
            }

            err = f64::min(
                (current_log - pattern_log0).abs(),
                (current_log - pattern_log1).abs()
            );
            err *= 2.0;
            return err;
        }
        (current_log - pattern_log0).abs()
    }
    fn find_harmonic_patterns(ohlc: Vec<Candle>, sorted_extremes: Vec<Extreme>, error_threshold: f64) {

        let harmonic_patterns = HarmonicPatterns::new();
        let mut output: HashMap<String, HashMap<String, HarmonicOutput>> = HashMap::new();

        for pattern in harmonic_patterns.iter() {
            println!("{:?}: xb_retrace {:?} {:?} ac_retrace {:?} {:?} bd_retrace {:?} {:?} xd_retrace {:?} {:?}",
                     pattern.name,
                     pattern.xb_retrace[0],
                     pattern.xb_retrace[1],
                     pattern.ac_retrace[0],
                     pattern.ac_retrace[1],
                     pattern.bd_retrace[0],
                     pattern.bd_retrace[1],
                     pattern.xd_retrace[0],
                     pattern.xd_retrace[1]);

            let mut data: HashMap<String, HarmonicOutput> = HashMap::new();


            data.insert("bull_signal".to_string(), HarmonicOutput::Float(vec![0.0; ohlc.len()]));
            data.insert("bull_patterns".to_string(), HarmonicOutput::String(Vec::new()));
            data.insert("bear_signal".to_string(), HarmonicOutput::Float(vec![0.0; ohlc.len()]));
            data.insert("bear_patterns".to_string(), HarmonicOutput::String(Vec::new()));

            // Insert the pattern data into the output HashMap
            output.insert(pattern.name.clone(), data);

        }

        // Get the first extreme to iterate
        let current_extreme = sorted_extremes.get(0).unwrap();
        let mut current_extreme_index = 0;

        let mut bull_signal = Some(sorted_extremes.get(current_extreme_index).unwrap().is_top);
        let mut pattern_used: String = "".to_string();

        let mut d_price: f64;

        'outer_loop: for i in current_extreme_index..ohlc.len() {
            if (current_extreme_index + 1) == i {
                bull_signal = None;
                current_extreme_index += 1;
            }

            if bull_signal == Some(true) {
                if let Some(pattern_map) = output.get_mut(&pattern_used) {
                    if let Some(HarmonicOutput::Float(bull_signal)) = pattern_map.get_mut("bull_signal") {
                        bull_signal[i] = 1.0; // Directly modify the value at index i
                    }
                }
            } else if bull_signal == Some(false) {
                if let Some(pattern_map) = output.get_mut(&pattern_used) {
                    if let Some(HarmonicOutput::Float(bear_signal)) = pattern_map.get_mut("bear_signal") {
                        bear_signal[i] = 1.0;
                    }
                }
            }

            if current_extreme_index + 1 >= sorted_extremes.len() {
                break
            }

            if current_extreme_index < 3 {
                continue;
            }

            // current_extreme_index will be the C point in the XABCD Pattern;
            if sorted_extremes.get(current_extreme_index).unwrap().is_top {
                // Last extreme was a top, we're checking for bull pattern
                d_price = ohlc.get(i).unwrap().lowest_price;

                for j in current_extreme_index..i {
                    // Check that the current low is the lowest since last confirmed top
                    if ohlc.get(j).unwrap().lowest_price < d_price {
                        continue 'outer_loop;
                    }
                }
            } else {
                // Last extreme was a bottom, we're checking for bear pattern
                d_price = ohlc.get(i).unwrap().highest_price;
                for j in current_extreme_index..i {
                    // Check that the current high is the highest since last confirmed bottom
                    if ohlc.get(j).unwrap().highest_price > d_price {
                        continue 'outer_loop;
                    }
                }
            }

            let dc_retrace = (d_price - sorted_extremes[i].price).abs() / sorted_extremes[i].height.unwrap();
            let xd_retrace = (d_price - sorted_extremes[i-2].price).abs() / sorted_extremes[i-2].height.unwrap();


            let mut best_pattern = None;
            let mut best_error = 0.0;

            for pattern in harmonic_patterns.iter() {
                let mut total_error = 0.0;
                total_error += get_error(sorted_extremes[i].retrace_ratio, pattern.ac_retrace);
                total_error += get_error(sorted_extremes[i-1].retrace_ratio, pattern.xb_retrace);
                total_error += get_error(Some(dc_retrace), pattern.bd_retrace);
                total_error += get_error(Some(xd_retrace), pattern.xd_retrace);
                if total_error < best_error {
                    best_error = total_error;
                    best_pattern = Some(pattern.name.clone());
                }
            }

            if best_error <= error_threshold {
                pattern_used = best_pattern.clone().unwrap();
                let mut data = HarmonicFound {
                    extreme_points: [
                        sorted_extremes[current_extreme_index - 3].price_index,
                        sorted_extremes[current_extreme_index - 2].price_index,
                        sorted_extremes[current_extreme_index - 1].price_index,
                        sorted_extremes[current_extreme_index].price_index
                    ],
                    total_error: best_error,
                    pattern_name: pattern_used.parse().unwrap(),
                    is_bull: bull_signal.unwrap(),
                };
                if bull_signal == Some(true) {
                    // TODO: Send Order bull HERE
                    data.pattern_name += "Bull";
                    data.is_bull = true;
                    if let Some(pattern_map) = output.get_mut(&pattern_used) {
                        if let Some(HarmonicOutput::Float(bull_signal)) = pattern_map.get_mut("bull_signal") {
                            bull_signal[i] = 1.0; // Directly modify the value at index i
                        }
                        if let Some(HarmonicOutput::Harmonic(bull_patterns)) = pattern_map.get_mut("bull_patterns") {
                            bull_patterns.push(data);
                        }
                    }
                } else if bull_signal == Some(false) {
                    // TODO: Send Order bear HERE
                    data.pattern_name += "Bear";
                    data.is_bull = false;
                    if let Some(pattern_map) = output.get_mut(&pattern_used) {
                        if let Some(HarmonicOutput::Float(bear_signal)) = pattern_map.get_mut("bear_signal") {
                            bear_signal[i] = 1.0;
                        }
                        if let Some(HarmonicOutput::Harmonic(bear_patterns)) = pattern_map.get_mut("bear_patterns") {
                            bear_patterns.push(data);
                        }
                    }

                }

            }
        }

    }

    #[test]
    fn test_harmonic_patterns() {
        let candle_feed = vec![
            Candle { lowest_price: 120.45, highest_price: 150.75, closed_price: 139.20, volume: 550000.0, initial_date: 20231001, initial_time: 90000, period: 120, closed: true },
            Candle { lowest_price: 125.30, highest_price: 155.90, closed_price: 138.00, volume: 70000.0, initial_date: 20231001, initial_time: 90200, period: 120, closed: true },
            Candle { lowest_price: 132.10, highest_price: 160.40, closed_price: 148.25, volume: 135000.0, initial_date: 20231001, initial_time: 90400, period: 120, closed: true },
            Candle { lowest_price: 118.75, highest_price: 148.90, closed_price: 135.50, volume: 30000.0, initial_date: 20231001, initial_time: 90600, period: 120, closed: true },
            Candle { lowest_price: 128.60, highest_price: 156.00, closed_price: 151.80, volume: 550000.0, initial_date: 20231002, initial_time: 90000, period: 120, closed: true },
            Candle { lowest_price: 135.80, highest_price: 165.30, closed_price: 155.10, volume: 70000.0, initial_date: 20231002, initial_time: 90200, period: 120, closed: true },
            Candle { lowest_price: 122.40, highest_price: 158.10, closed_price: 145.70, volume: 135000.0, initial_date: 20231002, initial_time: 90400, period: 120, closed: true },
            Candle { lowest_price: 129.15, highest_price: 159.50, closed_price: 154.60, volume: 30000.0, initial_date: 20231002, initial_time: 90600, period: 120, closed: true },
            Candle { lowest_price: 140.25, highest_price: 170.80, closed_price: 160.10, volume: 550000.0, initial_date: 20231003, initial_time: 90000, period: 120, closed: true },
            Candle { lowest_price: 134.50, highest_price: 162.40, closed_price: 158.30, volume: 70000.0, initial_date: 20231003, initial_time: 90200, period: 120, closed: true },
        ];

        let mut extremes = vec![
            Extreme { confirmation_index: 0, price_index: 0, price: 150.75, height: None, retrace_ratio: None, is_top: true },
            Extreme { confirmation_index: 1, price_index: 0, price: 120.45, height: None, retrace_ratio: None, is_top: false },
            Extreme { confirmation_index: 3, price_index: 2, price: 160.4, height: None, retrace_ratio: None, is_top: true },
            Extreme { confirmation_index: 4, price_index: 3, price: 118.75, height: None, retrace_ratio: None, is_top: false },
            Extreme { confirmation_index: 6, price_index: 5, price: 165.3, height: None, retrace_ratio: None, is_top: true },
            Extreme { confirmation_index: 7, price_index: 6, price: 122.4, height: None, retrace_ratio: None, is_top: false },
            Extreme { confirmation_index: 9, price_index: 8, price: 170.8, height: None, retrace_ratio: None, is_top: true }
        ];

        Extreme::generate_retracement_ratios(&mut extremes);

        find_harmonic_patterns(candle_feed, extremes, 0.004);
    }
}


