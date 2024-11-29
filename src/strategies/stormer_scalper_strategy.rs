use itertools::Position;
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::Next;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::trade::Trade;
use crate::strategies::strategy::HighResolutionTradingStrategy;

const BOLLINGER_PERIOD: usize = 21;
const BOLLINGER_MULTIPLIER: f64 = 0.38;
const SLOW_STOCHASTIC_PERIOD: usize = 8;
const SLOW_STOCHASTIC_EMA_PERIOD: usize = 3;
const STOCHASTIC_BUY_THRESHOLD: usize = 4;

struct StormerScalerStrategy {
    // indicators & context variables used for candlesticks with period of 15 seconds
    fast_candle_bollinger_bands: BollingerBands,
    fast_candle_stochastic: SlowStochastic,
    fast_open_position: Option<Position>,
    fast_candle_parser: TimeCandlestickGenerator,
    last_three_fast_closed_candles:[Candlestick; 3],

    // indicators & context variables used for candlesticks with period of 6 minutes
    slow_candle_bollinger_bands: BollingerBands,
    slow_candle_stochastic: SlowStochastic,
    slow_open_position: Option<Position>,
    slow_candle_parser: TimeCandlestickGenerator,
    last_three_slow_closed_candles:[Candlestick; 3],
}

impl HighResolutionTradingStrategy for StormerScalerStrategy {
    fn new() -> Self
    where
        Self: Sized,
    {
        StormerScalerStrategy {
            fast_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            fast_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_EMA_PERIOD).unwrap(),
            fast_candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, 15),
            last_three_fast_closed_candles: [Candlestick::default(); 3],
            fast_open_position: None,

            slow_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            slow_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_EMA_PERIOD).unwrap(),
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

    fn init_trading_day(&mut self, _: &Asset, _: &[Trade]) {

    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let mut slow_order_enter: Option<OrderAction> = None;

        let mut fast_order_exit: Option<OrderAction> = None;
        let mut slow_order_exit: Option<OrderAction> = None;

        if let Some(fast_candle) = self.fast_candle_parser.parse_only_closed_candles(new_trade) {
            let stochastic = self.fast_candle_stochastic.next(fast_candle.close);
            let bollinger_output = self.fast_candle_bollinger_bands.next(fast_candle.close);
            fast_order_enter = self.should_open_position_fast_scalp(stochastic, bollinger_output);
        }

        if let Some(slow_candle) = self.slow_candle_parser.parse_only_closed_candles(new_trade) {
            let stochastic = self.slow_candle_stochastic.next(slow_candle.close);
            let bollinger_output = self.slow_candle_bollinger_bands.next(slow_candle.close);
            slow_order_enter = self.should_open_position_slow_scalp(stochastic, bollinger_output);
        }

        if self.fast_open_position.is_some() {
            fast_order_exit = self.should_exit_fast_position();
        }

        if self.slow_open_position.is_some() {
            slow_order_exit = self.should_exit_slow_position();
        }

        TradeAction::custom_order(fast_order_exit, slow_order_exit, fast_order_enter, slow_order_enter)
    }
}

impl StormerScalerStrategy {
    fn should_open_position_fast_scalp(&self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        if self.fast_open_position.is_some() {
            return None;
        }
        None
    }

    fn should_open_position_slow_scalp(&self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        if self.slow_open_position.is_some() {
            return None;
        }
        None
    }

    fn should_exit_fast_position(&self) -> Option<OrderAction>{
        if self.fast_open_position.is_none(){
            return None;
        }
        None
    }

    fn should_exit_slow_position(&self) -> Option<OrderAction> {
        if self.slow_open_position.is_none() {
            return None;
        }
        None
    }
}