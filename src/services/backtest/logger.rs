use chrono::{NaiveDate, NaiveTime};
use crate::models::side::Side;
use crate::services::backtest::logger::BacktestEvent::RemovedForLackOfMargin;

#[derive(Debug)]
#[allow(unused)]
pub enum BacktestEvent {
    BalanceUpdate,
    CompulsoryClosePosition,
    OpenPosition,
    ClosePosition,
    ReducePosition,
    IncreasePosition,
    RemovedForLackOfMargin,
}

const LOG_ACTIVE: bool = true;

pub fn log_balance_change(
    date: NaiveDate,
    time: NaiveTime,
    event: BacktestEvent,
    value: f64,
    diff: f64
){
    if LOG_ACTIVE {
        let date = format_date(date);
        let time = format_time(time);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"{:?}\",\"balance\": {value}, \"amount_changed\": {diff}}}", event);
    }

}

pub fn log_open_position(
    date: NaiveDate,
    time: NaiveTime,
    side: Side,
    entry_price: f64,
    contracts: usize,
    fee: f64,
){
    if LOG_ACTIVE {
        let direction = if side == Side::BUY {
            "LONG"
        } else {
            "SHORT"
        };
        let date = format_date(date);
        let time = format_time(time);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"OpenPosition\",\"direction\": \"{direction}\",\"entry_price\": {entry_price},\"contracts\": {contracts}, \"fee_paid\": {fee}}}");
    }
 }

pub fn log_close_position(
    date: NaiveDate,
    time: NaiveTime,
    entry_price: f64,
    exit_price: f64,
    contracts: usize
){
    if LOG_ACTIVE {
        let date = format_date(date);
        let time = format_time(time);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"ClosePosition\",\"entry_price\": {entry_price},\"exit_price\": {exit_price},\"contracts\": {contracts}}}");
    }
}

pub fn log_compulsory_close(
    date: NaiveDate,
    time: NaiveTime,
    last_date: NaiveDate,
){
    if LOG_ACTIVE {
        let date = format_date(date);
        let time = format_time(time);
        let last_date = format_date(last_date);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"CompulsoryClosePosition\", \"date_last_datapoint\": \"{last_date}\"}}");
    }
}

pub fn log_increase_position(
    date: NaiveDate,
    time: NaiveTime,
    entry_price: f64,
    contracts: usize,
    fee: f64,
){
    if LOG_ACTIVE {
        let date = format_date(date);
        let time = format_time(time);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"IncreasePosition\",\"increase_price\": {entry_price},\"contracts\": {contracts}, \"fee_paid\": {fee}}}");
    }
}

pub fn log_decrease_position(
    date: NaiveDate,
    time: NaiveTime,
    entry_price: f64,
    contracts: usize,
    pnl: f64,
    fee: f64,
){
    if LOG_ACTIVE {
        let date = format_date(date);
        let time = format_time(time);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"ReducePosition\",\"reduce_price\": {entry_price},\"contracts\": {contracts}, \"gross_pnl\": {pnl}, \"fee_paid\": {fee}}}");
    }
}

pub fn log_lack_of_margin(
    date: NaiveDate,
    time: NaiveTime,
    contracts: f64,
    current_balance: f64
){
    if LOG_ACTIVE {
        let date = format_date(date);
        let time = format_time(time);
        println!("{{\"date\": \"{date}\",\"time\": \"{time}\",\"event\": \"{:?}\",\"contracts\": {contracts}, \"balance\": {current_balance}}}", RemovedForLackOfMargin);
    }

}


fn format_time(time: NaiveTime) -> String {
    time.format("%H:%M:%S").to_string()
}

fn format_date(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}