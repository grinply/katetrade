use std::cmp;
use std::collections::HashMap;
use itertools::Itertools;
use crate::models::pnl::Position;
use crate::utils::statistics::{self, normal_cdf};
use rand::distributions::{Distribution, Uniform};
use rand::{thread_rng, Rng};

const TRADING_DAYS_IN_YEAR: i32 = 252;



/// Calculates the **Bias Corrected Accelerated Bootstrap** based on the daily results by grouping the trading_results for each day.
/// Produces the **annualized percentage** confidence for the provided **low_conf** and **high_conf** values inputs.
// if you want to grasp the concept of this algorithm see the following video: https://www.youtube.com/watch?v=655X9eZGxls
#[allow(unused)]
pub fn bias_corrected_accelerated_bootstrap(
    low_conf: f64,
    high_conf: f64,
    bootstrap_iterations: usize,
    trading_results: Vec<Position>,
) -> (f64, f64) {
    let mut rng = thread_rng();
    let daily_pnls = calculate_daily_pnl(&trading_results);

    let pnl_sum: f64 = daily_pnls.iter().sum();
    let mean_pnl_real_value: f64 = pnl_sum / (daily_pnls.len() as f64);

    let mut z0_count = 0;
    let mut iteration = 0;

    let mut bootstrap_sample: Vec<f64> = Vec::with_capacity(daily_pnls.len());
    let mut result: Vec<f64> = Vec::with_capacity(daily_pnls.len());

    while iteration < bootstrap_iterations {
        let mean_pnl_bootstrap_sample =
            inplace_new_random_bootstrap_sample(&mut rng, &daily_pnls, &mut bootstrap_sample);
        result[iteration] = mean_pnl_bootstrap_sample;
        if mean_pnl_bootstrap_sample < mean_pnl_real_value {
            z0_count += 1;
        }
        iteration += 1;
    }

    let mut theta_dot = 0.0;
    let mut jack_knife = bootstrap_sample;

    for (i, current_day_pnl) in daily_pnls.iter().enumerate() {
        let avg_pnl_without_one_day = (pnl_sum - current_day_pnl) / (daily_pnls.len() - 1) as f64;
        theta_dot += avg_pnl_without_one_day;
        jack_knife[i] = avg_pnl_without_one_day;
    }
    theta_dot = theta_dot / daily_pnls.len() as f64;

    let (mut denom, mut numer) = (0.0, 0.0);
    for jack_knife_sample in jack_knife.iter() {
        let diff = theta_dot - jack_knife_sample;
        let xtemp = diff * diff;
        denom += xtemp;
        numer += xtemp * diff;
    }

    denom = f64::sqrt(denom);
    denom = denom * denom * denom;
    let accel = numer / (6.0 * denom + (1e-60));

    result.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let zlo = statistics::inverse_normal_cdf(low_conf);
    let zhi = statistics::inverse_normal_cdf(high_conf);
    let z0 = statistics::inverse_normal_cdf((z0_count / bootstrap_iterations) as f64);
    let alo = normal_cdf(z0 + (z0 + zlo) / (1.0 - accel * (z0 + zlo)));
    let ahi = normal_cdf(z0 + (z0 + zhi) / (1.0 - accel * (z0 + zhi)));
    println!("{}", ahi);

    let k = cmp::max(0, ((alo as usize) * (bootstrap_iterations + 1)) - 1); // Unbiased fractile estimator

    (
        result[k] * (TRADING_DAYS_IN_YEAR as f64) * 100.0,
        result[bootstrap_iterations - 1 - k] * (TRADING_DAYS_IN_YEAR as f64) * 100.0,
    )
}


