#[derive(Debug)]
#[allow(unused)]
pub enum OrderAction {
    MarketLong,
    MarketShort,
    LimitLong(f64),
    LimitShort(f64),
}

#[derive(Debug)]
#[allow(unused)]
pub struct TradeAction {
    pub(crate) enter_market_action: Option<OrderAction>,
    pub(crate) stop_loss_action: Option<OrderAction>,
    pub(crate) take_profit_action: Option<OrderAction>,
}

impl TradeAction {

    /// single order to enter or exit the market
    pub fn send_order(order: OrderAction) ->  TradeAction{
        TradeAction{
            enter_market_action: Some(order),
            stop_loss_action: None,
            take_profit_action: None,
        }
    }

    pub fn send_order_with_stop_loss(enter_order: OrderAction, stop_order: OrderAction) ->  TradeAction{
        TradeAction{
            enter_market_action: Some(enter_order),
            stop_loss_action: Some(stop_order),
            take_profit_action: None,
        }
    }

    pub fn send_order_with_stop_loss_and_take_profit(enter_order: OrderAction,
                                                     stop_order: OrderAction,
                                                     take_profit_order: OrderAction) ->  TradeAction{
        TradeAction{
            enter_market_action: Some(enter_order),
            stop_loss_action: Some(stop_order),
            take_profit_action: Some(take_profit_order),
        }
    }

    pub fn do_nothing() -> TradeAction{
        TradeAction {
            enter_market_action: None,
            stop_loss_action: None,
            take_profit_action: None
        }
    }

}