use std::cmp;
use std::fmt::{Display, Formatter};
use crate::models::pnl::Position;
use crate::models::side::Side;
use crate::services::backtest::exchange_event_handler::{ExchangeEventHandler, INITIAL_BALANCE};
use crate::utils::math;

const TRADING_DAYS_IN_A_YEAR: f64 = 252.0;

#[allow(unused)]
pub struct BacktestMetrics {
    pub total_trading_days: u64,
    pub final_balance: f64,
    pub initial_balance: f64,
    pub total_return_percentage: f64,
    pub total_gross_profit: f64,
    pub total_liquid_profit: f64,
    pub avg_trade_liquid_pnl: f64,
    pub avg_daily_liquid_profit: f64,
    pub worse_day_liquid_profit: f64,
    pub avg_long_trade_liquid_pnl: f64,
    pub avg_long_short_liquid_pnl: f64,
    pub best_day_liquid_profit: f64,
    pub sharp_ratio: f64,
    pub median_day_liquid_profit: f64,
    pub median_minutes_in_the_market_per_day: u64,
    pub total_fees_paid: f64,
    pub minutes_in_the_market: u64,
    pub win_rate_percentage: f64,
    pub total_trades: u64,
    pub total_long_trades: u64,
    pub total_short_trades: u64,
    pub quantity_of_contracts: u64,
    pub total_wins: u64,
    pub total_loses: u64,
    pub avg_risk_reward: f64,
    pub max_win_streak: i32,
    pub max_lose_streak: i32,
    pub max_draw_down_percentage: f64,
    pub avg_winning_trade: f64,
    pub avg_losing_trade: f64,
    pub profit_factor: f64,
    pub profit_factor_log_sum: f64,
    pub aborted: bool,
    pub avg_contracts_traded: usize,
    pub balance_evolution: Vec<f64>,
    pub liquid_profit_best_trade: f64,
    pub liquid_profit_worst_trade: f64,
    pub trades: Vec<Position>,
}

impl Display for BacktestMetrics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        writeln!(f, "{0: <30} {1: <25.2?}", "Trading Days", self.total_trading_days)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Start Balance", INITIAL_BALANCE)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Final Balance", self.final_balance)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Absolute Return", self.final_balance - INITIAL_BALANCE)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Return(%)", self.total_return_percentage)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Expected Return Per Trade", self.avg_risk_reward)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Profit Factor", self.profit_factor)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Sharp Ratio", self.sharp_ratio)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Win Rate(%)", self.win_rate_percentage)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Max DrawDown(%)", self.max_draw_down_percentage)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Winning Trade PNL", self.avg_winning_trade)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Losing Trade PNL", self.avg_losing_trade)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Risk/Reward (Lower is Better)", -self.avg_losing_trade/self.avg_winning_trade)?;
        writeln!(f, "{0: <30} {1: <25}", "Sharp Ratio Description", evaluate_sharp_ratio(self.sharp_ratio))?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Fees Paid", self.total_fees_paid)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Wins", self.total_wins)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Max Win Streak", self.max_win_streak)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Loses", self.total_loses)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Max Lose Streak", self.max_lose_streak)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Trades", self.total_trades)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Long Trades", self.total_long_trades)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Short Trades", self.total_short_trades)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Total Contracts Executed", self.quantity_of_contracts)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Contracts per Trade", self.avg_contracts_traded)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Liquid PNL Best Day", self.best_day_liquid_profit)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Liquid PNL Worst Day", self.worse_day_liquid_profit)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "Avg Daily Liquid PNL", self.avg_daily_liquid_profit)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "PNL Best Trade", self.liquid_profit_best_trade)?;
        writeln!(f, "{0: <30} {1: <25.2?}", "PNL Worst Trade", self.liquid_profit_worst_trade)
    }
}

