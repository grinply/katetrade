use chrono::{NaiveTime, Timelike};
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::{Next, Reset};
use crate::indicators::relative_strength_index::RelativeStrengthIndex;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::models::action::{market_order_from_side, OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::candlestick::CandleDirection::{LONG, SHORT};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::strategy::HighResolutionTradingStrategy;
use crate::utils::date;

const CANDLES_TO_TRACK: usize = 9;

const QUANTITY_PER_POSITION: usize = 2;
const CANDLESTICK_PERIOD: u64 = 10;
const BOLLINGER_PERIOD: usize = 21;
const BOLLINGER_MULTIPLIER: f64 = 0.45;
const SLOW_STOCHASTIC_PERIOD: usize = 8;
const SLOW_STOCHASTIC_SMA_PERIOD: usize = 3;
const STOCHASTIC_LONG_THRESHOLD: f64 = 80.0;
const STOCHASTIC_SHORT_THRESHOLD: f64 = 20.0;
const RSI_LONG_THRESHOLD: f64 = 70.0;
const RSI_SHORT_THRESHOLD: f64 = 30.0;
const RSI_PERIOD: usize = 9;

// scalping trailing stop loss configuration
const TICKS_STOP_LOSS: usize = 35;
const TICKS_FIRST_TARGET: usize = 15;
const TRAILING_SMA_PERIOD: usize = 5;
const HARD_STOP_CANDLESTICK_COUNT_TO_EXIT_POSITION: usize = 60;

pub struct StormerScalperStrategy {
    quantity_per_position: usize,
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    // indicators & context variables used for candlesticks with period of 15 seconds
    min_candles: usize,
    candle_count: usize,
    candles_elapsed_open_position: usize,
    bollinger_bands: BollingerBands,
    stochastic: SlowStochastic,
    open_position: Option<ScalperPosition>,
    candle_parser: TimeCandlestickGenerator,
    latest_closed_candles: [Candlestick; CANDLES_TO_TRACK],
    rsi: RelativeStrengthIndex,
    sma: SimpleMovingAverage,
}

impl HighResolutionTradingStrategy for StormerScalperStrategy {
    fn new() -> Self
    where
        Self: Sized,
    {
        StormerScalperStrategy {
            quantity_per_position: QUANTITY_PER_POSITION,
            points_per_tick_asset: Some(5.0),
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(10, 20, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(10, 40, 00).unwrap(),
            rsi: RelativeStrengthIndex::new(RSI_PERIOD),
            candle_count: 0,
            min_candles: 21,
            candles_elapsed_open_position: 0,
            bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, CANDLESTICK_PERIOD),
            latest_closed_candles: [Candlestick::default(); CANDLES_TO_TRACK],
            open_position: None,
            sma: SimpleMovingAverage::new(TRAILING_SMA_PERIOD).unwrap(),
        }
    }

    fn name(&self) -> &str {
        "stormer_scalper_strategy"
    }

    fn days_of_context(&self) -> u32 {
        0
    }

    fn init_trading_day(&mut self, asset: &Asset, trades: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);
        self.candle_count = 0;
        self.candles_elapsed_open_position = 0;

        self.rsi.reset();
        self.bollinger_bands.reset();
        self.stochastic.reset();
        self.latest_closed_candles = [Candlestick::default(); CANDLES_TO_TRACK];
        self.open_position = None;

        for trade in trades {
            if let Some(candle) = self.candle_parser.parse_only_closed_candles(trade) {
                self.rsi.next(candle.close);
                self.stochastic.next(candle.close);
                self.bollinger_bands.next(candle.close);
                self.sma.next(candle);
                self.update_candles(candle);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let fast_order_exit: Option<OrderAction> = self.should_exit_fast_position(new_trade);
        if let Some(candlestick) = self.candle_parser.parse_only_closed_candles(new_trade) {
            self.candle_count += 1;

            let stochastic = self.stochastic.next(candlestick.close);
            let bollinger_output = self.bollinger_bands.next(candlestick.close);
            let rsi = self.rsi.next(candlestick.close);
            let sma = self.sma.next(candlestick.close);
            self.update_candles(candlestick);
            fast_order_enter = self.should_open_position(stochastic, bollinger_output, rsi);

            if let Some(position) = self.open_position.as_mut() {
                self.candles_elapsed_open_position += 1;
                if position.first_chunk.is_finished() {
                    position.second_chunk.set_stop_loss(sma);
                }
            }
        }

        TradeAction::custom_order(fast_order_exit, fast_order_enter, None, None)
    }
}

impl StormerScalperStrategy {
    fn update_candles(&mut self, new_closed_candle: Candlestick) {
        for index in 0..CANDLES_TO_TRACK - 1 {
            self.latest_closed_candles[index] = self.latest_closed_candles[index + 1];
        }
        self.latest_closed_candles[CANDLES_TO_TRACK - 1] = new_closed_candle;
    }

    fn should_open_position(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput, rsi: f64) -> Option<OrderAction> {
        let first_candle = self.latest_closed_candles[CANDLES_TO_TRACK - 1];
        if self.candle_count < self.min_candles || self.open_position.is_some() ||
            first_candle.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() >= self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        let second_candle = self.latest_closed_candles[CANDLES_TO_TRACK - 2];
        let current_price = first_candle.close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            rsi < RSI_LONG_THRESHOLD &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            second_candle.direction() == SHORT &&
            first_candle.direction() == LONG {

            self.open_position = Some(ScalperPosition::new(
                Side::BUY,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketLong(self.quantity_per_position);
            self.candles_elapsed_open_position = 0;
            //println!("[{}|{}]sending market long {}", first_candle.date, first_candle.close_time?, first_candle.close);
            return Some(order);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            rsi > RSI_SHORT_THRESHOLD &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            second_candle.direction() == LONG &&
            first_candle.direction() == SHORT {

            self.open_position = Some(ScalperPosition::new(
                Side::SELL,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketShort(self.quantity_per_position);
            self.candles_elapsed_open_position = 0;
            //println!("[{}|{}]sending market short {}", first_candle.date, first_candle.close_time?, first_candle.close);
            return Some(order);
        }
        None
    }

    fn should_exit_fast_position(&mut self, new_trade: &Trade) -> Option<OrderAction> {
        if self.open_position.is_none() {
            return None;
        }

        if let Some(force_exit) = self.should_force_close_position_end_trading_day(
            new_trade,
            self.open_position.as_ref()?.side,
            self.open_position.as_ref()?.open_quantity,
        ) {
            self.open_position = None;
            return Some(force_exit);
        }

        let mut quantity_to_close: usize = 0;

        let is_first_chunk_finished = self.open_position.as_ref()?.first_chunk.is_finished();
        let should_close_first_chunk = self.open_position.as_mut()?.first_chunk.should_close_position(new_trade.price);

        if !is_first_chunk_finished && (should_close_first_chunk || self.candles_elapsed_open_position >= HARD_STOP_CANDLESTICK_COUNT_TO_EXIT_POSITION) {
            self.open_position.as_mut()?.first_chunk.finish_trailing();
            quantity_to_close += self.open_position.as_mut()?.quantity_per_chunk[0];
        }

        let is_second_chunk_finished = self.open_position.as_ref()?.second_chunk.is_finished();
        let should_close_second_chunk = self.open_position.as_mut()?.second_chunk.should_close_position(new_trade.price);

        if !is_second_chunk_finished && should_close_second_chunk {
            quantity_to_close += self.open_position.as_ref()?.quantity_per_chunk[1];
        }

        if quantity_to_close > 0 {
            let side = self.open_position.as_ref()?.side;
            self.open_position.as_mut()?.open_quantity -= quantity_to_close;
            if self.open_position.as_ref()?.open_quantity == 0 {
                self.open_position = None;
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
    quantity_per_chunk: [usize; 2],
    first_chunk: TrailingStop,
    second_chunk: TrailingStop,
}

impl ScalperPosition {
    fn new(
        side: Side,
        entry_price: f64,
        points_per_tick: f64,
        quantity_to_trade: usize,
    ) -> Self {
        assert!(quantity_to_trade % 2 == 0, "{} quantity must be divisible by 2", quantity_to_trade);
        ScalperPosition {
            side,
            open_quantity: quantity_to_trade,
            quantity_per_chunk: [quantity_to_trade / 2, quantity_to_trade / 2],
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
            second_chunk: TrailingStop::new_not_automatic_trailing(
                entry_price,
                side,
                points_per_tick,
            ),
        }
    }
}