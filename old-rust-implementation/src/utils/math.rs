#![allow(dead_code)]
#![allow(unused_variables)]

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

#[allow(unused)]
fn calculate_angle(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2- x1;
    let dy = y2 - y1;
    dy.atan2(dx)
}

#[cfg(test)]
mod test {
    use std::f64::consts::PI;
    use super::*;

    const EPSILON: f64 = 1e-12;

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

    #[test]
    fn test_horizontal_line() {
        // From (0.0, 0.0) to (10.0, 0.0), line is perfectly horizontal
        let angle = calculate_angle(0.0, 0.0, 10.0, 0.0);
        assert!(approx_eq(angle, 0.0));
    }

    #[test]
    fn test_vertical_line_up() {
        // From (0.0, 0.0) to (0.0, 10.0), line is perfectly vertical going up
        let angle = calculate_angle(0.0, 0.0, 0.0, 10.0);
        assert!(approx_eq(angle, PI / 2.0));
    }

    #[test]
    fn test_line_45_degrees() {
        // From (0.0, 0.0) to (10.0, 10.0), slope is 1, angle should be pi/4 (45 degrees).
        let angle = calculate_angle(0.0, 0.0, 10.0, 10.0);
        assert!(approx_eq(angle, PI / 4.0));
    }

    #[test]
    fn test_line_negative_45_degrees() {
        // From (0.0, 0.0) to (10.0, -10.0), slope is -1, angle should be -pi/4 (-45 degrees).
        let angle = calculate_angle(0.0, 0.0, 10.0, -10.0);
        assert!(approx_eq(angle, -PI / 4.0));
    }

    #[test]
    fn test_line_to_the_left() {
        // From (10.0, 0.0) to (0.0, 0.0), line goes left along the x-axis
        let angle = calculate_angle(10.0, 0.0, 0.0, 0.0);
        assert!(approx_eq(angle, PI));
    }

    #[test]
    fn angle_from_decreasing_points_on_real_trading() {
        let angle = calculate_angle(1.0, 123_555.0, 9.0, 123_420.0).to_degrees();
        assert!(approx_eq(angle, -86.608660580203662));
    }

    #[test]
    fn angle_from_increasing_points_on_real_trading() {
        let angle = calculate_angle(1.0, 123_335.0, 9.0, 123_445.0).to_degrees();
        assert!(approx_eq(angle, 85.840357706287364));
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }
}