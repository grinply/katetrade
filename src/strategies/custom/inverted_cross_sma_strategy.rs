use chrono::{NaiveTime, Timelike};
use ta::Next;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::indicators::volume_weighted_average_price::VolumeWeightedAveragePrice;
use crate::models::action::{market_order_from_side, OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{CandleDirection, Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::strategy::{TradingStrategy};
use crate::utils::{date, end_trading_day};

const SHORT_SMA_PERIOD: usize = 21;
const LONG_SMA_PERIOD: usize = 120;
const CANDLESTICK_PERIOD: u64 = 1;
const CANDLESTICK_PERIOD_UNIT: TimeUnit = TimeUnit::Minutes;
const QUANTITY_PER_POSITION: usize = 8;
const TICKS_STOP_LOSS: usize = 15;
const TICKS_RP_FIRST_CHUNK: usize = 15;
const MIN_CANDLES_TO_START: usize = 120;
const TICKS_FIRST_TRAILING_TRIGGER: usize = 0;
const TICKS_DISTANCE_WHEN_TRAILING: usize = 0;
const TICKS_FREQUENCY_TRAILING_UPDATE: usize = 1;

pub struct InvertedCrossSMAStrategy {
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,
    candle_count: usize,
    candle_parser: TimeCandlestickGenerator,
    short_sma: SimpleMovingAverage,
    long_sma: SimpleMovingAverage,
    // exit_sma: SimpleMovingAverage,
    open_position: Option<OpenPosition>
}

impl InvertedCrossSMAStrategy {

    fn crossed_sma(&self, previous_short_sma: f64, previous_long_sma: f64, current_short_sma: f64, current_long_sma: f64) -> bool {
        if (previous_short_sma <= previous_long_sma) && (current_short_sma > current_long_sma) {
            return true;
        }

        if previous_short_sma >= previous_long_sma && current_short_sma < current_long_sma {
            return true;
        }

        false
    }

    fn should_open_position(&mut self, candlestick: &Candlestick) -> Option<OrderAction> {
        if self.open_position.is_some() {
            return None;
        }

        if self.candle_count < MIN_CANDLES_TO_START ||
            candlestick.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() > self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        if self.crossed_sma(
            self.short_sma.previous_value(),
            self.long_sma.previous_value(),
            self.short_sma.value(),
            self.long_sma.value()
        ) {
            // println!("SMA has Crossed on {} {}", candlestick.date, candlestick.start_time);
            if self.short_sma.value() > self.long_sma.value() {
                self.open_position = Some(OpenPosition::new(
                    Side::SELL,
                    candlestick.close,
                    self.points_per_tick_asset.unwrap(),
                    QUANTITY_PER_POSITION
                ));
                return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION));
            }

            if self.short_sma.value() < self.long_sma.value() {
                self.open_position = Some(OpenPosition::new(
                    Side::BUY,
                    candlestick.close,
                    self.points_per_tick_asset.unwrap(),
                    QUANTITY_PER_POSITION
                ));
                return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION));
            }
        }
        None
    }

    // fn should_close_position(&mut self, candlestick: &Candlestick) -> Option<OrderAction> {
    fn should_close_position(&mut self, new_trade: &Trade) -> Option<OrderAction> {
        if self.open_position.is_none() {
            return None;
        }

        if let Some(force_exit) = end_trading_day::should_force_close_position_end_trading_day(
            date::convert_to_naive_time(new_trade.time)?.num_seconds_from_midnight(),
            // candlestick.close_time.unwrap().num_seconds_from_midnight(),
            self.open_position.as_ref()?.side,
            self.open_position.as_ref()?.open_quantity,
            self.time_force_close_open_positions.num_seconds_from_midnight()
        ) {
            self.open_position = None;
            return Some(force_exit);
        }

        let mut quantity_to_close: usize = 0;

        let is_first_chunk_finished = self.open_position.as_ref()?.first_chunk.is_finished();
        // let should_close_first_chunk = self.open_position.as_mut()?.first_chunk.should_close_position(candlestick.close);
        let should_close_first_chunk = self.open_position.as_mut()?.first_chunk.should_close_position(new_trade.price);

        if !is_first_chunk_finished && should_close_first_chunk {
            self.open_position.as_mut()?.first_chunk.finish_trailing();
            quantity_to_close += self.open_position.as_mut()?.quantity_per_chunk[0];
        }

        if quantity_to_close > 0 {
            let side = self.open_position.as_ref()?.side;
            self.open_position = None;

            let order = market_order_from_side(
                side.opposite_side(),
                quantity_to_close,
            );

            return Some(order);
        }

        None
    }


}

