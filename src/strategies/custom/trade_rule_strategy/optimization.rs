use crate::constants::CHROMOSOME_SIZE_TRADE_RULE;
use crate::models::asset::Asset;
use crate::models::candlestick::{TimeCandlestickGenerator, TimeUnit};
use crate::services::backtest::backtest::TradingBacktester;
use crate::services::backtest::metrics::BacktestMetrics;
use crate::services::optimization::brkga::FitnessExecutor;
use crate::strategies::custom::trade_rule_strategy::strategy::TradeRuleStrategy;

struct BacktestOptimizerTradeRule {
    training_backtester: TradingBacktester<'static>,
    validation_backtester: TradingBacktester<'static>,
}

pub fn fitness_optimizer_trade_rule_strategy(
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
        Some(Box::new(TimeCandlestickGenerator::new(TimeUnit::Minutes, 1))),
        false,
    ).unwrap();

    let validation_backtest_engine = TradingBacktester::new(
        asset_to_trade,
        validation_start_date,
        validation_end_date,
        true,
        Some(Box::new(TimeCandlestickGenerator::new(TimeUnit::Minutes, 1))),
        false,
    ).unwrap();

    Box::new(
        BacktestOptimizerTradeRule {
            training_backtester: training_backtest_engine,
            validation_backtester: validation_backtest_engine,
        }
    )
}

fn calculate_fitness(metrics: BacktestMetrics) -> f64{
    if metrics.max_draw_down_percentage.is_nan() ||
        metrics.max_draw_down_percentage.is_infinite() ||
        metrics.profit_factor.is_nan() ||
        metrics.profit_factor.is_infinite(){
        return -10_000.0;
    }

    (5.0 * metrics.profit_factor * metrics.total_return_percentage / 100.0) / (1.0 + 5.0 * metrics.max_draw_down_percentage/100.0)
}

impl FitnessExecutor for BacktestOptimizerTradeRule {
    fn calculate_fitness(&self, chromosome: &[f32]) -> f64 {
        let trading_model = Box::new(TradeRuleStrategy::decode(chromosome));
        let quantity_to_trade = trading_model.quantity_to_trade();
        let metrics = self.training_backtester.run_candlestick_backtest(
            trading_model,
            quantity_to_trade,
            false,
        );

        calculate_fitness(metrics)
    }

    fn chromosome_details(&self, chromosome: &[f32]) -> String {
        let trading_model = Box::new(TradeRuleStrategy::decode(chromosome));
        let quantity_to_trade = trading_model.quantity_to_trade();
        let training_metrics = self.training_backtester.run_candlestick_backtest(
            trading_model,
            quantity_to_trade,
            true,
        );

        let validation_metrics = self.validation_backtester.run_candlestick_backtest(
            Box::new(TradeRuleStrategy::decode(chromosome)),
            quantity_to_trade,
            true,
        );

        format!(
            "\n\n####training\n{}\n\n####validation\n{}",
            training_metrics,
            validation_metrics
        )
    }

    fn is_chromosome_valid(&self, chromosome: &[f32]) -> bool {
        let trading_model = TradeRuleStrategy::decode(chromosome);
        trading_model.is_valid()
    }

    fn chromosome_size(&self) -> usize {
        CHROMOSOME_SIZE_TRADE_RULE
    }
}