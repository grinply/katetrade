use crate::models::action::OrderAction::{Abort, MarketLong, MarketShort};
use crate::models::side::Side;

#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(unused)]
pub enum OrderAction {
    MarketLong(usize),     // will execute immediately a long position at the market
    MarketShort(usize),    // will execute immediately a short position at the market
    LimitLong(f64, usize), // will send a long limit order to the exchange order book that MAY ( or may not)  be triggered later with the given price
    LimitShort(f64, usize), // will send a short limit order to the exchange order book that MAY ( or may not)  be triggered later with the given price
    Abort,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(unused)]
pub struct TradeAction {
    pub orders: [Option<OrderAction>; 5],
}

pub fn market_order_from_side(side: Side, qty: usize) -> OrderAction {
    match side {
        Side::BUY => MarketLong(qty),
        Side::SELL => MarketShort(qty)
    }
}

impl TradeAction {
    /// single order to enter or exit the market
    pub fn send_order(order: OrderAction) -> TradeAction {
        TradeAction {
            orders: [Some(order), None, None, None, None],
        }
    }

    #[allow(unused)]
    pub fn send_order_with_stop_loss(enter_order: OrderAction, stop_order: OrderAction) -> TradeAction {
        TradeAction {
            orders: [Some(enter_order), Some(stop_order), None, None, None],
        }
    }

    #[allow(unused)]
    pub fn send_order_with_stop_loss_and_take_profit(enter_order: OrderAction,
                                                     stop_order: OrderAction,
                                                     take_profit_order: OrderAction) -> TradeAction {
        TradeAction {
            orders: [Some(enter_order), Some(stop_order), Some(take_profit_order), None, None],
        }
    }

    #[allow(unused)]
    pub fn do_nothing() -> TradeAction {
        TradeAction {
            orders: [None; 5]
        }
    }

    pub fn custom_order(
        first_order: Option<OrderAction>,
        second_order: Option<OrderAction>,
        third_order: Option<OrderAction>,
        fourth_order: Option<OrderAction>,
    ) -> TradeAction {
        TradeAction {
            orders: [first_order, second_order, third_order, fourth_order, None]
        }
    }

    #[allow(unused)]
    pub fn abort() -> TradeAction {
        TradeAction {
            orders: [Some(Abort); 5]
        }
    }
}