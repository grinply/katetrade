use chrono::{NaiveTime, Timelike};
use ta::Next;
use crate::indicators::exponential_moving_average::ExponentialMovingAverage;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::indicators::volume_weighted_average_price::VolumeWeightedAveragePrice;
use crate::models::action::{market_order_from_side, OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{CandleDirection, Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::strategy::{TradingStrategy};
use crate::utils::date;

const SHORT_SMA_PERIOD: usize = 9;

const LONG_SMA_PERIOD: usize = 9;
const EMA_PERIOD: usize = 9;
const CANDLESTICK_PERIOD: u64 = 1;
const QUANTITY_PER_POSITION: usize = 8;
const CANDLESTICK_SIGNAL_BUFFER_SIZE: usize = 10;

// const TICKS_STOP_LOSS: usize = 15;
// const TICKS_FIRST_TARGET_SIGNAL: usize = 20;
// const TICKS_SECOND_TARGET_SIGNAL: usize = 30;

pub struct VWAPDayInitStrategy {
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,
    min_candles: usize,
    candle_count: usize,
    quadrant_changed_signal_buffer: usize,
    current_side: CandleDirection,
    candle_parser: TimeCandlestickGenerator,
    short_sma: SimpleMovingAverage,
    long_sma: SimpleMovingAverage,
    ema: ExponentialMovingAverage,
    vwap: VolumeWeightedAveragePrice,
    open_position: Option<OpenPosition>
}

impl VWAPDayInitStrategy {
    fn get_current_operation_side(&self, candle: &Candlestick, vwap_value: f64) -> CandleDirection {
        match vwap_value > candle.close {
            true => CandleDirection::LONG,
            false => CandleDirection::SHORT
        }
    }

    fn quadrant_changed_signal_buffer_calculation(&mut self, candlestick: &Candlestick, vwap_value: f64) -> bool {
        let current_side = self.get_current_operation_side(&candlestick, vwap_value);
        let is_side_changed = current_side != self.current_side;

        self.current_side = current_side;

        if is_side_changed {
            self.quadrant_changed_signal_buffer = 1;
        } else {
            match self.quadrant_changed_signal_buffer {
                1..=CANDLESTICK_SIGNAL_BUFFER_SIZE => {
                    self.quadrant_changed_signal_buffer += 1;
                },
                _ if self.quadrant_changed_signal_buffer > CANDLESTICK_SIGNAL_BUFFER_SIZE => {
                    self.quadrant_changed_signal_buffer = 0;
                }
                _ => {}
            }
        }
        self.quadrant_changed_signal_buffer < CANDLESTICK_SIGNAL_BUFFER_SIZE && self.quadrant_changed_signal_buffer != 0
    }

    fn should_open_position(&mut self, vwap: f64, candlestick: &Candlestick) -> Option<OrderAction> {
        let pre_signal = self.quadrant_changed_signal_buffer_calculation(&candlestick, vwap);

        if self.open_position.is_some() {
            return None;
        }

        if self.candle_count < self.min_candles ||
            candlestick.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() > self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        // Keeping it simple. Maybe validate the distance between vwap and short_sma, or the short_sma angle in the future;
        // if self.current_side == CandleDirection::SHORT && self.short_sma.turns_down() {
        // if pre_signal && self.current_side == CandleDirection::SHORT && (self.short_sma.turns_down() || self.short_sma.trending_down() ) && (self.long_sma.turns_down() || self.long_sma.trending_down() ) {
        if pre_signal && self.current_side == CandleDirection::SHORT && (self.short_sma.trending_down() ) && (self.long_sma.trending_down() ) {
            self.open_position = Some(OpenPosition::new(
                Side::SELL,
                candlestick.close,
                self.points_per_tick_asset.unwrap(),
                QUANTITY_PER_POSITION
            ));
            return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION))
        }

        // if self.current_side == CandleDirection::LONG && self.short_sma.turns_up() {
        // if pre_signal && self.current_side == CandleDirection::LONG && (self.short_sma.turns_up() || self.short_sma.trending_up()) && (self.long_sma.turns_up() || self.long_sma.trending_up()) {
        if pre_signal && self.current_side == CandleDirection::LONG && (self.short_sma.trending_up()) && (self.long_sma.trending_up()) {
            self.open_position = Some(OpenPosition::new(
                Side::BUY,
                candlestick.close,
                self.points_per_tick_asset.unwrap(),
                QUANTITY_PER_POSITION
            ));
            return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION))
        }

        None
    }

    fn should_close_position(&mut self, new_trade: &Trade) -> Option<OrderAction> {
        if self.open_position.is_none() {
            return None;
        }

        if let Some(force_exit) = self.should_force_close_position_end_trading_day(
            new_trade,
            self.open_position.as_ref()?.side,
            self.open_position.as_ref()?.open_quantity,
        ) {
            self.quadrant_changed_signal_buffer = 0;
            self.open_position = None;
            return Some(force_exit);
        }

        // let stop_position = self.open_position.as_mut()?.first_chunk.should_close_position(new_trade.price);
        //
        // if stop_position {
        //     if self.open_position.as_ref()?.side == Side::SELL {
        //         self.open_position = None;
        //         return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION))
        //     }
        //
        //     if self.open_position.as_ref()?.side == Side::BUY {
        //         self.open_position = None;
        //         return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION))
        //     }
        // }

        if let Some(_position) = self.open_position.as_mut() {
            if self.open_position.as_ref()?.side == Side::SELL && self.short_sma.turns_up() {
                self.open_position = None;
                self.quadrant_changed_signal_buffer = 0;
                return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION))
            }
            if self.open_position.as_ref()?.side == Side::BUY && self.short_sma.turns_down() {
                self.open_position = None;
                self.quadrant_changed_signal_buffer = 0;
                return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION))
            }
        }

        // if let Some(_position) = self.open_position.as_mut() {
        //     if self.open_position.as_ref()?.side == Side::SELL && self.short_sma.turns_up() {
        //         if new_trade.price < self.open_position.as_ref()?.first_chunk.entry_price  - (self.open_position.as_ref()?.first_chunk.entry_price * self.points_per_tick_asset.unwrap()) {
        //             self.open_position = None;
        //             return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION))
        //         }
        //     }
        //
        //     if self.open_position.as_ref()?.side == Side::BUY && self.short_sma.turns_down() {
        //         if new_trade.price > self.open_position.as_ref()?.first_chunk.entry_price  + (self.open_position.as_ref()?.first_chunk.entry_price * self.points_per_tick_asset.unwrap()) {
        //             self.open_position = None;
        //             return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION))
        //         }
        //     }
        // }

        None
    }

    fn should_force_close_position_end_trading_day(&self, new_trade: &Trade, side: Side, open_quantity: usize) -> Option<OrderAction> {
        let current_time = date::convert_to_naive_time(new_trade.time);
        assert!(
            current_time.is_some(),
            "should never fail when converting a numeric time from an trade into NaiveTime. time from trade: {}", new_trade.time
        );
        let current_time = current_time?;

        // we have an open position be the end of trading day is approaching, we must close it
        let current_time = current_time.num_seconds_from_midnight();
        let force_close_time = self.time_force_close_open_positions.num_seconds_from_midnight();

        if current_time >= force_close_time {
            return if side == Side::BUY {
                Some(OrderAction::MarketShort(open_quantity))
            } else {
                Some(OrderAction::MarketLong(open_quantity))
            }
        }
        None
    }
}

