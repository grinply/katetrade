use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::Next;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::candlestick::CandleDirection::{LONG, SHORT};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::strategies::strategy::HighResolutionTradingStrategy;

const BOLLINGER_PERIOD: usize = 21;
const BOLLINGER_MULTIPLIER: f64 = 0.38;
const SLOW_STOCHASTIC_PERIOD: usize = 8;
const SLOW_STOCHASTIC_SMA_PERIOD: usize = 3;
const STOCHASTIC_LONG_THRESHOLD: f64 = 80.0;
const STOCHASTIC_SHORT_THRESHOLD: f64 = 20.0;

pub struct StormerScalerStrategy {
    // indicators & context variables used for candlesticks with period of 15 seconds
    fast_candle_bollinger_bands: BollingerBands,
    fast_candle_stochastic: SlowStochastic,
    fast_open_position: Option<ScalperPosition>,
    fast_candle_parser: TimeCandlestickGenerator,
    last_three_fast_closed_candles: [Candlestick; 3],

    // indicators & context variables used for candlesticks with period of 6 minutes
    slow_candle_bollinger_bands: BollingerBands,
    slow_candle_stochastic: SlowStochastic,
    slow_open_position: Option<ScalperPosition>,
    slow_candle_parser: TimeCandlestickGenerator,
    last_three_slow_closed_candles: [Candlestick; 3],
}

impl HighResolutionTradingStrategy for StormerScalerStrategy {
    fn new() -> Self
    where
        Self: Sized,
    {
        StormerScalerStrategy {
            fast_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            fast_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
            fast_candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, 15),
            last_three_fast_closed_candles: [Candlestick::default(); 3],
            fast_open_position: None,

            slow_candle_bollinger_bands: BollingerBands::new(BOLLINGER_PERIOD, BOLLINGER_MULTIPLIER).unwrap(),
            slow_candle_stochastic: SlowStochastic::new(SLOW_STOCHASTIC_PERIOD, SLOW_STOCHASTIC_SMA_PERIOD).unwrap(),
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

    fn init_trading_day(&mut self, _: &Asset, _: &[Trade]) {}

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let mut slow_order_enter: Option<OrderAction> = None;

        let mut fast_order_exit: Option<OrderAction> = None;
        let mut slow_order_exit: Option<OrderAction> = None;

        if self.fast_open_position.is_some() {
            fast_order_exit = self.should_exit_fast_position();
        }

        if self.slow_open_position.is_some() {
            slow_order_exit = self.should_exit_slow_position();
        }

        if let Some(fast_candle) = self.fast_candle_parser.parse_only_closed_candles(new_trade) {
            let stochastic = self.fast_candle_stochastic.next(fast_candle.close);
            let bollinger_output = self.fast_candle_bollinger_bands.next(fast_candle.close);
            self.update_fast_candles(fast_candle);
            fast_order_enter = self.should_open_position_fast_scalp(stochastic, bollinger_output);
        }

        if let Some(slow_candle) = self.slow_candle_parser.parse_only_closed_candles(new_trade) {
            let stochastic = self.slow_candle_stochastic.next(slow_candle.close);
            let bollinger_output = self.slow_candle_bollinger_bands.next(slow_candle.close);
            self.update_slow_candles(slow_candle);
            slow_order_enter = self.should_open_position_slow_scalp(stochastic, bollinger_output);
        }

        TradeAction::custom_order(fast_order_exit, slow_order_exit, fast_order_enter, slow_order_enter)
    }
}

impl StormerScalerStrategy {
    fn update_fast_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_fast_closed_candles[0] = self.last_three_fast_closed_candles[1];
        self.last_three_fast_closed_candles[1] = self.last_three_fast_closed_candles[2];
        self.last_three_fast_closed_candles[2] = new_closed_candle;
    }

    fn update_slow_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_slow_closed_candles[0] = self.last_three_slow_closed_candles[1];
        self.last_three_slow_closed_candles[1] = self.last_three_slow_closed_candles[2];
        self.last_three_slow_closed_candles[2] = new_closed_candle;
    }

    fn should_open_position_fast_scalp(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        if self.fast_open_position.is_some() {
            return None;
        }

        let first_candle_direction = self.last_three_fast_closed_candles[2].direction();
        let second_candle_direction = self.last_three_fast_closed_candles[1].direction();
        let third_canle_direction = self.last_three_fast_closed_candles[0].direction();

        let current_price = self.last_three_fast_closed_candles[2].close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            first_candle_direction == SHORT &&
            second_candle_direction == LONG {
            return Some(OrderAction::MarketLong);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            first_candle_direction == LONG &&
            second_candle_direction == SHORT {
            return Some(OrderAction::MarketShort);
        }

        // looking for long positions
        if first_candle_direction == LONG &&
            second_candle_direction == LONG &&
            third_canle_direction == LONG {
            return Some(OrderAction::MarketLong);
        }

        // looking for short positions
        if first_candle_direction == SHORT &&
            second_candle_direction == SHORT &&
            third_canle_direction == SHORT {
            return Some(OrderAction::MarketShort);
        }

        None
    }

    fn should_open_position_slow_scalp(&self, stochastic: f64, bollinger_output: BollingerBandsOutput) -> Option<OrderAction> {
        if self.slow_open_position.is_some() {
            return None;
        }
        let first_candle_direction = self.last_three_slow_closed_candles[2].direction();
        let second_candle_direction = self.last_three_slow_closed_candles[1].direction();
        let third_direction = self.last_three_slow_closed_candles[0].direction();

        let current_price = self.last_three_slow_closed_candles[2].close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            stochastic >= STOCHASTIC_LONG_THRESHOLD &&
            first_candle_direction == SHORT &&
            second_candle_direction == LONG {
            return Some(OrderAction::MarketLong);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            stochastic <= STOCHASTIC_SHORT_THRESHOLD &&
            first_candle_direction == LONG &&
            second_candle_direction == SHORT {
            return Some(OrderAction::MarketShort);
        }

        // um dois tres de compra


        // um dois tres de venda

        None
    }

    fn should_exit_fast_position(&self) -> Option<OrderAction> {
        if self.fast_open_position.is_none() {
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

struct ScalperPosition {
    side: Side,
    entry_price: f64,
}

impl ScalperPosition {
    fn new(side: Side, entry_price: f64) -> Self {
        ScalperPosition { side, entry_price }
    }
}