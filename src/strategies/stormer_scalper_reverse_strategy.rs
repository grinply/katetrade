use chrono::{NaiveTime, Timelike};
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::{Next, Reset};
use crate::indicators::relative_strength_index::RelativeStrengthIndex;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::indicators::stormer_indicator::StormerIndicator;
use crate::models::action::{market_order_from_side, OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::candlestick::CandleDirection::{LONG, SHORT};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::strategy::HighResolutionTradingStrategy;
use crate::utils::date;

const QUANTITY_PER_POSITION: usize = 20;

// scalping trailing stop loss configuration
const TICKS_STOP_LOSS: usize = 15;
const TICKS_FIRST_TARGET: usize = 35;
const HARD_STOP_CANDLESTICK_COUNT_TO_EXIT_POSITION: usize = 60;

pub struct StormerScalperReverseStrategy {
    quantity_per_position: usize,
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    // indicators & context variables used for candlesticks with period of 15 seconds
    min_candles: usize,
    candle_count: usize,
    open_position: Option<ScalperPosition>,

    stormer_indicator: StormerIndicator,
}

impl HighResolutionTradingStrategy for StormerScalperReverseStrategy {
    fn new() -> Self
    where
        Self: Sized,
    {
        StormerScalperReverseStrategy {
            quantity_per_position: QUANTITY_PER_POSITION,
            points_per_tick_asset: Some(5.0),
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(10, 30, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(10, 40, 00).unwrap(),
            candle_count: 0,
            min_candles: 21,
            open_position: None,
            stormer_indicator: StormerIndicator::default()
        }
    }

    fn name(&self) -> &str {
        "stormer_scalper_reverse_strategy"
    }

    fn days_of_context(&self) -> u32 {
        1
    }

    fn init_trading_day(&mut self, asset: &Asset, _: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);
        self.candle_count = 0;
        self.open_position = None;
        self.stormer_indicator.reset();
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let order_exit: Option<OrderAction> = self.should_exit_fast_position(new_trade);
        let mut order_enter: Option<OrderAction> = None;

        let indicator_output = self.stormer_indicator.next(new_trade);

        if self.open_position.is_none() && indicator_output.value > 0.0 && indicator_output.suggested_direction.is_some() {
            self.open_position = Some(ScalperPosition::new(
                indicator_output.suggested_direction.unwrap().opposite_side(),
                new_trade.price,
                self.points_per_tick_asset.unwrap(),
                self.quantity_per_position,
            ));

            order_enter = Some(match self.open_position.as_ref().unwrap().side {
                Side::BUY => OrderAction::MarketLong(self.quantity_per_position),
                Side::SELL => OrderAction::MarketShort(self.quantity_per_position),
            });
        }


        TradeAction::custom_order(order_exit, order_enter, None, None)
    }
}

impl StormerScalperReverseStrategy {
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

        if self.open_position.as_ref()?.position.is_finished() {
            let side = self.open_position.as_ref()?.side;
            let order = market_order_from_side(
                side.opposite_side(),
                self.quantity_per_position,
            );
            self.open_position = None;
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
    position: TrailingStop,
}

impl ScalperPosition {
    fn new(side: Side, entry_price: f64, points_per_tick: f64, quantity_to_trade: usize) -> Self {
        ScalperPosition {
            side,
            open_quantity: quantity_to_trade,
            position: TrailingStop::new_not_automatic_trailing(
                entry_price,
                side,
                points_per_tick,
            ),
        }
    }
}