impl TradingStrategy for VWAPDayInitStrategy {
    fn new() -> Self
    where
        Self: Sized
    {
        VWAPDayInitStrategy {
            points_per_tick_asset: None,
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(10, 30, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(17, 30, 00).unwrap(),
            min_candles: 20,
            candle_count: 0,
            quadrant_changed_signal_buffer: 0,
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Minutes, CANDLESTICK_PERIOD),
            short_sma: SimpleMovingAverage::new(SHORT_SMA_PERIOD).unwrap(),
            long_sma: SimpleMovingAverage::new(LONG_SMA_PERIOD).unwrap(),
            ema: ExponentialMovingAverage::default(SHORT_SMA_PERIOD),
            vwap: VolumeWeightedAveragePrice::new().unwrap(),
            current_side: CandleDirection::UNDEFINED,
            open_position: None,
        }
    }

    fn name(&self) -> &str {
        "VWAP_init_trading_day_strategy"
    }

    fn init_trading_day(&mut self, asset: &Asset, _trades: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);
        self.candle_count = 0;
        self.quadrant_changed_signal_buffer = 0;

        self.short_sma.reset();
        self.long_sma.reset();
        self.ema.reset();
        self.vwap.reset();
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut order_open_position: Option<OrderAction> = None;
        let order_close_position: Option<OrderAction> = self.should_close_position(new_trade);

        if let Some(candlestick) = self.candle_parser.parse_only_closed_candles(new_trade) {
            self.candle_count += 1;

            let _short_sma = self.short_sma.next(candlestick);
            let _long_sma = self.long_sma.next(candlestick);
            let _ema = self.ema.next(candlestick);
            let vwap = self.vwap.next(&candlestick);

            order_open_position = self.should_open_position(vwap, &candlestick);

            if let Some(position) = self.open_position.as_mut() {
                position.first_chunk.set_stop_loss(vwap);
            }
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
            first_chunk: TrailingStop::new_not_automatic_trailing(
                entry_price,
                side,
                points_per_tick,
            ),
            // first_chunk: TrailingStop::new(
            //     entry_price,
            //     side,
            //     points_per_tick,
            //     TICKS_STOP_LOSS,
            //     TICKS_FIRST_TARGET_SIGNAL,
            //     1,
            //     1,
            //     1
            // )
        }
    }
}