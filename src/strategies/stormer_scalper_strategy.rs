use chrono::{NaiveTime, Timelike};
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::Next;
use crate::models::action;
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

// regular trade configuration
const REGULAR_TICKS_PROFIT: usize = 62;
const REGULAR_TICKS_LOSS: usize = 31;

// scalping trailing stop loss configuration
const TICKS_STOP_LOSS: usize = 8;
const TICKS_FIRST_TARGET: usize = 8;
const TICKS_SECOND_TARGET: usize = 13;
const TICKS_THIRD_TARGET: usize = 20;
const TICKS_START_TRAILING: usize = 5;
const TICKS_UPDATE_TRAILING_FREQUENCY: usize = 1;
const TICKS_DISTANCE_FROM_PRICE: usize = 2;

pub struct StormerScalperStrategy {
    quantity_per_position: usize,
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    // indicators & context variables used for candlesticks with period of 15 seconds
    fast_candle_bollinger_bands: BollingerBands,
    fast_candle_stochastic: SlowStochastic,
    fast_open_position: Option<ScalperPosition>,
    fast_candle_parser: TimeCandlestickGenerator,
    last_three_fast_closed_candles: [Candlestick; 3],

    // indicators & context variables used for candlesticks with period of 6 minutes
    slow_candle_bollinger_bands: BollingerBands,
    slow_candle_stochastic: SlowStochastic,
    slow_open_position: Option<RegularPosition>,
    slow_candle_parser: TimeCandlestickGenerator,
    last_three_slow_closed_candles: [Candlestick; 3],
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

            fast_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            fast_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            fast_candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, 15),
            last_three_fast_closed_candles: [Candlestick::default(); 3],
            fast_open_position: None,

            slow_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            slow_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            slow_candle_parser: TimeCandlestickGenerator::new(TimeUnit::Minutes, 6),
            last_three_slow_closed_candles: [Candlestick::default(); 3],
            slow_open_position: None,
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
        let cut_off_time_send_orders = date::decrease_time(
            &asset.end_trading_time,
            15,
            TimeUnit::Minutes,
        );
        let time_to_close_open_positions = date::increase_time(
            &cut_off_time_send_orders,
            10,
            TimeUnit::Minutes,
        );
        self.cut_off_time_sending_orders = cut_off_time_send_orders;
        self.time_force_close_open_positions = time_to_close_open_positions;
        for trade in trades {
            // update indicators of 15 seconds
            if let Some(fast_candle) = self.fast_candle_parser.parse_only_closed_candles(trade) {
                self.fast_candle_stochastic.next(fast_candle.close);
                self.fast_candle_bollinger_bands.next(fast_candle.close);
                self.update_fast_candles(fast_candle);
            }

            // update indicators of 6 minutes
            if let Some(slow_candle) = self.slow_candle_parser.parse_only_closed_candles(trade) {
                self.slow_candle_stochastic.next(slow_candle.close);
                self.slow_candle_bollinger_bands.next(slow_candle.close);
                self.update_slow_candles(slow_candle);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let mut slow_order_enter: Option<OrderAction> = None;

        let fast_order_exit: Option<OrderAction> = self.should_exit_fast_position(new_trade);
        let slow_order_exit: Option<OrderAction> = self.should_exit_slow_position(new_trade);

        if let Some(fast_candle) = self.fast_candle_parser.parse_only_closed_candles(new_trade) {
            let stochastic = self.fast_candle_stochastic.next(fast_candle.close);
            let bollinger_output = self.fast_candle_bollinger_bands.next(fast_candle.close);
            self.update_fast_candles(fast_candle);
            fast_order_enter = self.should_open_position_fast_scalp(stochastic, bollinger_output);
        }

        if let Some(slow_candle) = self.slow_candle_parser.parse_only_closed_candles(new_trade) {
            let stochastic = self.slow_candle_stochastic.next(slow_candle.close);
            let bollinger_output = self.slow_candle_bollinger_bands.next(slow_candle.close);
            self.update_slow_candles(slow_candle);
            slow_order_enter = self.should_open_position_slow_scalp(stochastic, bollinger_output);
        }

        TradeAction::custom_order(fast_order_exit, slow_order_exit, fast_order_enter, slow_order_enter)
    }
}

