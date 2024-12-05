use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::Candlestick;
use crate::models::market::Market;
use crate::models::pnl::Position;
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::backtest::backtest::DEBUG_BACKTEST_LOG;
use crate::services::backtest::data_point::DataPoint;

#[allow(unused)]
use crate::strategies::strategy::{CandlestickTradingStrategy, HighResolutionTradingStrategy};
use crate::utils::date;

pub const INITIAL_BALANCE: f64 = 500_000.0;
pub const MINIMUM_BALANCE: f64 = 800.0;
const DEFAULT_SLIPPAGE_MULTIPLIER: f64 = 0.0001;
const COMPULSORY_CLOSE_FEE: f64 = 25.0;


/// [ExchangeEventHandler] is the core component that emulates a real broker/exchange
/// when running backtest simulations on historical data to evaluate performance of trading strategies.
///
/// There are two types of strategies that can be used with [ExchangeEventHandler]:
/// + [CandlestickTradingStrategy] that operates using closed [Candlestick]
/// + [HighResolutionTradingStrategy] that operates using raw [Trade]
#[allow(unused)]
pub(crate) struct ExchangeEventHandler<'a> {
    asset: &'a Asset,
    pub(crate) current_balance: f64,
    slippage_multiplier: f64, // defaults to 0.0001
    initial_qty_per_order: u32,
    base_qty_per_order: u32,
    total_candlesticks_processed: u64,
    pub(crate) current_position: Option<Position>,
    // scheduled_orders: Vec<TradeEvent<'a>>,
    event_index: usize,
    pub(crate) positions: Vec<Position>,
    total_contracts_executed_current_month: u64,
    current_month_year: i32,
    current_datapoint: Candlestick,
    pub(crate) last_datapoint: Candlestick,
    fixed_traded_qty: bool,
    pub(crate) removed_for_lack_of_margin: bool,
}

