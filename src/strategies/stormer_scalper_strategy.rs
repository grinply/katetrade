use chrono::{NaiveTime, Timelike};
use itertools::enumerate;
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

const QUANTITY_PER_POSITION: usize = 8;
const CANDLESTICK_PERIOD: u64 = 15;
const BOLLINGER_PERIOD: usize = 21;
const BOLLINGER_MULTIPLIER: f64 = 0.38;
const SLOW_STOCHASTIC_PERIOD: usize = 8;
const SLOW_STOCHASTIC_SMA_PERIOD: usize = 3;
const STOCHASTIC_LONG_THRESHOLD: f64 = 80.0;
const STOCHASTIC_SHORT_THRESHOLD: f64 = 20.0;


// scalping trailing stop loss configuration
const TICKS_FIRST_TARGET: usize = 8;
const TICKS_SECOND_TARGET: usize = 13; // actually + TICKS_START_TRAILING
const TICKS_THIRD_TARGET: usize = 20; // actually + TICKS_START_TRAILING
const TICKS_START_TRAILING: usize = 1;
const TICKS_UPDATE_TRAILING_FREQUENCY: usize = 1;
const TICKS_DISTANCE_FROM_PRICE: usize = 4;

pub struct StormerScalperStrategy {
    quantity_per_position: usize,
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    // indicators & context variables used for candlesticks with period of 15 seconds
    min_candles: usize,
    candle_count: usize,
    bollinger_bands: BollingerBands,
    stochastic: SlowStochastic,
    open_positions: Vec<ScalperPosition>,
    candle_parser: TimeCandlestickGenerator,
    last_three_closed_candles: [Candlestick; 3],
}

