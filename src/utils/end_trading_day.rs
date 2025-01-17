use crate::models::action::OrderAction;
use crate::models::side::Side;

pub(crate) fn should_force_close_position_end_trading_day(current_time: u32, side: Side, open_quantity: usize, force_close_time: u32) -> Option<OrderAction> {

    if current_time >= force_close_time {
        return if side == Side::BUY {
            Some(OrderAction::MarketShort(open_quantity))
        } else {
            Some(OrderAction::MarketLong(open_quantity))
        }
    }
    None
}