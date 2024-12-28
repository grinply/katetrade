use chrono::{NaiveTime, Timelike};
use crate::models::action::{market_order_from_side, OrderAction};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::trailing_stop::TrailingStop;
use crate::utils::date;

pub struct OpenPosition {
    pub(crate) side: Side,
    pub(crate) open_quantity: usize,
    quantity_per_chunk: Vec<usize>,
    action_per_chunk: Vec<TrailingStop>
}

impl OpenPosition {
    pub fn new(
        side: Side,
        quantity_to_trade: usize,
        number_of_orders: usize,
        action_per_chunk: Vec<TrailingStop>,
    ) -> OpenPosition {
        assert_eq!(quantity_to_trade % number_of_orders, 0, "{} quantity must be divisible by the number of orders", quantity_to_trade);
        assert_eq!(action_per_chunk.len(), number_of_orders, "Action per chunk must have the same number of orders");
        let quantity_per_chunk_unit = quantity_to_trade / number_of_orders;

        OpenPosition {
            side,
            open_quantity: quantity_to_trade,
            quantity_per_chunk: vec![quantity_per_chunk_unit; number_of_orders],
            action_per_chunk
        }
    }

    pub fn new_not_automatic_trailing(
        side: Side,
        quantity_to_trade: usize,
        number_of_orders: usize,
        entry_price: f64,
        points_per_tick: f64
    ) -> OpenPosition {
        assert_eq!(quantity_to_trade % number_of_orders, 0, "{} quantity must be divisible by the number of orders", quantity_to_trade);
        let quantity_per_chunk_unit = quantity_to_trade / number_of_orders;

        OpenPosition {
            side,
            open_quantity: quantity_to_trade,
            quantity_per_chunk: vec![quantity_per_chunk_unit; number_of_orders],
            action_per_chunk: vec![TrailingStop::new_not_automatic_trailing(entry_price, side, points_per_tick); number_of_orders]
        }
    }


    pub(crate) fn should_force_close_position_end_trading_day(new_trade: &Trade, side: Side, open_quantity: usize, time_force_close_open_positions: NaiveTime) -> Option<OrderAction> {
        let current_time = date::convert_to_naive_time(new_trade.time);
        assert!(
            current_time.is_some(),
            "should never fail when converting a numeric time from an trade into NaiveTime. time from trade: {}", new_trade.time
        );
        let current_time = current_time?;

        if current_time.num_seconds_from_midnight() >= time_force_close_open_positions.num_seconds_from_midnight() {
            return Some(market_order_from_side(
                side.opposite_side(),
                open_quantity)
            );
        }
        None
    }
}