#[derive(Copy, Clone, Debug)]
#[allow(unused)]
enum TradeEvent<'b> {
    NewCandle(&'b Candlestick),
    NewTrade(&'b Trade),
    ExecuteOrderLong(usize),
    ExecuteOrderShort(usize),
    ZeroPosition,
    ScheduleOrderLong(f64, usize),
    ScheduleOrderShort(f64, usize),
    Abort,
    None,
}

impl<'a> ExchangeEventHandler<'a> {
    pub(crate) fn new(asset: &'a Asset, base_qty_to_trade: u32, fixed_traded_qty: bool) -> Self {
        ExchangeEventHandler {
            asset,
            current_position: None,
            current_datapoint: Candlestick::default(),
            last_datapoint: Candlestick::default(),
            current_balance: INITIAL_BALANCE,
            base_qty_per_order: base_qty_to_trade,
            initial_qty_per_order: base_qty_to_trade,
            slippage_multiplier: DEFAULT_SLIPPAGE_MULTIPLIER,
            total_candlesticks_processed: 0,
            event_index: 0,
            positions: vec![],
            total_contracts_executed_current_month: 0,
            current_month_year: -1,
            fixed_traded_qty,
            removed_for_lack_of_margin: false,
        }
    }

    pub(crate) fn process_trade(&mut self, trade: &Trade) {
        let event = TradeEvent::NewTrade(trade);
        self.process_events(event);
    }

    pub(crate) fn process_candlestick(&mut self, candlestick: &Candlestick) {
        let event = TradeEvent::NewCandle(candlestick);
        self.process_events(event);
    }

    pub(crate) fn process_orders(&mut self, action: TradeAction) {
        for order_option in action.orders {
            if let Some(order) = order_option {
                let event = match order {
                    OrderAction::MarketLong(qty) => TradeEvent::ExecuteOrderLong(qty),
                    OrderAction::MarketShort(qty) => TradeEvent::ExecuteOrderShort(qty),
                    OrderAction::LimitLong(price, qty) => TradeEvent::ScheduleOrderLong(price, qty),
                    OrderAction::LimitShort(price, qty) => TradeEvent::ScheduleOrderShort(price, qty),
                    OrderAction::Abort => TradeEvent::Abort
                };
                self.process_events(event);
            }
        }
    }

    fn process_events(&mut self, event: TradeEvent) {
        match event {
            TradeEvent::NewTrade(trade) => {
                self.update_internal_candlestick_with_trade(trade);
                self.new_datapoint(trade);
                self.track_current_month();
            }
            TradeEvent::NewCandle(candlestick) => {
                self.update_internal_candlesticks(candlestick);
                self.new_datapoint(candlestick);
                self.track_current_month();
            }
            TradeEvent::ExecuteOrderLong(qty) => {
                self.execute_market_long_order(qty);
            }
            TradeEvent::ExecuteOrderShort(qty) => {
                self.execute_market_short_order(qty);
            }
            TradeEvent::ZeroPosition => {
                self.zero_current_position();
            }
            TradeEvent::ScheduleOrderLong(_, _) | TradeEvent::ScheduleOrderShort(_, _) => {
                panic!("operation not allowed");
            }
            TradeEvent::Abort => {
                self.removed_for_lack_of_margin = true;
            }
            TradeEvent::None => (),
        }
    }

    fn new_datapoint(&mut self, new_datapoint: &impl DataPoint) {
        self.should_updated_traded_qty(new_datapoint);
        self.check_compulsory_close_position(new_datapoint);
        self.check_scheduled_order_trigger(new_datapoint);
        self.track_mark_to_market(new_datapoint);
    }

    fn update_internal_candlestick_with_trade(&mut self, new_trade: &Trade) {
        self.last_datapoint = self.current_datapoint;
        self.current_datapoint.date = new_trade.date;
        self.current_datapoint.start_time = date::convert_to_naive_time(new_trade.time).unwrap();
        self.current_datapoint.close_time = Some(self.current_datapoint.start_time);
        self.current_datapoint.open = new_trade.price;
        self.current_datapoint.high = new_trade.price;
        self.current_datapoint.low = new_trade.price;
        self.current_datapoint.close = new_trade.price;
        self.current_datapoint.volume = new_trade.volume;
    }

    fn update_internal_candlesticks(&mut self, new_candle: &Candlestick) {
        self.last_datapoint = self.current_datapoint;
        self.current_datapoint.date = new_candle.date;
        self.current_datapoint.start_time = new_candle.start_time;
        self.current_datapoint.close_time = new_candle.close_time;
        self.current_datapoint.open = new_candle.open;
        self.current_datapoint.high = new_candle.high;
        self.current_datapoint.low = new_candle.low;
        self.current_datapoint.close = new_candle.close;
        self.current_datapoint.volume = new_candle.volume;
        self.current_datapoint.trades = new_candle.trades;
        self.current_datapoint.is_closed = new_candle.is_closed;
    }

    fn track_mark_to_market(&mut self, new_datapoint: &impl DataPoint) {
        if let Some(open_position) = self.current_position.as_mut() {
            let diff = new_datapoint.current_price().log10() - self.last_datapoint
                .current_price().log10();

            if diff < 0.0 {
                if open_position.side == Side::BUY {
                    // price decrease is a loss for long positions
                    open_position.sum_per_candle_log_lose_returns += -1.0 * diff;
                    open_position.sum_per_candle_total_log_returns += diff;
                } else {
                    // price decrease is a win for short positions
                    open_position.sum_per_candle_log_win_returns += -1.0 * diff;
                    open_position.sum_per_candle_total_log_returns += -1.0 * diff;
                }
            } else {
                if open_position.side == Side::BUY {
                    // price increase is a win for long positions
                    open_position.sum_per_candle_log_win_returns += diff;
                    open_position.sum_per_candle_total_log_returns += diff;
                } else {
                    // price increase is a loss for short positions
                    open_position.sum_per_candle_log_lose_returns += diff;
                    open_position.sum_per_candle_total_log_returns -= diff;
                }
            }
        }
    }

    fn should_updated_traded_qty(&mut self, datapoint: &impl DataPoint) {
        if self.fixed_traded_qty {
            return;
        }

        // we may adjust the traded qty in each new trading day
        // the amount traded is proportional to the initial balance
        // if the initial_qty_per_order is 5 contracts and given the initial balance of 10.000
        // if on the next trading day the current balance is 20.000 the traded qty will be 10 contracts
        // keeping the proportion as the balance changes
        if datapoint.date() != self.last_datapoint.date() {
            self.base_qty_per_order = (self.current_balance as u32 * self.initial_qty_per_order) / INITIAL_BALANCE as u32;

            // this is the minimum quantity to trade we will never trade less than this amount
            if self.base_qty_per_order < self.initial_qty_per_order {
                self.base_qty_per_order = self.initial_qty_per_order;
            }
        }
    }


    /// close an existing open existing position in the following cases:
    /// 1) the day is change and the position is open ( we don't allow overnight positions to be open )
    /// 2) the PNL of the position is negative and reached the minimum balance allowed
    /// a compulsory close fee of 25$ is applied if any such cases occur
    pub(crate) fn check_compulsory_close_position(&mut self, datapoint: &impl DataPoint) {
        if self.current_position.is_none() {
            return;
        }

        let position = self.current_position.as_mut().unwrap();

        // use high or low instead of close
        let gross_pnl = Self::calculate_gross_pnl(
            self.asset,
            position.side,
            position.avg_entry_price,
            datapoint.current_price(),
            position.open_qty as f64,
        );


        if datapoint.date() != self.last_datapoint.date() ||
            (gross_pnl < 0.0 && -1.0 * gross_pnl >= MINIMUM_BALANCE) {
            if DEBUG_BACKTEST_LOG {
                println!("[{}/{:?}] position with {} | entry: {:.2}, exit: {:.2} | qty: {} | result: {:.2}",
                         datapoint.date(), datapoint.time(), position.side, position.avg_entry_price, datapoint.current_price(), position.open_qty, gross_pnl);
            }
            let open_qty = position.open_qty;
            let last_datapoint = self.last_datapoint;
            position.total_paid_in_fees += COMPULSORY_CLOSE_FEE;
            self.current_balance -= COMPULSORY_CLOSE_FEE;
            self.decrease_position(last_datapoint.current_price(), &last_datapoint, open_qty as usize);
        }
    }

    fn check_scheduled_order_trigger(&mut self, _: &impl DataPoint) {
        // todo!()
    }

    fn execute_market_long_order(&mut self, quantity: usize) {
        self.execute_market_order(Side::BUY, quantity);
    }

    fn execute_market_short_order(&mut self, quantity: usize) {
        self.execute_market_order(Side::SELL, quantity);
    }

    fn execute_market_order(&mut self, direction: Side, quantity: usize) {
        let datapoint = self.current_datapoint;
        let entry_price = self.calculate_price_with_slippage(datapoint.current_price(), direction.clone());
        match &mut self.current_position {
            None => self.open_new_position(entry_price, direction, &datapoint, quantity),
            Some(position) => {
                if position.side == direction {
                    self.increase_position(entry_price, quantity);
                } else {
                    self.decrease_position(entry_price, &datapoint.to_owned(), quantity);
                }
            }
        }
    }

    fn zero_current_position(&mut self) {
        assert!(self.current_position.is_some(), "sanity check, should not try to close a position that doesnt exit");
    }

    /// slippage is a multiplier that makes the price bit worse when executing an order in the selected direction
    /// this aims to emulate the real behavior of the market, since between seen the price, executing the order and
    /// the conclusion in the exchange the actual execution price may ( and probably will be ) slight different
    fn calculate_price_with_slippage(&self, price: f64, _: Side) -> f64 {
        price
    }

    fn increase_position(&mut self, current_entry_price: f64, quantity_to_increase: usize) {
        assert!(self.current_position.is_some(), "sanity check, should not try to increase a position that doesnt exit");
        if !self.has_margin_execute_order() {
            self.removed_for_lack_of_margin = true;
            return;
        }

        let current_fee = &self.calculate_fees(quantity_to_increase as u64);
        let position = self.current_position.as_mut().unwrap();


        let last_entry = position.avg_entry_price;
        let new_total_open_qty = position.open_qty + quantity_to_increase as i32;

        position.avg_entry_price = ((position.avg_entry_price * position.open_qty as f64) + (current_entry_price * quantity_to_increase as f64)) / new_total_open_qty as f64;

        if DEBUG_BACKTEST_LOG {
            println!(
                "[{}/{:?}] position of {} contracts increased more {} contracts with price {current_entry_price}, entry updated from {last_entry} to {}",
                self.last_datapoint.date,
                self.last_datapoint.close_time.unwrap(),
                position.open_qty,
                quantity_to_increase,
                position.avg_entry_price
            );
        }


        assert!(position.avg_entry_price > 0.0);
        position.total_qty = position.total_qty + quantity_to_increase as i32;
        position.open_qty += quantity_to_increase as i32;

        position.quantity_of_contracts += self.base_qty_per_order;
        position.total_paid_in_fees += current_fee;
        self.total_contracts_executed_current_month += quantity_to_increase as u64;
        self.current_balance -= current_fee;

        assert!(position.total_qty >= position.open_qty, "open quantity cannot be greater than total quantity");
    }

    fn has_margin_execute_order(&self) -> bool {
        self.asset.margin_per_contract * (self.base_qty_per_order as f64) < self.current_balance
    }

    fn decrease_position(&mut self, current_exit_price: f64, datapoint: &impl DataPoint, quantity_to_decrease: usize) {
        if !self.has_margin_execute_order() {
            self.removed_for_lack_of_margin = true;
            return;
        }

        assert!(self.current_position.is_some(), "sanity check, should not try to decrease a position that doesnt exit");
        assert!(self.current_position.as_ref().unwrap().open_qty > 0, "cant decrease a position that doesnt have any contract/shares left open");

        let mut open_inverse_position_qty = 0;

        let quantity_to_decrease = if quantity_to_decrease as i32 > self.current_position.as_ref().unwrap().open_qty {
            open_inverse_position_qty = quantity_to_decrease - self.current_position.as_ref().unwrap().open_qty as usize;
            self.current_position.as_ref().unwrap().open_qty
        }else{
            quantity_to_decrease as i32
        };

        let current_fee = self.calculate_fees(quantity_to_decrease as u64);
        let executed_qty = self.current_position.as_ref().unwrap().total_qty - self.current_position.as_ref().unwrap().open_qty;
        self.current_position.as_mut().unwrap().avg_exit_price = ((self.current_position.as_ref().unwrap().avg_exit_price * executed_qty as f64) + (current_exit_price * quantity_to_decrease as f64)) / (executed_qty + quantity_to_decrease as i32) as f64;

        assert!(self.current_position.as_ref().unwrap().avg_exit_price > 0.0);
        self.current_position.as_mut().unwrap().open_qty -= quantity_to_decrease as i32;
        self.current_position.as_mut().unwrap().quantity_of_contracts += quantity_to_decrease as u32;
        self.current_position.as_mut().unwrap().total_paid_in_fees += current_fee;
        self.total_contracts_executed_current_month += quantity_to_decrease as u64;

        let pnl = Self::calculate_gross_pnl(
            self.asset,
            self.current_position.as_ref().unwrap().side,
            self.current_position.as_ref().unwrap().avg_entry_price,
            current_exit_price,
            quantity_to_decrease as f64,
        );

        if DEBUG_BACKTEST_LOG {
            println!("[{}/{:?}] position with {} | entry: {:.2}, exit: {:.2} | qty: {quantity_to_decrease} | result: {:.2}",
                     datapoint.date(), datapoint.time(), self.current_position.as_ref().unwrap().side, self.current_position.as_ref().unwrap().avg_entry_price, current_exit_price, pnl);
        }
        self.current_position.as_mut().unwrap().gross_pnl += pnl;
        self.current_balance += pnl;
        self.current_balance -= current_fee;

        assert!(self.current_position.as_ref().unwrap().total_qty >= self.current_position.as_ref().unwrap().open_qty, "open quantity quantity cannot be greater than total quantity");

        let side = self.current_position.as_ref().unwrap().side.opposite_side();
        if self.current_position.as_ref().unwrap().open_qty == 0 {
            self.close_position(datapoint);
        }

        if open_inverse_position_qty > 0 {
            self.open_new_position(
                datapoint.current_price(),
                side,
                datapoint,
                open_inverse_position_qty
            );
        }

    }


    fn track_current_month(&mut self) {
        let current_month_year = self.current_datapoint.date() / 100;
        if self.current_month_year == -1 {
            self.current_month_year = current_month_year;
        }

        if self.current_month_year != current_month_year {
            self.current_month_year = current_month_year;
            self.total_contracts_executed_current_month = 0;
        }
    }

    fn open_new_position(&mut self, entry_price: f64, side: Side, datapoint: &impl DataPoint, quantity: usize) {
        assert!(self.current_position.is_none(), "sanity check, should not try open a position without closing the existing one");
        let fee_open_position = self.calculate_fees(self.base_qty_per_order as u64);
        let new_position = Position {
            side,
            avg_entry_price: entry_price,
            avg_exit_price: 0.0,
            date: datapoint.date(),
            entry_time: datapoint.exit_time().unwrap(),
            exit_time: None,
            start_datapoint_number: self.total_candlesticks_processed as i32,
            end_datapoint_number: 0,
            total_qty: quantity as i32,
            open_qty: quantity as i32,
            gross_pnl: 0.0,
            liquid_pnl: 0.0,
            total_paid_in_fees: fee_open_position,
            quantity_of_contracts: quantity as u32,
            is_closed: false,
            sum_per_candle_total_log_returns: 0.0,
            sum_per_candle_log_win_returns: 0.0,
            sum_per_candle_log_lose_returns: 0.0,
        };
        self.current_position = Some(new_position);
        self.current_balance -= fee_open_position;
        self.total_contracts_executed_current_month += self.base_qty_per_order as u64;
    }

    fn close_position(&mut self, datapoint: &impl DataPoint) {
        assert!(self.current_position.is_some(), "sanity check, should not try to close a position that doesnt exit");
        let mut closed_position = self.current_position.as_ref().unwrap().clone();
        assert_eq!(0, closed_position.open_qty, "cant close a position that has open contracts/shares in the market");

        if closed_position.open_qty > 0 {
            let current_exit_price = datapoint.current_price();
            let quantity_to_decrease = closed_position.open_qty;
            let executed_qty = closed_position.total_qty - closed_position.open_qty;
            closed_position.avg_exit_price = ((closed_position.avg_exit_price * executed_qty as f64) + (current_exit_price * quantity_to_decrease as f64)) / (executed_qty + quantity_to_decrease) as f64;
            let pnl = Self::calculate_gross_pnl(
                self.asset,
                closed_position.side,
                closed_position.avg_entry_price,
                current_exit_price,
                quantity_to_decrease as f64,
            );
            self.current_balance += closed_position.gross_pnl;
            closed_position.gross_pnl += pnl;
        }

        closed_position.is_closed = true;
        closed_position.exit_time = datapoint.exit_time();
        closed_position.liquid_pnl = closed_position.gross_pnl - closed_position.total_paid_in_fees;
        closed_position.end_datapoint_number = self.total_candlesticks_processed as i32;
        closed_position.open_qty = 0;

        self.positions.push(closed_position);
        self.current_position = None;
    }

    fn calculate_gross_pnl(asset: &Asset, side: Side, avg_entry_price: f64, avg_exit_price: f64, quantity: f64) -> f64 {
        let price_diff = match side {
            Side::BUY => avg_exit_price - avg_entry_price,
            Side::SELL => avg_entry_price - avg_exit_price
        };
        let result = price_diff * quantity * asset.value_per_point;
        result
    }

    //TODO: we need a better way to define and calculate the fees for each orders send to the market
    // https://investimentos.btgpactual.com/custos
    fn calculate_fees(&self, total_orders: u64) -> f64 {
        let asset = self.asset;
        let brokerage = if asset.market == Market::FUTURES && asset.ticker == "WIN" || asset.ticker == "WDO" {
            match self.total_contracts_executed_current_month {
                0..=50 => 0.95,
                51..=100 => 0.90,
                101..=200 => 0.87,
                201..=500 => 0.76,
                501..=2500 => 0.63,
                2501..=5000 => 0.33,
                5001..=15000 => 0.12,
                15001.. => 0.05
            }
        } else if asset.market == Market::FUTURES && asset.ticker == "DOL" || asset.ticker == "IND" {
            match self.total_contracts_executed_current_month {
                0..=50 => 3.90,
                51..=100 => 3.69,
                101..=250 => 3.54,
                251..=500 => 2.55,
                501.. => 1.20
            }
        } else if asset.market == Market::FUTURES && asset.ticker == "BIT" {
            0.0
        } else {
            panic!("no brokerage rate defined for asset with ticker {}", asset.ticker);
        };

        let iss_percentage = 0.067;
        let iss_amount = iss_percentage * brokerage * total_orders as f64;
        let exchange_registration_fees = if asset.ticker == "WIN" {
            0.16 * total_orders as f64
        } else if asset.ticker == "WDO" {
            0.75 * total_orders as f64
        } else {
            // TODO: this is not right just temporary to allow testing
            0.75 * total_orders as f64
            //panic!("no exchange registration fee defined for asset with ticker {}", asset.ticker);
        };

        let exchange_emolument_fees = if asset.ticker == "WIN" {
            0.09 * total_orders as f64
        } else if asset.ticker == "WDO" {
            0.40 * total_orders as f64
        } else {
            // TODO: this is not right just temporary to allow testing
            0.40 * total_orders as f64
            //panic!("no exchange registration fee defined for asset with ticker {}", asset.ticker);
        };

        brokerage + iss_amount + exchange_registration_fees + exchange_emolument_fees
    }
}