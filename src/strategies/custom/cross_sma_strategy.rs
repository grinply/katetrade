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
const TICKS_STOP_LOSS: usize = 3;
const TICKS_RP_FIRST_CHUNK: usize = 11;

pub struct CrossSMAStrategy {
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,
    candle_parser: TimeCandlestickGenerator,
    short_sma: SimpleMovingAverage,
    long_sma: SimpleMovingAverage,
    open_position: Option<OpenPosition>
}

impl CrossSMAStrategy {

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

        if self.crossed_sma(
            self.short_sma.previous_value(),
            self.long_sma.previous_value(),
            self.short_sma.value(),
            self.long_sma.value()
        ) {
            if self.short_sma.value() > self.long_sma.value() {
                return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION));
            }

            if self.short_sma.value() < self.long_sma.value() {
                return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION));
            }
        }
        None
    }

    fn should_close_position(&mut self, candlestick: &Candlestick) -> Option<OrderAction> {
        if self.open_position.is_none() {
            return None;
        }

        if let Some(force_exit) = end_trading_day::should_force_close_position_end_trading_day(
            candlestick.close_time.unwrap().num_seconds_from_midnight(),
            self.open_position.as_ref()?.side,
            self.open_position.as_ref()?.open_quantity,
            self.time_force_close_open_positions.num_seconds_from_midnight()
        ) {
            self.open_position = None;
            return Some(force_exit);
        }

        let mut quantity_to_close: usize = 0;

        let is_first_chunk_finished = self.open_position.as_ref()?.first_chunk.is_finished();
        let should_close_first_chunk = self.open_position.as_mut()?.first_chunk.should_close_position(candlestick.close);

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

impl TradingStrategy for CrossSMAStrategy {
    fn new() -> Self
    where
        Self: Sized
    {
        CrossSMAStrategy {
            points_per_tick_asset: None,
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(17, 20, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(17, 30, 00).unwrap(),
            candle_parser: TimeCandlestickGenerator::new(CANDLESTICK_PERIOD_UNIT, CANDLESTICK_PERIOD),
            short_sma: SimpleMovingAverage::new(SHORT_SMA_PERIOD).unwrap(),
            long_sma: SimpleMovingAverage::new(LONG_SMA_PERIOD).unwrap(),
            open_position: None,
        }
    }

    fn name(&self) -> &str {
        "cross_sma_strategy"
    }

    fn init_trading_day(&mut self, asset: &Asset, trades: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);

        self.short_sma.reset();
        self.long_sma.reset();

        for trade in trades {
            if let Some(candle) = self.candle_parser.parse_only_closed_candles(trade) {
                self.short_sma.next(candle);
                self.long_sma.next(candle);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut order_open_position: Option<OrderAction> = None;
        let mut order_close_position: Option<OrderAction> = None;

        if let Some(candlestick) = self.candle_parser.parse_only_closed_candles(new_trade) {
            let _short_sma = self.short_sma.next(candlestick);
            let _long_sma = self.long_sma.next(candlestick);

            order_close_position = self.should_close_position(&candlestick);
            order_open_position = self.should_open_position(&candlestick);
        }

        TradeAction::custom_order(order_close_position, order_open_position, None, None)
    }
}

#[allow(unused)]
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
                1,
                1,
                1
            )
            // first_chunk: TrailingStop::new_not_automatic_trailing()
        }
    }
}