impl HighResolutionTradingStrategy for StormerScalperStrategy {
    fn new() -> Self
    where
        Self: Sized,
    {
        StormerScalperStrategy {
            quantity_per_position: QUANTITY_PER_POSITION,

            points_per_tick_asset: Some(5.0),
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(10, 25, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(11, 00, 00).unwrap(),
            candle_count: 0,
            min_candles: 21,
            bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, CANDLESTICK_PERIOD),
            last_three_closed_candles: [Candlestick::default(); 3],
            open_positions: Vec::new(),
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
        self.candle_count = 0;

        for trade in trades {
            if let Some(fast_candle) = self.candle_parser.parse_only_closed_candles(trade) {
                self.stochastic.next(fast_candle.close);
                self.bollinger_bands.next(fast_candle.close);
                self.update_fast_candles(fast_candle);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let fast_order_exit: Option<OrderAction> = self.should_exit_position(new_trade);

        if let Some(fast_candle) = self.candle_parser.parse_only_closed_candles(new_trade) {
            self.candle_count += 1;
            let stochastic = self.stochastic.next(fast_candle.close);
            let bollinger_output = self.bollinger_bands.next(fast_candle.close);
            self.update_fast_candles(fast_candle);
            fast_order_enter = self.should_open_position(stochastic, bollinger_output);
        }

        TradeAction::custom_order(fast_order_exit, fast_order_enter, None, None)
    }
}

impl StormerScalperStrategy {
    fn update_fast_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_closed_candles[0] = self.last_three_closed_candles[1];
        self.last_three_closed_candles[1] = self.last_three_closed_candles[2];
        self.last_three_closed_candles[2] = new_closed_candle;
    }

    fn should_open_position(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        let first_candle = self.last_three_closed_candles[2];
        if self.candle_count < self.min_candles ||
            first_candle.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() >= self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        let second_candle = self.last_three_closed_candles[1];
        let current_price = first_candle.close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            second_candle.direction() == SHORT &&
            first_candle.direction() == LONG {
            for position in self.open_positions.iter_mut() {
                if current_price > position.entry_price && !position.first_chunk.is_finished(){
                    position.first_chunk.start_trailing_now(current_price);
                }
            }

            let mut position = ScalperPosition::new(
                Side::BUY,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            );

            Self::set_all_stop_loses(&mut position, bollinger_output.lower);
            self.open_positions.push(position);
            return Some(OrderAction::MarketLong(self.quantity_per_position));
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            second_candle.direction() == LONG &&
            first_candle.direction() == SHORT {

            for position in self.open_positions.iter_mut() {
                if current_price < position.entry_price && !position.first_chunk.is_finished(){
                    position.first_chunk.start_trailing_now(current_price);
                }
            }

            let mut position = ScalperPosition::new(
                Side::SELL,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            );
            Self::set_all_stop_loses(&mut position, bollinger_output.upper);
            self.open_positions.push(position);
            return Some(OrderAction::MarketShort(self.quantity_per_position));
        }
        None
    }

    fn set_all_stop_loses(position: &mut ScalperPosition, new_stop_loss_price: f64) {
        position.first_chunk.set_stop_loss(new_stop_loss_price);
        position.second_chunk.set_stop_loss(new_stop_loss_price);
        position.final_chunk.set_stop_loss(new_stop_loss_price);
    }

    fn should_exit_position(&mut self, new_trade: &Trade) -> Option<OrderAction> {
        if self.open_positions.is_empty() {
            return None;
        }

        if let Some(close_action) = self.should_force_close_position_end_trading_day(new_trade) {
            return Some(close_action);
        }

        let mut positions_to_remove: Vec<usize> = vec![];
        let mut total_quantity_to_close: usize = 0;

        let side = self.open_positions.first()?.side;
        for (index_position, open_position) in enumerate(self.open_positions.iter_mut()) {
            let mut quantity_to_close: usize = 0;

            let is_first_chunk_finished = open_position.first_chunk.is_finished();
            let should_close_first_chunk = open_position.first_chunk.should_close_position(new_trade.price);

            if !is_first_chunk_finished && should_close_first_chunk {
                quantity_to_close += open_position.quantity_per_chunk[0];
            }

            let is_second_chunk_finished = open_position.second_chunk.is_finished();
            let should_close_second_chunk = open_position.second_chunk.should_close_position(new_trade.price);

            if !is_second_chunk_finished && should_close_second_chunk {
                quantity_to_close += open_position.quantity_per_chunk[1];
            }

            let is_final_chunk_finished = open_position.final_chunk.is_finished();
            let should_close_final_chunk = open_position.final_chunk.should_close_position(new_trade.price);

            if !is_final_chunk_finished && should_close_final_chunk {
                quantity_to_close += open_position.quantity_per_chunk[2];
            }

            if quantity_to_close > 0 {
                open_position.open_quantity -= quantity_to_close;
                if open_position.open_quantity == 0 {
                    positions_to_remove.push(index_position);
                }

                total_quantity_to_close += quantity_to_close;
            }
        }

        let mut index_offset = 0;
        for index in positions_to_remove {
            if index != 0 {
                self.open_positions.remove(index - index_offset);
            }else{
                self.open_positions.remove(0);
            }
            index_offset+=1
        }

        if total_quantity_to_close > 0 {
            return Some(market_order_from_side(
                side.opposite_side(),
                total_quantity_to_close,
            ));
        }

        None
    }

    fn should_force_close_position_end_trading_day(&self, new_trade: &Trade) -> Option<OrderAction> {
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
            let side = self.open_positions.first()?.side;
            let open_quantity = self.open_positions.iter()
                .fold(0, |total_open_quantity, position|
                    total_open_quantity + position.open_quantity);

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
    entry_price: f64,
    open_quantity: usize,
    quantity_per_chunk: [usize; 3],
    first_chunk: TrailingStop,
    second_chunk: TrailingStop,
    final_chunk: TrailingStop,
}

impl ScalperPosition {
    fn new(side: Side, entry_price: f64, points_per_tick: f64, quantity_to_trade: usize) -> Self {
        assert!(quantity_to_trade % 4 == 0);

        ScalperPosition {
            side,
            entry_price,
            open_quantity: quantity_to_trade,

            quantity_per_chunk: [quantity_to_trade / 2, quantity_to_trade / 4, quantity_to_trade / 4],
            first_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                50, // this will be overwritten by set_all_stop_loses function
                TICKS_FIRST_TARGET,
                1,
                1,
                1,
            ),
            second_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                50, // this will be overwritten by set_all_stop_loses function
                TICKS_SECOND_TARGET,
                TICKS_START_TRAILING,
                TICKS_DISTANCE_FROM_PRICE,
                TICKS_UPDATE_TRAILING_FREQUENCY,
            ),
            final_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                50, // this will be overwritten by set_all_stop_loses function
                TICKS_THIRD_TARGET,
                TICKS_START_TRAILING,
                TICKS_DISTANCE_FROM_PRICE,
                TICKS_UPDATE_TRAILING_FREQUENCY,
            ),
        }
    }
}