impl TradingStrategy for InvertedCrossSMAStrategy {
    fn new() -> Self
    where
        Self: Sized
    {
        InvertedCrossSMAStrategy {
            points_per_tick_asset: Some(5.0),
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(17, 20, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(17, 30, 00).unwrap(),
            candle_count: 0,
            candle_parser: TimeCandlestickGenerator::new(CANDLESTICK_PERIOD_UNIT, CANDLESTICK_PERIOD),
            short_sma: SimpleMovingAverage::new(SHORT_SMA_PERIOD).unwrap(),
            long_sma: SimpleMovingAverage::new(LONG_SMA_PERIOD).unwrap(),
            // exit_sma: SimpleMovingAverage::new(EXIT_SMA_PERIOD).unwrap(),
            open_position: None,
        }
    }

    fn name(&self) -> &str {
        "inverted_cross_sma_strategy"
    }

    fn init_trading_day(&mut self, asset: &Asset, trades: &[Trade]) {
        // self.points_per_tick_asset = Some(asset.min_point_variation);
        // self.short_sma.reset();
        // self.long_sma.reset();
        //
        // for trade in trades {
        //     if let Some(candle) = self.candle_parser.parse_only_closed_candles(trade) {
        //         self.short_sma.next(candle.close);
        //         self.long_sma.next(candle.close);
        //     }
        // }

    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut order_open_position: Option<OrderAction> = None;
        let mut order_close_position: Option<OrderAction> = None;
        order_close_position = self.should_close_position(new_trade);

        if let Some(candlestick) = self.candle_parser.parse_only_closed_candles(new_trade) {
            self.candle_count += 1;

            let _short_sma = self.short_sma.next(candlestick);
            let _long_sma = self.long_sma.next(candlestick);
            // let exit_sma = self.exit_sma.next(candlestick);

            // println!("Candle count: {}", self.candle_count);
            // println!("Short sma: {:?}", _short_sma);
            // println!("Long sma: {:?}", _long_sma);

            // order_close_position = self.should_close_position(&candlestick);
            order_open_position = self.should_open_position(&candlestick);


            // if let Some(position) = self.open_position.as_mut() {
            //     position.first_chunk.set_stop_loss(exit_sma);
            // }

        }

        TradeAction::custom_order(order_close_position, order_open_position, None, None)
    }
}

#[allow(unused)]
#[derive(Debug)]
struct OpenPosition {
    side: Side,
    open_quantity: usize,
    quantity_per_chunk: [usize; 1],
    first_chunk: TrailingStop,
}

#[allow(unused)]
impl OpenPosition {
    fn new(
        side: Side,
        entry_price: f64,
        points_per_tick: f64,
        quantity_to_trade: usize,
    ) -> Self {
        // assert_eq!(quantity_to_trade % 2, 0, "{} quantity must be divisible by 2", quantity_to_trade);
        OpenPosition {
            side,
            open_quantity: quantity_to_trade,
            quantity_per_chunk: [quantity_to_trade],
            first_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                TICKS_STOP_LOSS,
                TICKS_RP_FIRST_CHUNK,
                TICKS_FIRST_TRAILING_TRIGGER,
                TICKS_DISTANCE_WHEN_TRAILING,
                TICKS_FREQUENCY_TRAILING_UPDATE
            )
        }
    }
    fn new_not_automatic_trailing(
        side: Side,
        entry_price: f64,
        points_per_tick: f64,
        quantity_to_trade: usize,
    ) -> Self {
        // assert_eq!(quantity_to_trade % 2, 0, "{} quantity must be divisible by 2", quantity_to_trade);
        OpenPosition {
            side,
            open_quantity: quantity_to_trade,
            quantity_per_chunk: [quantity_to_trade],
            first_chunk: TrailingStop::new_not_automatic_trailing(
                entry_price,
                side,
                points_per_tick,
            )
        }
    }


}