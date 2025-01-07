use chrono::{NaiveTime, Timelike};
use ta::Next;
use crate::indicators::exponential_moving_average::ExponentialMovingAverage;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::indicators::volume_weighted_average_price::VolumeWeightedAveragePrice;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{CandleDirection, Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::strategy::HighResolutionTradingStrategy;

const SMA_PERIOD: usize = 5;
const CANDLESTICK_PERIOD: u64 = 1;
const QUANTITY_PER_POSITION: usize = 2;
const CANDLESTICK_SIGNAL_BUFFER_SIZE: usize = 10;
const MAXIMUM_TICKS_STOP_LOSS: usize = 20;
const TICKS_FIRST_TARGET_SIGNAL: usize = 20;
const TICKS_SECOND_TARGET_SIGNAL: usize = 80;

pub struct VWAPDayInitStrategy {
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,
    min_candles: usize,
    candle_count: usize,
    quadrant_changed_signal_buffer: usize,
    current_side: CandleDirection,
    candles_elapsed_open_position: usize,
    candle_parser: TimeCandlestickGenerator,
    sma: SimpleMovingAverage,
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

        if self.candle_count < self.min_candles || !pre_signal || self.open_position.is_some() ||
            candlestick.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() > self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        // Keeping it simple. Maybe validate the distance between vwap and sma, or the sma angle in the future;
        if self.current_side == CandleDirection::SHORT && self.sma.turns_down() {
            self.open_position = Some(OpenPosition::new(
                Side::SELL,
                candlestick.close,
                self.points_per_tick_asset.unwrap(),
                QUANTITY_PER_POSITION
            ));
            return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION))
        }

        if self.current_side == CandleDirection::LONG && self.sma.turns_up() {
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

        if let Some(position) = self.open_position.as_mut() {
            self.candles_elapsed_open_position += 1;

            if self.current_side == CandleDirection::SHORT && self.sma.turns_up() {
                self.open_position = None;
                return Some(OrderAction::MarketLong(QUANTITY_PER_POSITION))
            }

            if self.current_side == CandleDirection::LONG && self.sma.turns_down() {
                self.open_position = None;
                return Some(OrderAction::MarketShort(QUANTITY_PER_POSITION))
            }
        }

        None
    }
}

impl HighResolutionTradingStrategy for VWAPDayInitStrategy {
    fn new() -> Self
    where
        Self: Sized
    {
        VWAPDayInitStrategy {
            points_per_tick_asset: None,
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(11, 30, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(17, 00, 00).unwrap(),
            min_candles: 10,
            candles_elapsed_open_position: 0,
            candle_count: 0,
            quadrant_changed_signal_buffer: 0,
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Minutes, CANDLESTICK_PERIOD),
            sma: SimpleMovingAverage::new(9).unwrap(),
            ema: ExponentialMovingAverage::new(9, 1.0),
            vwap: VolumeWeightedAveragePrice::new().unwrap(),
            current_side: CandleDirection::UNDEFINED,
            open_position: None,
        }
    }

    fn name(&self) -> &str {
        "VWAP_init_trading_day_strategy"
    }

    fn init_trading_day(&mut self, asset: &Asset, trades: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);
        self.candles_elapsed_open_position = 0;
        self.candle_count = 0;
        self.quadrant_changed_signal_buffer = 0;

        self.sma.reset();
        self.ema.reset();
        self.vwap.reset();

        // for trade in trades {
        //     if let Some(candle) = self.candle_parser.parse_only_closed_candles(trade) {
        //         self.sma.next(candle);
        //         self
        //     }
        // }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut order_open_position: Option<OrderAction> = None;
        let order_close_position: Option<OrderAction> = self.should_close_position(new_trade);

        if let Some(candlestick) = self.candle_parser.parse_only_closed_candles(new_trade) {
            self.candle_count += 1;

            let sma = self.sma.next(candlestick);
            let ema = self.ema.next(candlestick);
            let vwap = self.vwap.next(&candlestick);

            order_open_position = self.should_open_position(vwap, &candlestick);
        }

        TradeAction::custom_order(order_close_position, order_open_position, None, None)
    }
}

struct OpenPosition {
    side: Side,
    open_quantity: usize,
    quantity_per_chunk: [usize; 2],
    first_chunk: TrailingStop,
}

impl OpenPosition {
    fn new(
        side: Side,
        entry_price: f64,
        points_per_tick: f64,
        quantity_to_trade: usize,
    ) -> Self {
        assert!(quantity_to_trade % 2 == 0, "{} quantity must be divisible by 2", quantity_to_trade);
        OpenPosition {
            side,
            open_quantity: quantity_to_trade,
            quantity_per_chunk: [quantity_to_trade / 2, quantity_to_trade / 2],
            first_chunk: TrailingStop::new_not_automatic_trailing(
                entry_price,
                side,
                points_per_tick,
            ),
        }
    }
}