fn evaluate_sharp_ratio(sharp_ratio: f64) -> String {
    match sharp_ratio {
        (..1.0) => "Sub-optimal".to_string(),
        (1.0..2.0) => "Acceptable".to_string(),
        (2.0..3.0) => "Very Good".to_string(),
        (3.0..) => "Excellent".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn get_daily_percentage_returns(liquid_profit_per_day: Vec<f64>, daily_start_balances: Vec<f64>) -> Vec<f64> {
    liquid_profit_per_day.iter().zip(daily_start_balances.iter())
        .map(|(returns_current_day, start_balance_current_day)| 100.0 * (*returns_current_day / *start_balance_current_day))
        .collect()
}

pub(crate) fn calculate_backtest_results(event_handler: &ExchangeEventHandler) -> BacktestMetrics {
    let mut total_trades = 0;
    let mut total_contracts: u64 = 0;
    let mut total_fees_paid = 0.0;
    let mut total_liquid_pnl = 0.0;
    let mut total_gross_pnl = 0.0;
    let mut total_wins = 0;
    let mut total_loses = 0;

    let mut total_long_trades = 0;
    let mut total_short_trades = 0;
    let mut avg_long_trade_pnl = 0.0;
    let mut avg_short_trade_pnl = 0.0;

    let mut max_win_streak = 0;
    let mut max_lose_streak = 0;
    let mut current_win_lose_streak: i32 = 0;

    let mut max_draw_down_percentage: f64 = 0.0;
    let mut local_max_balance: f64 = INITIAL_BALANCE;
    let mut local_min_balance_after_local_max = INITIAL_BALANCE;
    let mut current_balance = INITIAL_BALANCE;

    let mut daily_start_balance: Vec<f64> = vec![INITIAL_BALANCE];

    let mut liquid_profit_best_trade = 0.0;
    let mut liquid_profit_worst_trade = 0.0;
    let mut liquid_profit_per_day = vec![0.0];
    let mut liquid_profit_best_day = 0.0;
    let mut liquid_profit_worst_day = 0.0;
    let mut current_day = if event_handler.positions.len() > 0 {
        event_handler.positions.first().unwrap().date
    } else {
        -1
    };
    let mut total_trading_days = 1;
    let mut profit_factor_log_sum = 0.0;
    let mut sum_winning_trades: f64 = 0.0;
    let mut sum_losing_trades: f64 = 0.0;
    let mut balance_evolution_over_time: Vec<f64> = vec![INITIAL_BALANCE];

    let mut positions =  vec![];

    for position in &event_handler.positions {
        positions.push(position.clone());
        profit_factor_log_sum += position.sum_per_candle_total_log_returns;

        if position.date != current_day {
            // update daily liquid worst/best daily liquid profits
            let index_liquid_profit = liquid_profit_per_day.len() - 1;
            let pnl_current_day = liquid_profit_per_day[index_liquid_profit];
            if pnl_current_day > liquid_profit_best_day {
                liquid_profit_best_day = pnl_current_day;
            }
            if pnl_current_day < liquid_profit_worst_day {
                liquid_profit_worst_day = pnl_current_day;
            }

            liquid_profit_per_day.push(0.0);
            total_trading_days += 1;
            daily_start_balance.push(current_balance);

        }

        let index_liquid_profit = liquid_profit_per_day.len() - 1;
        let liquid_rolling_profit_current_day = *liquid_profit_per_day.last().unwrap() + position.liquid_pnl;
        liquid_profit_per_day[index_liquid_profit] = liquid_rolling_profit_current_day;

        total_trades += 1;
        total_liquid_pnl += position.liquid_pnl;
        total_gross_pnl += position.gross_pnl;
        total_contracts += position.quantity_of_contracts as u64;
        total_fees_paid += position.total_paid_in_fees;
        current_day = position.date;

        if position.liquid_pnl > liquid_profit_best_trade {
            liquid_profit_best_trade = position.liquid_pnl;
        }

        if position.liquid_pnl < liquid_profit_worst_trade {
            liquid_profit_worst_trade = position.liquid_pnl;
        }

        if position.side == Side::BUY {
            total_long_trades+=1;
            avg_long_trade_pnl+=position.liquid_pnl;
        }else{
            total_short_trades+=1;
            avg_short_trade_pnl+=position.liquid_pnl;
        }

        if position.liquid_pnl > 0.0 {
            total_wins += 1;
            sum_winning_trades+= position.liquid_pnl;
            if current_win_lose_streak < 0 {
                max_lose_streak = cmp::max(max_lose_streak, -current_win_lose_streak);
                current_win_lose_streak = 1;
            } else {
                current_win_lose_streak += 1;
                max_win_streak = cmp::max(max_win_streak, current_win_lose_streak);
            }
        } else {
            total_loses += 1;
            sum_losing_trades+= position.liquid_pnl;
            if current_win_lose_streak > 0 {
                max_win_streak = cmp::max(max_win_streak, current_win_lose_streak);
                current_win_lose_streak = -1;
            } else {
                current_win_lose_streak -= 1;
                max_lose_streak = cmp::max(max_lose_streak, -current_win_lose_streak);
            }
        }

        current_balance += position.liquid_pnl;
        balance_evolution_over_time.push(current_balance);
        if current_balance > local_max_balance {
            local_max_balance = current_balance;
            local_min_balance_after_local_max = current_balance;
        }

        if current_balance < local_min_balance_after_local_max {
            local_min_balance_after_local_max = current_balance;
            let current_draw_down = (local_max_balance - local_min_balance_after_local_max) / local_max_balance;
            if max_draw_down_percentage < current_draw_down {
                max_draw_down_percentage = current_draw_down;
            }
        }
    }

    // check the pnl of the last day to see if it is the worst or best pnl
    let index_liquid_profit = liquid_profit_per_day.len() - 1;
    let pnl_current_day = liquid_profit_per_day[index_liquid_profit];
    if pnl_current_day > liquid_profit_best_day {
        liquid_profit_best_day = pnl_current_day;
    }

    if pnl_current_day < liquid_profit_worst_day {
        liquid_profit_worst_day = pnl_current_day;
    }

    let avg_contracts_traded = if total_trades == 0 {
        0
    }else{
        total_contracts / (2 * total_trades)
    };

    let avg_losing_trade = sum_losing_trades / total_loses as f64;
    let avg_winning_trade = sum_winning_trades / total_wins as f64;
    let win_rate = total_wins as f64 / total_trades as f64;
    let expected_return = (win_rate * avg_winning_trade) - ((1.0- win_rate) * -avg_losing_trade);

    // sharpe calculation
    let daily_percentage_returns = get_daily_percentage_returns(liquid_profit_per_day, daily_start_balance);
    let sum_daily_excess_return_percentage: f64 = daily_percentage_returns.iter()
        .map(|daily_percentage_return| *daily_percentage_return)
        .sum();
    let avg_daily_excess_return_percentage = sum_daily_excess_return_percentage / daily_percentage_returns.len() as f64;
    let standard_deviation_percentage_returns = math::standard_deviation(daily_percentage_returns);
    let sharp_ratio = (avg_daily_excess_return_percentage / standard_deviation_percentage_returns) * TRADING_DAYS_IN_A_YEAR.sqrt();

    BacktestMetrics {
        final_balance: event_handler.current_balance,
        initial_balance: INITIAL_BALANCE,
        total_return_percentage: ((event_handler.current_balance - INITIAL_BALANCE) / INITIAL_BALANCE) * 100.0,
        total_gross_profit: total_gross_pnl,
        total_liquid_profit: total_liquid_pnl,
        avg_trade_liquid_pnl: total_liquid_pnl / total_trades as f64,
        avg_daily_liquid_profit: total_liquid_pnl / total_trading_days as f64,
        worse_day_liquid_profit: liquid_profit_worst_day,
        avg_long_trade_liquid_pnl: avg_long_trade_pnl / (total_long_trades as f64 + 1e-30),
        avg_long_short_liquid_pnl: avg_short_trade_pnl / (total_short_trades as f64 + 1e-30),
        best_day_liquid_profit: liquid_profit_best_day,
        median_day_liquid_profit: 0.0,
        median_minutes_in_the_market_per_day: 0,
        total_fees_paid,
        minutes_in_the_market: 0,
        win_rate_percentage: 100.0 * win_rate,
        total_trades,
        total_long_trades,
        total_short_trades,
        quantity_of_contracts: total_contracts,
        total_wins,
        total_loses,
        avg_risk_reward: expected_return,
        max_win_streak,
        max_lose_streak,
        total_trading_days,
        max_draw_down_percentage: max_draw_down_percentage * 100.0,
        sharp_ratio,
        avg_winning_trade,
        avg_losing_trade,
        profit_factor: sum_winning_trades/(-1.0*sum_losing_trades),
        profit_factor_log_sum,
        aborted: event_handler.removed_for_lack_of_margin,
        balance_evolution: vec![],
        avg_contracts_traded: avg_contracts_traded as usize,
        liquid_profit_worst_trade,
        liquid_profit_best_trade,
        trades: positions,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_calculate_daily_return_percentages_correctly() {
        let start_of_day_balances = vec![10_000.0, 10_132.0, 9279.0];
        let result_each_day = vec![132.0, -853.0, 2714.0];

        let expected_percentage_returns: Vec<f64> = vec![1.32, -8.418870904066326, 29.248841469985994];
        let percentage_returns = get_daily_percentage_returns(result_each_day, start_of_day_balances);
        assert_eq!(expected_percentage_returns, percentage_returns);
    }

    // todo: commented function as reference for future test implementations
    // fn should_calculate_avg_entry_price_correctly_after_increase_position(){
    //
    // }
    //
    // fn should_calculate_avg_exit_price_correctly_after_decrease_position(){
    //
    // }
}
