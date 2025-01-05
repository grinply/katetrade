// #[cfg(test)]
// mod tests {
//     use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
//
//     #[derive(Clone, Debug, Copy)]
//     pub struct Trade {
//         pub trade_number: i32,
//         pub price: f64,
//         pub volume: f32,
//         pub date: i32,
//         pub time: i32,
//         pub qty: i32,
//         pub buy_agent: i8,
//         pub sell_agent: i8,
//         pub trade_type: i8,
//     }
//
//     #[derive(Debug)]
//     pub struct Candle {
//         pub lowest_price: f64,
//         pub highest_price: f64,
//         pub closed_price: f64,
//         pub volume: f32,
//         pub initial_date: i32,
//         pub initial_time: i32,
//         pub period: i64,
//         pub closed: bool,
//     }
//     fn calculate_candles_by_period(trade_feed: Vec<Trade>, period: i64) -> Vec<Candle> {
//         let mut first_trade = trade_feed[0];
//         let mut first_candle_datetime = to_naive_datetime(first_trade.date, first_trade.time);
//
//         let mut lowest_candle_price: f64 = first_trade.price;
//         let mut highest_candle_price: f64 = first_trade.price;
//         let mut volume_candle: f32 = 0.0;
//         let mut current_price: f64 = first_trade.price;
//
//         let mut candles: Vec<Candle> = Vec::new();
//
//         for (i, trade) in trade_feed.iter().enumerate() {
//             let current_datetime = to_naive_datetime(trade.date, trade.time);
//
//             if current_datetime
//                 .signed_duration_since(first_candle_datetime)
//                 .num_seconds()
//                 >= period
//             {
//                 // Candle closed, push it.
//                 candles.push(Candle {
//                     lowest_price: lowest_candle_price,
//                     highest_price: highest_candle_price,
//                     volume: volume_candle,
//                     closed_price: current_price,
//                     initial_date: first_trade.date,
//                     initial_time: first_trade.time,
//                     period,
//                     closed: true,
//                 });
//
//                 // Reset variables for new candle
//                 first_trade = *trade;
//                 first_candle_datetime = to_naive_datetime(first_trade.date, first_trade.time);
//                 lowest_candle_price = trade.price;
//                 highest_candle_price = trade.price;
//                 volume_candle = 0.0;
//             }
//
//             current_price = trade.price;
//             volume_candle += trade.volume;
//
//             if (current_price < lowest_candle_price) {
//                 lowest_candle_price = current_price;
//             }
//
//             if (current_price > highest_candle_price) {
//                 highest_candle_price = current_price;
//             }
//
//             // Caso seja o último trade, fecha o candle
//             if trade_feed.len() == (i + 1) {
//                 candles.push(Candle {
//                     lowest_price: lowest_candle_price,
//                     highest_price: highest_candle_price,
//                     volume: volume_candle,
//                     closed_price: current_price,
//                     initial_date: first_trade.date,
//                     initial_time: first_trade.time,
//                     period,
//                     closed: false,
//                 });
//             }
//         }
//
//         candles
//     }
//
//     fn to_naive_datetime(date: i32, time: i32) -> NaiveDateTime {
//         NaiveDateTime::new(
//             NaiveDate::parse_from_str(format!("{:}", date).as_str(), "%Y%m%d").unwrap(),
//             NaiveTime::parse_from_str(format!("{:06}", time).as_str(), "%H%M%S").unwrap(),
//         )
//     }
//
//     #[derive(Debug)]
//     struct Extreme {
//         pub confirmation_index: i32, // Index that this extreme point was confirmed using directional change
//         pub price_index: i32,        // Index of the extreme point is inside the Vec<Trade>
//         // pub trade: Trade,
//         pub price: f64,
//         pub height: Option<f64>, // The height of the segment between two consecutive extreme points (absolute).
//         pub retrace_ratio: Option<f64>, // The ratio of the current segment's height to the previous segment's height.
//         pub is_top: bool,
//     }
//
//     impl Extreme {
//         // Maybe generate these values in the for inside directional change for performance?
//         fn generate_retracement_ratios(extremes: &mut Vec<Extreme>) {
//             // Temporários para armazenar os novos valores
//             let mut heights: Vec<Option<f64>> = vec![None; extremes.len()];
//             let mut retrace_ratios: Vec<Option<f64>> = vec![None; extremes.len()];
//
//             // Primeiro passo: calcular heights e retrace_ratios
//             for i in 1..extremes.len() {
//                 let current = &extremes[i];
//                 let previous = &extremes[i - 1];
//
//                 // Calcular height como a diferença absoluta nos preços
//                 let height = (current.price - previous.price).abs() as f64;
//                 heights[i] = Some(height);
//
//                 // Calcular retrace_ratio se o height anterior existir
//                 if let Some(prev_height) = heights[i - 1] {
//                     retrace_ratios[i] = Some(height / prev_height);
//                 }
//             }
//
//             // Segundo passo: aplicar os novos valores de volta ao vetor `extremes`
//             for i in 1..extremes.len() {
//                 extremes[i].height = heights[i];
//                 extremes[i].retrace_ratio = retrace_ratios[i];
//             }
//         }
//     }
//
//     fn directional_change_percentage(candle_feed: Vec<Candle>, threshold: f64) -> Vec<Extreme> {
//         let mut is_high_flow = true;
//         let mut tmp_price_min = candle_feed[0].lowest_price;
//         let mut tmp_price_max = candle_feed[0].highest_price;
//         let mut tmp_price_min_index = 0;
//         let mut tmp_price_max_index = 0;
//
//         let mut tops_and_bottoms: Vec<Extreme> = vec![];
//
//         for (i, candle) in candle_feed.iter().enumerate() {
//             let current_candle = candle;
//
//             if is_high_flow {
//                 if current_candle.highest_price > tmp_price_max {
//                     tmp_price_max = current_candle.highest_price;
//                     tmp_price_max_index = i;
//                 } else if current_candle.closed_price
//                     < (tmp_price_max - (tmp_price_max * threshold))
//                 {
//                     tops_and_bottoms.push(Extreme {
//                         confirmation_index: i as i32,
//                         price_index: tmp_price_max_index as i32,
//                         price: tmp_price_max,
//                         height: None,
//                         retrace_ratio: None,
//                         is_top: true,
//                     });
//
//                     is_high_flow = false;
//                     tmp_price_min = current_candle.lowest_price;
//                     tmp_price_min_index = i;
//                 }
//             } else {
//                 if current_candle.lowest_price < tmp_price_min {
//                     tmp_price_min = current_candle.lowest_price;
//                     tmp_price_min_index = i;
//                 } else if current_candle.closed_price
//                     > (tmp_price_min + (tmp_price_min * threshold))
//                 {
//                     tops_and_bottoms.push(Extreme {
//                         confirmation_index: i as i32,
//                         price_index: tmp_price_min_index as i32,
//                         price: tmp_price_min,
//                         height: None,
//                         retrace_ratio: None,
//                         is_top: false,
//                     });
//                     is_high_flow = true;
//                     tmp_price_min = current_candle.highest_price;
//                     tmp_price_min_index = i;
//                 }
//             }
//         }
//
//         tops_and_bottoms
//     }
//
//     #[test]
//     fn test_generate_candles_by_period() {
//         let trade_feed = vec![
//             Trade {
//                 trade_number: 1,
//                 price: 100.0,
//                 volume: 500000.0000,
//                 date: 20231001,
//                 time: 90000,
//                 qty: 10,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 2,
//                 price: 101.0,
//                 volume: 50000.0000,
//                 date: 20231001,
//                 time: 90100,
//                 qty: 15,
//                 buy_agent: 2,
//                 sell_agent: 1,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 3,
//                 price: 102.0,
//                 volume: 20000.0000,
//                 date: 20231001,
//                 time: 90200,
//                 qty: 12,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 4,
//                 price: 98.0,
//                 volume: 50000.0000,
//                 date: 20231001,
//                 time: 90300,
//                 qty: 18,
//                 buy_agent: 2,
//                 sell_agent: 1,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 5,
//                 price: 99.0,
//                 volume: 110000.0000,
//                 date: 20231001,
//                 time: 90400,
//                 qty: 11,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 6,
//                 price: 100.0,
//                 volume: 25000.0000,
//                 date: 20231001,
//                 time: 90500,
//                 qty: 11,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 7,
//                 price: 100.0,
//                 volume: 30000.0000,
//                 date: 20231001,
//                 time: 90600,
//                 qty: 11,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 8,
//                 price: 100.0,
//                 volume: 500000.0000,
//                 date: 20231002,
//                 time: 90000,
//                 qty: 10,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 9,
//                 price: 101.0,
//                 volume: 50000.0000,
//                 date: 20231002,
//                 time: 90100,
//                 qty: 15,
//                 buy_agent: 2,
//                 sell_agent: 1,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 10,
//                 price: 102.0,
//                 volume: 20000.0000,
//                 date: 20231002,
//                 time: 90200,
//                 qty: 12,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 11,
//                 price: 98.0,
//                 volume: 50000.0000,
//                 date: 20231002,
//                 time: 90300,
//                 qty: 18,
//                 buy_agent: 2,
//                 sell_agent: 1,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 12,
//                 price: 99.0,
//                 volume: 110000.0000,
//                 date: 20231002,
//                 time: 90400,
//                 qty: 11,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 13,
//                 price: 100.0,
//                 volume: 25000.0000,
//                 date: 20231002,
//                 time: 90500,
//                 qty: 11,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//             Trade {
//                 trade_number: 14,
//                 price: 100.0,
//                 volume: 30000.0000,
//                 date: 20231002,
//                 time: 90600,
//                 qty: 11,
//                 buy_agent: 1,
//                 sell_agent: 2,
//                 trade_type: 0,
//             },
//         ];
//
//         let candles = calculate_candles_by_period(trade_feed, 120);
//
//         for candle in candles {
//             println!("{:?}", candle);
//         }
//     }
//
//     #[test]
//     fn test_directional_change_percentage() {
//         let candle_feed = vec![
//             Candle {
//                 lowest_price: 120.45,
//                 highest_price: 150.75,
//                 closed_price: 139.20,
//                 volume: 550000.0,
//                 initial_date: 20231001,
//                 initial_time: 90000,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 125.30,
//                 highest_price: 155.90,
//                 closed_price: 138.00,
//                 volume: 70000.0,
//                 initial_date: 20231001,
//                 initial_time: 90200,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 132.10,
//                 highest_price: 160.40,
//                 closed_price: 148.25,
//                 volume: 135000.0,
//                 initial_date: 20231001,
//                 initial_time: 90400,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 118.75,
//                 highest_price: 148.90,
//                 closed_price: 135.50,
//                 volume: 30000.0,
//                 initial_date: 20231001,
//                 initial_time: 90600,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 128.60,
//                 highest_price: 156.00,
//                 closed_price: 151.80,
//                 volume: 550000.0,
//                 initial_date: 20231002,
//                 initial_time: 90000,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 135.80,
//                 highest_price: 165.30,
//                 closed_price: 155.10,
//                 volume: 70000.0,
//                 initial_date: 20231002,
//                 initial_time: 90200,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 122.40,
//                 highest_price: 158.10,
//                 closed_price: 145.70,
//                 volume: 135000.0,
//                 initial_date: 20231002,
//                 initial_time: 90400,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 129.15,
//                 highest_price: 159.50,
//                 closed_price: 154.60,
//                 volume: 30000.0,
//                 initial_date: 20231002,
//                 initial_time: 90600,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 140.25,
//                 highest_price: 170.80,
//                 closed_price: 160.10,
//                 volume: 550000.0,
//                 initial_date: 20231003,
//                 initial_time: 90000,
//                 period: 120,
//                 closed: true,
//             },
//             Candle {
//                 lowest_price: 134.50,
//                 highest_price: 162.40,
//                 closed_price: 158.30,
//                 volume: 70000.0,
//                 initial_date: 20231003,
//                 initial_time: 90200,
//                 period: 120,
//                 closed: true,
//             },
//         ];
//
//         let threshold = 0.01;
//         let extremes = directional_change_percentage(candle_feed, threshold);
//
//         assert_eq!(extremes.len(), 7);
//
//         assert_eq!(extremes[0].is_top, true);
//         assert_eq!(extremes[0].price, 150.75);
//         assert_eq!(extremes[0].confirmation_index, 0);
//         assert_eq!(extremes[0].price_index, 0);
//
//         assert_eq!(extremes[1].is_top, false);
//         assert_eq!(extremes[1].price, 120.45);
//         assert_eq!(extremes[1].confirmation_index, 1);
//         assert_eq!(extremes[1].price_index, 0);
//
//         assert_eq!(extremes[2].is_top, true);
//         assert_eq!(extremes[2].price, 160.4);
//         assert_eq!(extremes[2].confirmation_index, 3);
//         assert_eq!(extremes[2].price_index, 2);
//
//         assert_eq!(extremes[3].is_top, false);
//         assert_eq!(extremes[3].price, 118.75);
//         assert_eq!(extremes[3].confirmation_index, 4);
//         assert_eq!(extremes[3].price_index, 3);
//
//         assert_eq!(extremes[4].is_top, true);
//         assert_eq!(extremes[4].price, 165.3);
//         assert_eq!(extremes[4].confirmation_index, 6);
//         assert_eq!(extremes[4].price_index, 5);
//
//         assert_eq!(extremes[5].is_top, false);
//         assert_eq!(extremes[5].price, 122.4);
//         assert_eq!(extremes[5].confirmation_index, 7);
//         assert_eq!(extremes[5].price_index, 6);
//
//         assert_eq!(extremes[6].is_top, true);
//         assert_eq!(extremes[6].price, 170.8);
//         assert_eq!(extremes[6].confirmation_index, 9);
//         assert_eq!(extremes[6].price_index, 8);
//
//         println!("All assertions passed!");
//     }
//
//     #[test]
//     fn test_extreme_retracement_ratios_calculation() {
//         let mut extremes = vec![
//             Extreme {
//                 confirmation_index: 0,
//                 price_index: 0,
//                 price: 150.75,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: true,
//             },
//             Extreme {
//                 confirmation_index: 1,
//                 price_index: 0,
//                 price: 120.45,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: false,
//             },
//             Extreme {
//                 confirmation_index: 3,
//                 price_index: 2,
//                 price: 160.4,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: true,
//             },
//             Extreme {
//                 confirmation_index: 4,
//                 price_index: 3,
//                 price: 118.75,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: false,
//             },
//             Extreme {
//                 confirmation_index: 6,
//                 price_index: 5,
//                 price: 165.3,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: true,
//             },
//             Extreme {
//                 confirmation_index: 7,
//                 price_index: 6,
//                 price: 122.4,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: false,
//             },
//             Extreme {
//                 confirmation_index: 9,
//                 price_index: 8,
//                 price: 170.8,
//                 height: None,
//                 retrace_ratio: None,
//                 is_top: true,
//             },
//         ];
//
//         Extreme::generate_retracement_ratios(&mut extremes);
//
//         assert_eq!(extremes[0].height, None);
//         assert_eq!(extremes[0].retrace_ratio, None);
//
//         assert_eq!(extremes[1].height, Some(30.299999999999997));
//         assert_eq!(extremes[1].retrace_ratio, None);
//
//         assert_eq!(extremes[2].height, Some(39.95));
//         assert_eq!(extremes[2].retrace_ratio, Some(1.3184818481848186));
//
//         assert_eq!(extremes[3].height, Some(41.650000000000006));
//         assert_eq!(extremes[3].retrace_ratio, Some(1.0425531914893618));
//
//         assert_eq!(extremes[4].height, Some(46.55000000000001));
//         assert_eq!(extremes[4].retrace_ratio, Some(1.1176470588235294));
//
//         assert_eq!(extremes[5].height, Some(42.900000000000006));
//         assert_eq!(extremes[5].retrace_ratio, Some(0.9215896885069816));
//
//         assert_eq!(extremes[6].height, Some(48.400000000000006));
//         assert_eq!(extremes[6].retrace_ratio, Some(1.1282051282051282));
//     }
// }
