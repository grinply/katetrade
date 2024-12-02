use chrono::{NaiveTime, Timelike};
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::Next;
use crate::models::action::{market_order_from_side, OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::candlestick::CandleDirection::{LONG, SHORT};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::strategy::HighResolutionTradingStrategy;
use crate::utils::date;

const BOLLINGER_PERIOD: usize = 21;
const BOLLINGER_MULTIPLIER: f64 = 0.38;
const SLOW_STOCHASTIC_PERIOD: usize = 8;
const SLOW_STOCHASTIC_SMA_PERIOD: usize = 3;
const STOCHASTIC_LONG_THRESHOLD: f64 = 80.0;
const STOCHASTIC_SHORT_THRESHOLD: f64 = 20.0;

// scalping trailing stop loss configuration
const TICKS_STOP_LOSS: usize = 13;
const TICKS_FIRST_TARGET: usize = 10;
const TICKS_SECOND_TARGET: usize = 13;
const TICKS_THIRD_TARGET: usize = 26;
const TICKS_START_TRAILING: usize = 5;
const TICKS_UPDATE_TRAILING_FREQUENCY: usize = 1;
const TICKS_DISTANCE_FROM_PRICE: usize = 2;

pub struct StormerScalperStrategy {
    quantity_per_position: usize,
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    // indicators & context variables used for candlesticks with period of 15 seconds
    min_fast_candles: usize,
    fast_candle_count: usize,
    fast_candle_bollinger_bands: BollingerBands,
    fast_candle_stochastic: SlowStochastic,
    fast_open_position: Option<ScalperPosition>,
    fast_candle_parser: TimeCandlestickGenerator,
    last_three_fast_closed_candles: [Candlestick; 3],
}

impl HighResolutionTradingStrategy for StormerScalperStrategy {
    fn new() -> Self
    where
        Self: Sized,
    {
        StormerScalperStrategy {
            quantity_per_position: 12,
            points_per_tick_asset: None,
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(17, 45, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(17, 55, 00).unwrap(),

            fast_candle_count: 0,
            min_fast_candles: 21,
            fast_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            fast_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            fast_candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, 15),
            last_three_fast_closed_candles: [Candlestick::default(); 3],
            fast_open_position: None,
        }
    }

    fn name(&self) -> &str {
        "stormer_scalper_strategy"
    }

    fn days_of_context(&self) -> u32 {
        1
    }

    fn init_trading_day(&mut self, asset: &Asset, trades: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);
        self.fast_candle_count = 0;

        let cut_off_time_send_orders = date::decrease_time(
            &asset.end_trading_time,
            10,
            TimeUnit::Minutes,
        );
        let time_to_close_open_positions = date::increase_time(
            &cut_off_time_send_orders,
            5,
            TimeUnit::Minutes,
        );
        self.cut_off_time_sending_orders = cut_off_time_send_orders;
        self.time_force_close_open_positions = time_to_close_open_positions;

        for trade in trades {
            if let Some(fast_candle) = self.fast_candle_parser.parse_only_closed_candles(trade) {
                self.fast_candle_stochastic.next(fast_candle.close);
                self.fast_candle_bollinger_bands.next(fast_candle.close);
                self.update_fast_candles(fast_candle);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let fast_order_exit: Option<OrderAction> = self.should_exit_fast_position(new_trade);

        if let Some(fast_candle) = self.fast_candle_parser.parse_only_closed_candles(new_trade) {
            self.fast_candle_count +=1;
            let stochastic = self.fast_candle_stochastic.next(fast_candle.close);
            let bollinger_output = self.fast_candle_bollinger_bands.next(fast_candle.close);
            self.update_fast_candles(fast_candle);
            fast_order_enter = self.should_open_position_fast_scalp(stochastic, bollinger_output);
        }

        TradeAction::custom_order(fast_order_exit, fast_order_enter, None, None)
    }
}

impl StormerScalperStrategy {
    fn update_fast_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_fast_closed_candles[0] = self.last_three_fast_closed_candles[1];
        self.last_three_fast_closed_candles[1] = self.last_three_fast_closed_candles[2];
        self.last_three_fast_closed_candles[2] = new_closed_candle;
    }

    fn should_open_position_fast_scalp(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        let latest_candle = self.last_three_fast_closed_candles[2];
        if self.fast_candle_count < self.min_fast_candles || self.fast_open_position.is_some() ||
            latest_candle.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() >= self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        let first_candle_direction = latest_candle.direction();
        let second_candle_direction = self.last_three_fast_closed_candles[1].direction();
        let current_price = latest_candle.close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            second_candle_direction == SHORT &&
            first_candle_direction == LONG {
            self.fast_open_position = Some(ScalperPosition::new(
                Side::BUY,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketLong(self.quantity_per_position);
            return Some(order);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            second_candle_direction == LONG &&
            first_candle_direction == SHORT {
            self.fast_open_position = Some(ScalperPosition::new(
                Side::SELL,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketShort(self.quantity_per_position);
            return Some(order);
        }
        None
    }

    fn should_exit_fast_position(&mut self, new_trade: &Trade) -> Option<OrderAction> {
        if self.fast_open_position.is_none() {
            return None;
        }

        if let Some(force_exit) = self.should_force_close_position_end_trading_day(
            new_trade,
            self.fast_open_position.as_ref()?.side,
            self.fast_open_position.as_ref()?.open_quantity,
        ) {
            self.fast_open_position = None;
            return Some(force_exit);
        }


        let mut quantity_to_close: usize = 0;

        let is_first_chunk_finished = self.fast_open_position.as_ref()?.first_chunk.is_finished();
        let should_close_first_chunk = self.fast_open_position.as_mut()?.first_chunk.should_close_position(new_trade.price);

        if !is_first_chunk_finished && should_close_first_chunk {
            quantity_to_close += self.fast_open_position.as_mut()?.quantity_per_chunk[0];
        }

        let is_second_chunk_finished = self.fast_open_position.as_ref()?.second_chunk.is_finished();
        let should_close_second_chunk = self.fast_open_position.as_mut()?.second_chunk.should_close_position(new_trade.price);

        if !is_second_chunk_finished && should_close_second_chunk {
            quantity_to_close += self.fast_open_position.as_ref()?.quantity_per_chunk[1];
        }

        let is_final_chunk_finished = self.fast_open_position.as_ref()?.final_chunk.is_finished();
        let should_close_final_chunk = self.fast_open_position.as_mut()?.final_chunk.should_close_position(new_trade.price);

        if !is_final_chunk_finished && should_close_final_chunk {
            quantity_to_close += self.fast_open_position.as_ref()?.quantity_per_chunk[2];
        }

        if quantity_to_close > 0 {
            let side = self.fast_open_position.as_ref()?.side;
            self.fast_open_position.as_mut()?.open_quantity-= quantity_to_close;
            if self.fast_open_position.as_ref()?.open_quantity == 0 {
                self.fast_open_position = None;
            }

            let order = market_order_from_side(
                side.opposite_side(),
                quantity_to_close,
            );

            return Some(order);
        }

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
            return Some(market_order_from_side(
                side.opposite_side(),
                open_quantity)
            );
        }
        None
    }
}

struct ScalperPosition {
    side: Side,
    open_quantity: usize,
    quantity_per_chunk: [usize; 3],
    first_chunk: TrailingStop,
    second_chunk: TrailingStop,
    final_chunk: TrailingStop,
}

impl ScalperPosition {
    fn new(side: Side, entry_price: f64, points_per_tick: f64, quantity_to_trade: usize) -> Self {
        let qty_per_chunk = quantity_to_trade / 3;
        ScalperPosition {
            side,
            open_quantity: quantity_to_trade,
            // if the amount of contracts is not evenly divisible we add the extra contracts in the first chunk to minimize risk
            quantity_per_chunk: [qty_per_chunk + (quantity_to_trade % 3), qty_per_chunk, qty_per_chunk],
            first_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                TICKS_STOP_LOSS,
                TICKS_FIRST_TARGET,
                1,
                1,
                1,
            ),
            second_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                TICKS_STOP_LOSS,
                TICKS_SECOND_TARGET,
                TICKS_START_TRAILING,
                TICKS_DISTANCE_FROM_PRICE,
                TICKS_UPDATE_TRAILING_FREQUENCY,
            ),
            final_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                TICKS_STOP_LOSS,
                TICKS_THIRD_TARGET,
                TICKS_START_TRAILING,
                TICKS_DISTANCE_FROM_PRICE,
                TICKS_UPDATE_TRAILING_FREQUENCY,
            ),
        }
    }
}