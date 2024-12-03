use chrono::{NaiveTime, Timelike};
use ta::indicators::{BollingerBands, BollingerBandsOutput, SlowStochastic};
use ta::Next;
use crate::constants::CHROMOSOME_SIZE_TRADE_RULE;
use crate::indicators::relative_strength_index::RelativeStrengthIndex;
use crate::models::action::{market_order_from_side, OrderAction, TradeAction};
use crate::models::asset::Asset;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TimeCandlestickGenerator, TimeUnit};
use crate::models::candlestick::CandleDirection::{LONG, SHORT};
use crate::models::side::Side;
use crate::models::trade::Trade;
use crate::services::backtest::backtest::TradingBacktester;
use crate::services::optimization::brkga::FitnessExecutor;
use crate::services::trailing_stop::TrailingStop;
use crate::strategies::custom::trade_rule_strategy::optimization::BacktestOptimizerTradeRule;
use crate::strategies::custom::trade_rule_strategy::strategy::TradeRuleStrategy;
use crate::strategies::strategy::HighResolutionTradingStrategy;
use crate::utils::date;
use crate::utils::math::value_on_range;

struct ScalperOptimizer {
    pub(crate) training_backtester: TradingBacktester<'static>,
    pub(crate) validation_backtester: TradingBacktester<'static>,
}

pub fn scalper_optimizer_function(
    asset_to_trade: &'static Asset,
    training_start_date: u32,
    training_end_date: u32,
    validation_start_date: u32,
    validation_end_date: u32,
) -> Box<dyn FitnessExecutor> {
    let training_backtest_engine = TradingBacktester::new(
        asset_to_trade,
        training_start_date,
        training_end_date,
        true,
        None,
        false,
    ).unwrap();

    let validation_backtest_engine = TradingBacktester::new(
        asset_to_trade,
        validation_start_date,
        validation_end_date,
        true,
        None,
        false,
    ).unwrap();

    Box::new(
        ScalperOptimizer {
            training_backtester: training_backtest_engine,
            validation_backtester: validation_backtest_engine,
        }
    )
}

/////////// OPTIMIZATION
impl FitnessExecutor for ScalperOptimizer {
    fn calculate_fitness(&self, chromosome: &[f32]) -> f64 {
        let trading_model = Box::new(StormerScalperStrategy::decode(chromosome));
        let metrics = self.training_backtester.run_high_resolution_backtest(
            trading_model,
            QUANTITY_PER_POSITION as u32,
            false,
        );

        metrics.profit_factor
    }

    fn chromosome_details(&self, chromosome: &[f32]) -> String {
        let trading_model = Box::new(StormerScalperStrategy::decode(chromosome));
        let training_metrics = self.training_backtester.run_high_resolution_backtest(
            trading_model,
            QUANTITY_PER_POSITION as u32,
            true,
        );

        let validation_metrics = self.validation_backtester.run_high_resolution_backtest(
            Box::new(StormerScalperStrategy::decode(chromosome)),
            QUANTITY_PER_POSITION as u32,
            true,
        );

        format!(
            "\n\n####training\n{}\n\n####validation\n{}",
            training_metrics,
            validation_metrics
        )
    }

    fn is_chromosome_valid(&self, chromosome: &[f32]) -> bool {
        true
    }

    fn chromosome_size(&self) -> usize {
        16
    }
}


//////////////

const QUANTITY_PER_POSITION: usize = 8;

struct Configuration {
    candlestick_period: usize, // 10
    bollinger_bands_period: usize, // 21
    bollinger_bands_multiplier: f64, // 0.0 to 1.0
    slow_stochastic_period: usize,
    slow_stochastic_sma_period: usize,
    stochastic_long_threshold: f64, // 80.0
    stochastic_short_threshold: f64, // 20.0
    rsi_long_threshold: f64, // 70
    rsi_short_threshold: f64, // 30
    rsi_period: usize, //9

    // trailing stop loss configuration
    ticks_stop_loss: usize, // 35
    ticks_first_target: usize, // 15
    ticks_second_target: usize, // 20
    ticks_third_target: usize, // 28
    ticks_start_trailing: usize, // 5
    ticks_update_trailing_frequency: usize, // 1
    ticks_distance_from_price: usize // 2
}

pub struct StormerScalperStrategy {
    quantity_per_position: usize,
    points_per_tick_asset: Option<f64>,
    cut_off_time_sending_orders: NaiveTime,
    time_force_close_open_positions: NaiveTime,

    // indicators & context variables used for candlesticks with period of 15 seconds
    min_candles: usize,
    candle_count: usize,
    bollinger_bands: BollingerBands,
    stochastic: SlowStochastic,
    open_position: Option<ScalperPosition>,
    candle_parser: TimeCandlestickGenerator,
    last_three_closed_candles: [Candlestick; 3],
    rsi: RelativeStrengthIndex,
    configuration: Configuration
}

