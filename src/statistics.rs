pub fn standard_deviation(values: Vec<f64>) -> f64 {
    let sum: f64 = values.iter().sum();
    let avg_value = sum / values.len() as f64;

    let sum_all_deviations_from_avg = values.iter()
        .fold(0.0, |sum_deviations, current_value| sum_deviations + (*current_value - avg_value).powf(2.0));

    f64::sqrt(sum_all_deviations_from_avg / (values.len() as f64 - 1.0))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_calculate_standard_deviation_correctly(){
        let values = vec![192.0, 183.0, 180.0, 179.0];

        let sd = standard_deviation(values);
        assert_eq!(5.916079783099616, sd);
    }

}