impl StormerScalperStrategy {
    fn update_fast_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_fast_closed_candles[0] = self.last_three_fast_closed_candles[1];
        self.last_three_fast_closed_candles[1] = self.last_three_fast_closed_candles[2];
        self.last_three_fast_closed_candles[2] = new_closed_candle;
    }

    fn update_slow_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_slow_closed_candles[0] = self.last_three_slow_closed_candles[1];
        self.last_three_slow_closed_candles[1] = self.last_three_slow_closed_candles[2];
        self.last_three_slow_closed_candles[2] = new_closed_candle;
    }

    fn should_open_position_fast_scalp(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        let latest_candle = self.last_three_fast_closed_candles[2];
        if self.fast_open_position.is_some() ||
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
            first_candle_direction == SHORT &&
            second_candle_direction == LONG {
            self.fast_open_position = Some(ScalperPosition::new(
                Side::BUY,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketLong(self.quantity_per_position);
            println!("[{}] executing {:?} scalp order at {} with price {:.2}",
                     latest_candle.date, order, latest_candle.close_time.unwrap(), latest_candle.close);
            return Some(order);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            first_candle_direction == LONG &&
            second_candle_direction == SHORT {
            self.fast_open_position = Some(ScalperPosition::new(
                Side::SELL,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketShort(self.quantity_per_position);
            println!("[{}] executing {:?} scalp order at {} with price {:.2}",
                     latest_candle.date, order, latest_candle.close_time.unwrap(), latest_candle.close);
            return Some(order);
        }

        None
    }

    fn should_open_position_slow_scalp(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        let latest_candle = self.last_three_slow_closed_candles[2];
        if self.slow_open_position.is_some() ||
            latest_candle.close_time.expect("candlestick should be closed").num_seconds_from_midnight() >=
                self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        let first_candle_direction = latest_candle.direction();
        let second_candle_direction = self.last_three_slow_closed_candles[1].direction();
        let current_price = latest_candle.close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            first_candle_direction == SHORT &&
            second_candle_direction == LONG {
            self.slow_open_position = Some(RegularPosition::new(
                Side::BUY,
                self.points_per_tick_asset?,
                current_price,
                REGULAR_TICKS_PROFIT,
                REGULAR_TICKS_LOSS,
                self.quantity_per_position,
            ));

            let order = OrderAction::MarketLong(self.quantity_per_position);
            println!("[{}] executing {:?} slow order at {} with price {:.2}",
                     latest_candle.date, order, latest_candle.close_time.unwrap(), latest_candle.close);
            return Some(order);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            first_candle_direction == LONG &&
            second_candle_direction == SHORT {
            self.slow_open_position = Some(RegularPosition::new(
                Side::SELL,
                self.points_per_tick_asset?,
                current_price,
                REGULAR_TICKS_PROFIT,
                REGULAR_TICKS_LOSS,
                self.quantity_per_position,
            ));

            let order = OrderAction::MarketShort(self.quantity_per_position);
            println!("[{}] executing {:?} slow order at {} with price {:.2}",
                     latest_candle.date, order, latest_candle.close_time.unwrap(), latest_candle.close);
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
            println!("[{}] compulsory close {:?} scalp order at {} with price {:.2}",
                     new_trade.date, force_exit, date::convert_to_naive_time(new_trade.time).unwrap(), new_trade.time);
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
            println!("[{}] closing {:?} scalp order at {} with price {:.2}",
                     new_trade.date, order, date::convert_to_naive_time(new_trade.time).unwrap(), new_trade.time);
            return Some(order);
        }

        None
    }

    fn should_exit_slow_position(&mut self, new_trade: &Trade) -> Option<OrderAction> {
        if self.slow_open_position.is_none() {
            return None;
        }

        if let Some(force_exit) = self.should_force_close_position_end_trading_day(
            new_trade,
            self.slow_open_position.as_ref()?.side,
            self.slow_open_position.as_ref()?.open_quantity,
        ) {
            self.slow_open_position = None;

            println!("[{}] compulsory close {:?} slow order at {} with price {:.2}",
                     new_trade.date, force_exit, date::convert_to_naive_time(new_trade.time).unwrap(), new_trade.time);
            return Some(force_exit);
        }

        let open_position = self.slow_open_position.as_ref()?;

        if open_position.should_close_position(new_trade.price) {
            let order = market_order_from_side(
                open_position.side,
                open_position.open_quantity,
            );
            let close_order = Some(order);
            self.slow_open_position = None;
            println!("[{}] closing {:?} slow order at {} with price {:.2}",
                     new_trade.date, order, date::convert_to_naive_time(new_trade.time).unwrap(), new_trade.time);
            return close_order;
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



struct RegularPosition {
    side: Side,
    open_quantity: usize,
    target_profit: f64,
    stop_loss: f64,
}

struct ScalperPosition {
    side: Side,
    open_quantity: usize,
    quantity_per_chunk: [usize; 3],
    first_chunk: TrailingStop,
    second_chunk: TrailingStop,
    final_chunk: TrailingStop,
}

impl RegularPosition {
    fn new(side: Side, points_per_tick: f64, entry_price: f64, ticks_profit: usize, ticks_loss: usize, quantity_to_trade: usize) -> Self {
        match side {
            Side::BUY => {
                RegularPosition {
                    side,
                    open_quantity: quantity_to_trade,
                    target_profit: entry_price + (points_per_tick * ticks_profit as f64),
                    stop_loss: entry_price - (points_per_tick * ticks_loss as f64),
                }
            }
            Side::SELL => {
                RegularPosition {
                    side,
                    open_quantity: quantity_to_trade,
                    target_profit: entry_price - (points_per_tick * ticks_profit as f64),
                    stop_loss: entry_price + (points_per_tick * ticks_loss as f64),
                }
            }
        }
    }

    fn should_close_position(&self, current_price: f64) -> bool {
        match self.side {
            Side::BUY => {
                if current_price >= self.target_profit || current_price <= self.stop_loss {
                    true
                } else {
                    false
                }
            }
            Side::SELL => {
                if current_price <= self.target_profit || current_price >= self.stop_loss {
                    true
                } else {
                    false
                }
            }
        }
    }
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
                0,
                0,
                0,
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