impl StormerScalperStrategy {
    pub fn decode(chromosome: &[f32]) -> Self {
        let configuration = Configuration {
            candlestick_period: 10,
            bollinger_bands_period: value_on_range((1.0, 100.0), chromosome[0] as f64) as usize,
            bollinger_bands_multiplier: value_on_range((1.0, 100.0), chromosome[1] as f64),
            slow_stochastic_period: value_on_range((1.0, 100.0), chromosome[2] as f64) as usize,
            slow_stochastic_sma_period: value_on_range((1.0, 100.0), chromosome[3] as f64) as usize,
            stochastic_long_threshold: value_on_range((1.0, 100.0), chromosome[4] as f64),
            stochastic_short_threshold: value_on_range((1.0, 100.0), chromosome[5] as f64),
            rsi_long_threshold: value_on_range((1.0, 100.0), chromosome[6] as f64),
            rsi_short_threshold: value_on_range((1.0, 100.0), chromosome[7] as f64),
            rsi_period: value_on_range((1.0, 100.0), chromosome[8] as f64) as usize,
            ticks_stop_loss: value_on_range((2.0, 60.0), chromosome[9] as f64) as usize,
            ticks_first_target: value_on_range((2.0, 60.0), chromosome[10] as f64) as usize,
            ticks_second_target: value_on_range((2.0, 60.0), chromosome[11] as f64) as usize,
            ticks_third_target: value_on_range((2.0, 60.0), chromosome[12] as f64) as usize,
            ticks_start_trailing: value_on_range((2.0, 60.0), chromosome[13] as f64) as usize,
            ticks_update_trailing_frequency: value_on_range((2.0, 60.0), chromosome[14] as f64) as usize,
            ticks_distance_from_price: value_on_range((2.0, 60.0), chromosome[15] as f64) as usize,
        };
        Self::new(configuration)
    }

    fn new(configuration: Configuration) -> Self
    where
        Self: Sized,
    {
        StormerScalperStrategy {
            quantity_per_position: QUANTITY_PER_POSITION,
            points_per_tick_asset: None,
            cut_off_time_sending_orders: NaiveTime::from_hms_opt(17, 50, 00).unwrap(),
            time_force_close_open_positions: NaiveTime::from_hms_opt(17, 55, 00).unwrap(),
            rsi: RelativeStrengthIndex::new(configuration.rsi_period),

            candle_count: 0,
            min_candles: 10,
            bollinger_bands: BollingerBands::new(configuration.bollinger_bands_period, configuration.bollinger_bands_multiplier).unwrap(),
            stochastic: SlowStochastic::new(configuration.slow_stochastic_period, configuration.slow_stochastic_sma_period).unwrap(),
            candle_parser: TimeCandlestickGenerator::new(TimeUnit::Seconds, configuration.candlestick_period as u64),
            last_three_closed_candles: [Candlestick::default(); 3],
            open_position: None,
            configuration,
        }
    }

    fn new_trailing_stop(&self, side: Side, entry_price: f64, points_per_tick: f64, quantity_to_trade: usize) -> ScalperPosition {
        assert!(quantity_to_trade % 2 == 0);
        assert!(quantity_to_trade % 4 == 0);

        ScalperPosition {
            side,
            open_quantity: quantity_to_trade,

            quantity_per_chunk: [quantity_to_trade / 2, quantity_to_trade / 4, quantity_to_trade / 4],
            first_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                self.configuration.ticks_stop_loss,
                self.configuration.ticks_first_target,
                1,
                1,
                1,
            ),
            second_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                self.configuration.ticks_stop_loss,
                self.configuration.ticks_second_target,
                self.configuration.ticks_start_trailing,
                self.configuration.ticks_distance_from_price,
                self.configuration.ticks_update_trailing_frequency,
            ),
            final_chunk: TrailingStop::new(
                entry_price,
                side,
                points_per_tick,
                self.configuration.ticks_stop_loss,
                self.configuration.ticks_third_target,
                self.configuration.ticks_start_trailing,
                self.configuration.ticks_distance_from_price,
                self.configuration.ticks_update_trailing_frequency,
            ),
        }
    }
}

impl HighResolutionTradingStrategy for StormerScalperStrategy {
    fn new() -> Self
    where
        Self: Sized
    {
        todo!()
    }


    fn name(&self) -> &str {
        "stormer_scalper_strategy"
    }

    fn days_of_context(&self) -> u32 {
        1
    }