#[allow(unused)]
// returns the mean value from the generated sample
fn inplace_new_random_bootstrap_sample<R: Rng>(
    rng: &mut R,
    daily_pnls: &Vec<f64>,
    bootstramp_sample_to_update: &mut Vec<f64>,
) -> f64 {
    let mut pnl_sum = 0.0;
    let uniform = Uniform::new(0, daily_pnls.len());

    for bootstrap_value in bootstramp_sample_to_update.iter_mut() {
        let k = uniform.sample(rng);
        *bootstrap_value = daily_pnls[k];
        pnl_sum += daily_pnls[k];
    }
    return pnl_sum / (bootstramp_sample_to_update.len() as f64);
}

/// Sum all PNL values from the same day and produce a Vec containing all the daily PNLs
#[allow(unused)]
fn calculate_daily_pnl(trading_results: &Vec<Position>) -> Vec<f64> {
    let grouped_pnls_by_day = trading_results
        .into_iter()
        .fold(HashMap::new(), |mut acc, pnl| {
            *acc.entry(pnl.date).or_insert(0.0) += pnl.gross_pnl;
            acc
        });
    let mut all_daily_pnls = Vec::with_capacity(grouped_pnls_by_day.len());

    //sort this by date
    for date in grouped_pnls_by_day.keys().sorted() {
        all_daily_pnls.push(grouped_pnls_by_day[date]);
    }
    all_daily_pnls
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_inplace_new_random_bootstrap_sample_deterministic() {
        // Sample data
        let daily_pnls = vec![11.0, 20.0, 30.0, 40.0];
        let mut bootstrap_sample = vec![0.0, 0.0, 0.0, 0.0];

        // Create a seeded RNG for deterministic behavior
        let seed = [0u8; 32]; // 256-bit seed (all zeros)
        let mut rng = StdRng::from_seed(seed);

        let mean =
            inplace_new_random_bootstrap_sample(&mut rng, &daily_pnls, &mut bootstrap_sample);

        let expected_bootstrap_sample = vec![20.0, 40.0, 11.0, 20.0];
        let sum_values: f64 = expected_bootstrap_sample.iter().sum();
        let expected_mean = sum_values / 4.0; // 20.0

        assert_eq!(bootstrap_sample, expected_bootstrap_sample);
        assert_eq!(mean, expected_mean);
    }

    #[test]
    fn should_compute_daily_pnl_grouping_results_from_same_day() {
        let mut trading_results: Vec<Position> = Vec::new();

        trading_results.push(Position {
            avg_entry_price: 1.0,
            avg_exit_price: 1.0,
            date: 17092024,
            entry_time: 132205,
            exit_time: 132205,
            start_datapoint_number: 1,
            end_datapoint_number: 2,
            qty: 2,
            gross_pnl: 21.4,
        });

        trading_results.push(Position {
            avg_entry_price: 1.0,
            avg_exit_price: 1.0,
            date: 17092024,
            entry_time: 132205,
            exit_time: 132205,
            start_datapoint_number: 1,
            end_datapoint_number: 2,
            qty: 2,
            gross_pnl: 10.5,
        });

        trading_results.push(Position {
            avg_entry_price: 1.0,
            avg_exit_price: 1.0,
            date: 18092024,
            entry_time: 132205,
            exit_time: 132205,
            start_datapoint_number: 1,
            end_datapoint_number: 2,
            qty: 2,
            gross_pnl: -40.5,
        });

        trading_results.push(Position {
            avg_entry_price: 1.0,
            avg_exit_price: 1.0,
            date: 19092024,
            entry_time: 132205,
            exit_time: 132205,
            start_datapoint_number: 1,
            end_datapoint_number: 2,
            qty: 2,
            gross_pnl: 22.5,
        });

        let daily_pnls = calculate_daily_pnl(&trading_results);
        assert_eq!(3, daily_pnls.len());
        assert_eq!(
            trading_results[0].gross_pnl + trading_results[1].gross_pnl,
            daily_pnls[0]
        );
        assert_eq!(trading_results[2].gross_pnl, daily_pnls[1]);
        assert_eq!(trading_results[3].gross_pnl, daily_pnls[2]);
    }
}
