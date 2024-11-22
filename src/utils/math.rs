

/// select a value inside the continuous range
/// continuous_range represent a continuous range of values where (start,end) where start < end
/// location_on_range should be a value between 0.0 and 1.0 which used to find a position of a value inside the range
pub fn value_on_range( continuous_range: (f64, f64), location_on_range: f64) -> f64 {
    assert!(location_on_range >= 0.0 && location_on_range <= 1.0, "location_on_range should be a value between 0.0 and 1.0");
    assert!(continuous_range.0 < continuous_range.1, "the first value should be lower than the second in the defined range");
    let lowest_value_on_range = continuous_range.0;
    let highest_value_on_range = continuous_range.1;
    lowest_value_on_range + location_on_range * (highest_value_on_range - lowest_value_on_range)
}

/// calculate the percentage difference between two numbers
pub fn percentage_difference(a: f64, b: f64) -> f64 {
    if a == b && b == 0.0 {
        return 0.0;
    }
    let min = a.min(b);
    let max = a.max(b);
    (max - min) / max
}

/// calculate the standard deviation for all values present in the vector
/// to know more: https://wikipedia.org/wiki/Standard_deviation
pub fn standard_deviation(values: Vec<f64>) -> f64 {
    let sum: f64 = values.iter().sum();
    let avg_value = sum / values.len() as f64;

    let sum_all_deviations_from_avg = values.iter()
        .fold(0.0, |sum_deviations, current_value| sum_deviations + (*current_value - avg_value).powf(2.0));

    f64::sqrt(sum_all_deviations_from_avg / (values.len() as f64))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_calculate_standard_deviation_correctly(){
        let values = vec![192.0, 183.0, 180.0, 179.0];

        let sd = standard_deviation(values);
        assert_eq!(5.123475382979799, sd);
    }

    #[test]
    fn should_calculate_percentage_difference_correctly() {
        let values_to_test = vec![
            (1.0, 2.0, 0.5),
            (2.0, 1.0, 0.5),
            (100.0, 50.0, 0.5),
            (2.0, 2.0, 0.0),
            (0.0, 200.0, 1.0),
            (0.0, 0.0, 0.0)
        ];

        for test in values_to_test {
            assert_eq!(percentage_difference(test.0, test.1), test.2);
        }
    }

    #[test]
    fn test_map_range() {
        let values_to_test = vec![
            ((-1000.0, 1000.0), 1.0, 1000.0),
            ((-1000.0, 1000.0), 0.0, -1000.0),
            ((-1000.0, 1000.0), 0.5, 0.0),
            ((0.0, 100.0), 1.0, 100.0),
            ((0.0, 100.0), 0.0, 0.0),
            ((0.0, 100.0), 0.5, 50.0),
            ((2.0, 200.0), 1.0, 200.0),
            ((2.0, 200.0), 0.0, 2.0),
            ((2.0, 200.0), 0.5, 101.0),
        ];

        for test in values_to_test {
            let range = test.0;
            let position_on_range = test.1;
            let expected_value = test.2;
            assert_eq!(expected_value, value_on_range(range, position_on_range));
        }
    }
}