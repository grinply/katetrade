use crate::models::action::{OrderAction, TradeAction, market_order_from_side};
use crate::models::asset::Asset;
use crate::models::candlestick::{CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::trade::Trade;
use ta::indicators::BollingerBands;
use ta::{Next, Reset};
use crate::models::side::Side;
use crate::strategies::strategy::TradingStrategy;

pub struct BollingerBandReversion {
    bollinger_bands: BollingerBands,
    candle_parser: TimeCandlestickGenerator,
    open_position: Option<Position>,
}

struct Position {
    side: Side,
    entry_price: f64,
    quantity: usize,
}

impl TradingStrategy for BollingerBandReversion {
    fn new() -> Self {
        BollingerBandReversion {
            bollinger_bands: BollingerBands::new(20, 2.0).unwrap(),
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Minutes, 1),
            open_position: None,
        }
    }

    fn name(&self) -> &str {
        "bollinger_band_reversion"
    }

    fn days_of_context(&self) -> u32 {
        0
    }

    fn init_trading_day(&mut self, _asset: &Asset, context_trades: &[Trade]) {
        self.bollinger_bands.reset();
        for trade in context_trades {
            if let Some(candle) = self.candle_parser.parse_only_closed_candles(trade) {
                self.bollinger_bands.next(candle.close);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction { //Original ChatGPT
        if let Some(candle) = self.candle_parser.parse_only_closed_candles(new_trade) {
            let bands = self.bollinger_bands.next(candle.close);

            match &mut self.open_position {
                // Exit logic
                Some(position) => {
                    if (position.side == Side::BUY && candle.close >= bands.average)
                        || (position.side == Side::SELL && candle.close <= bands.average)
                        || (candle.close > bands.upper || candle.close < bands.lower)
                    {
                        let quantity_to_close = position.quantity;
                        let exit_action = market_order_from_side(
                            position.side.opposite_side(),
                            quantity_to_close,
                        );
                        self.open_position = None;
                        return TradeAction::custom_order(Some(exit_action), None, None, None);
                    }
                }
                // Entry logic
                None => {
                    if candle.close <= bands.lower {
                        self.open_position = Some(Position {
                            side: Side::BUY,
                            entry_price: candle.close,
                            quantity: 1,
                        });
                        let entry_action = OrderAction::MarketLong(1);
                        return TradeAction::custom_order(None, Some(entry_action), None, None);
                    } else if candle.close >= bands.upper {
                        self.open_position = Some(Position {
                            side: Side::SELL,
                            entry_price: candle.close,
                            quantity: 1,
                        });
                        let entry_action = OrderAction::MarketShort(1);
                        return TradeAction::custom_order(None, Some(entry_action), None, None);
                    }
                }
            }
        }

        TradeAction::do_nothing()
    }

    //     fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction { //Randomly changed signals
//         if let Some(candle) = self.candle_parser.parse_only_closed_candles(new_trade) {
//             let bands = self.bollinger_bands.next(candle.close);
//
//             match &mut self.open_position {
//                 // Exit logic
//                 Some(position) => {
//                     if (position.side == Side::BUY && candle.close <= bands.average)
//                         || (position.side == Side::SELL && candle.close >= bands.average)
//                         || (candle.close < bands.upper || candle.close > bands.lower)
//                     {
//                         let quantity_to_close = position.quantity;
//                         let exit_action = market_order_from_side(
//                             position.side.opposite_side(),
//                             quantity_to_close,
//                         );
//                         self.open_position = None;
//                         return TradeAction::custom_order(Some(exit_action), None, None, None);
//                     }
//                 }
//                 // Entry logic
//                 None => {
//                     if candle.close <= bands.lower {
//                         self.open_position = Some(Position {
//                             side: Side::BUY,
//                             entry_price: candle.close,
//                             quantity: 1,
//                         });
//                         let entry_action = OrderAction::MarketShort(1);
//                         return TradeAction::custom_order(None, Some(entry_action), None, None);
//                     } else if candle.close >= bands.upper {
//                         self.open_position = Some(Position {
//                             side: Side::SELL,
//                             entry_price: candle.close,
//                             quantity: 1,
//                         });
//                         let entry_action = OrderAction::MarketLong(1);
//                         return TradeAction::custom_order(None, Some(entry_action), None, None);
//                     }
//                 }
//             }
//         }
//
//         TradeAction::do_nothing()
//     }
}
