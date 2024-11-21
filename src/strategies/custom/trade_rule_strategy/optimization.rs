use crate::constants::CHROMOSOME_SIZE_TRADE_RULE;
use crate::models::asset::Asset;
use crate::services::backtest::{BacktestMetrics, TradingBacktester};
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
        TradeRuleStrategy::candlestick_configuration(),
        false,
    ).unwrap();

    let validation_backtest_engine = TradingBacktester::new(
        asset_to_trade,
        validation_start_date,
        validation_end_date,
        TradeRuleStrategy::candlestick_configuration(),
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
        metrics.avg_winning_trade.is_infinite() ||
        metrics.avg_winning_trade.is_nan() ||
        metrics.avg_losing_trade.is_infinite() ||
        metrics.avg_losing_trade.is_nan() ||
        metrics.profit_factor.is_infinite() ||
        metrics.profit_factor.is_nan() {
        return -10_000.0;
    }

    let mut base_result = metrics.total_return_percentage / 100.0;

    if metrics.total_trades < 533 {
        base_result /= (533.0 - metrics.total_trades as f64) * 19.0;
    }

    if metrics.win_rate_percentage < 60.0 {
        base_result /= (60.0 - metrics.win_rate_percentage) * 250.0;
    }

    if metrics.max_draw_down_percentage > 35.0 {
        base_result /= (metrics.max_draw_down_percentage - 35.0) * 155.0;
    }

    if metrics.profit_factor < 1.2 {
        base_result /= 10_000.0;
    }

    if metrics.avg_winning_trade < metrics.avg_losing_trade {
        base_result /= 10_000.0;
    }

    base_result
}

impl FitnessExecutor for BacktestOptimizerTradeRule {
    fn calculate_fitness(&self, chromosome: &[f32]) -> f64 {
        let trading_model = TradeRuleStrategy::decode(chromosome);
        let quantity_to_trade = trading_model.quantity_to_trade();
        let metrics = self.training_backtester.run_backtest(
            trading_model,
            quantity_to_trade,
            false,
        );

        calculate_fitness(metrics)
    }

    fn chromosome_details(&self, chromosome: &[f32]) -> String {
        let trading_model = TradeRuleStrategy::decode(chromosome);
        let quantity_to_trade = trading_model.quantity_to_trade();
        let training_metrics = self.training_backtester.run_backtest(
            trading_model,
            quantity_to_trade,
            false,
        );

        let validation_metrics = self.validation_backtester.run_backtest(
            TradeRuleStrategy::decode(chromosome),
            quantity_to_trade,
            false,
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