    fn init_trading_day(&mut self, asset: &Asset, trades: &[Trade]) {
        self.points_per_tick_asset = Some(asset.min_point_variation);
        self.candle_count = 0;

        let cut_off_time_send_orders = date::decrease_time(
            &asset.end_trading_time,
            5,
            TimeUnit::Minutes,
        );
        let time_to_close_open_positions = date::increase_time(
            &cut_off_time_send_orders,
            2,
            TimeUnit::Minutes,
        );
        self.cut_off_time_sending_orders = cut_off_time_send_orders;
        self.time_force_close_open_positions = time_to_close_open_positions;

        for trade in trades {
            if let Some(fast_candle) = self.candle_parser.parse_only_closed_candles(trade) {
                self.rsi.next(fast_candle.close);
                self.stochastic.next(fast_candle.close);
                self.bollinger_bands.next(fast_candle.close);
                self.update_fast_candles(fast_candle);
            }
        }
    }

    fn process_new_trade(&mut self, new_trade: &Trade) -> TradeAction {
        let mut fast_order_enter: Option<OrderAction> = None;
        let fast_order_exit: Option<OrderAction> = self.should_exit_fast_position(new_trade);

        if let Some(fast_candle) = self.candle_parser.parse_only_closed_candles(new_trade) {
            self.candle_count += 1;
            let stochastic = self.stochastic.next(fast_candle.close);
            let bollinger_output = self.bollinger_bands.next(fast_candle.close);
            let rsi = self.rsi.next(fast_candle.close);
            self.update_fast_candles(fast_candle);
            fast_order_enter = self.should_open_position(stochastic, bollinger_output, rsi);
        }

        TradeAction::custom_order(fast_order_exit, fast_order_enter, None, None)
    }
}

impl StormerScalperStrategy {
    fn update_fast_candles(&mut self, new_closed_candle: Candlestick) {
        self.last_three_closed_candles[0] = self.last_three_closed_candles[1];
        self.last_three_closed_candles[1] = self.last_three_closed_candles[2];
        self.last_three_closed_candles[2] = new_closed_candle;
    }

    fn should_open_position(&mut self, stochastic: f64, bollinger_output: BollingerBandsOutput, rsi: f64) -> Option<OrderAction> {
        let first_candle = self.last_three_closed_candles[2];
        if self.candle_count < self.min_candles || self.open_position.is_some() ||
            first_candle.close_time.expect("candlestick should be closed")
                .num_seconds_from_midnight() >= self.cut_off_time_sending_orders.num_seconds_from_midnight() {
            return None;
        }

        let second_candle = self.last_three_closed_candles[1];
        let current_price = first_candle.close;

        // looking for long positions
        if current_price > bollinger_output.upper &&
            rsi < self.configuration.rsi_long_threshold &&
            stochastic >= self.configuration.stochastic_long_threshold &&
            second_candle.direction() == SHORT &&
            first_candle.direction() == LONG {
            self.open_position = Some(self.new_trailing_stop(
                Side::BUY,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketLong(self.quantity_per_position);
            return Some(order);
        }

        // looking for short positions
        if current_price < bollinger_output.lower &&
            rsi > self.configuration.rsi_short_threshold &&
            stochastic <= self.configuration.stochastic_short_threshold &&
            second_candle.direction() == LONG &&
            first_candle.direction() == SHORT {
            self.open_position = Some(self.new_trailing_stop(
                Side::SELL,
                current_price,
                self.points_per_tick_asset?,
                self.quantity_per_position,
            ));
            let order = OrderAction::MarketShort(self.quantity_per_position);
            return Some(order);
        }
        None
    }

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


        let mut quantity_to_close: usize = 0;

        let is_first_chunk_finished = self.open_position.as_ref()?.first_chunk.is_finished();
        let should_close_first_chunk = self.open_position.as_mut()?.first_chunk.should_close_position(new_trade.price);

        if !is_first_chunk_finished && should_close_first_chunk {
            quantity_to_close += self.open_position.as_mut()?.quantity_per_chunk[0];
        }

        let is_second_chunk_finished = self.open_position.as_ref()?.second_chunk.is_finished();
        let should_close_second_chunk = self.open_position.as_mut()?.second_chunk.should_close_position(new_trade.price);

        if !is_second_chunk_finished && should_close_second_chunk {
            quantity_to_close += self.open_position.as_ref()?.quantity_per_chunk[1];
        }

        let is_final_chunk_finished = self.open_position.as_ref()?.final_chunk.is_finished();
        let should_close_final_chunk = self.open_position.as_mut()?.final_chunk.should_close_position(new_trade.price);

        if !is_final_chunk_finished && should_close_final_chunk {
            quantity_to_close += self.open_position.as_ref()?.quantity_per_chunk[2];
        }

        if quantity_to_close > 0 {
            let side = self.open_position.as_ref()?.side;
            self.open_position.as_mut()?.open_quantity -= quantity_to_close;
            if self.open_position.as_ref()?.open_quantity == 0 {
                self.open_position = None;
            }

            let order = market_order_from_side(
                side.opposite_side(),
                quantity_to_close,
            );

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
    quantity_per_chunk: [usize; 3],
    first_chunk: TrailingStop,
    second_chunk: TrailingStop,
    final_chunk: TrailingStop,
}



