use chrono::NaiveTime;
use crate::indicators::simple_moving_average::SimpleMovingAverage;
use crate::indicators::volume_weighted_average_price::VolumeWeightedAveragePrice;
use crate::models::action::TradeAction;
use crate::models::asset::Asset;
use crate::models::candlestick::{TimeCandlestickGenerator, TimeUnit};
use crate::models::exchange::Exchange::B3;
use crate::models::trade::Trade;
use crate::strategies::strategy::HighResolutionTradingStrategy;

const SMA_PERIOD: usize = 5;
const CANDLESTICK_PERIOD: u64 = 15;
const MAXIMUM_TICKS_STOP_LOSS: usize = 5;
const TICKS_FIRST_TARGET_SIGNAL: usize = 15;


pub fn get_points_per_tick_asset(asset_name: String) -> Option<f64> {
    if asset_name.eq("WIN") {
        return Some(5.0)
    }
    println!("INFO: default case, using WIN ticker points per asset");
    Some(5.0)
}

pub struct VWAPDayInitStrategy {
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,
    min_candles: usize,
    candles_elapsed_open_position: usize,
    candle_parser: TimeCandlestickGenerator,
    sma: SimpleMovingAverage,
    vwap: VolumeWeightedAveragePrice
}

impl VWAPDayInitStrategy {

}

impl HighResolutionTradingStrategy for VWAPDayInitStrategy {
    fn new() -> Self
    where
        Self: Sized
    {
        VWAPDayInitStrategy {
            points_per_tick_asset: get_points_per_tick_asset(Asset::get_asset("WIN", B3, )),
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(9, 30, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(10, 00, 00).unwrap(),
            min_candles: 10,
            candles_elapsed_open_position: 0,
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, CANDLESTICK_PERIOD),
            sma: SimpleMovingAverage::new(5).unwrap(),
            vwap: VolumeWeightedAveragePrice::new().unwrap(),
        }
    }

    fn name(&self) -> &str {
        "VWAP_init_trading_day_strategy"
    }

    fn init_trading_day(&mut self, asset: &Asset, context_trades: &[Trade]) {
        todo!()
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        todo!()
    }
}