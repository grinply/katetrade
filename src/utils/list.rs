
/// create an evenly spaced Vec of points between 0 and the target_number
pub fn create_vec_of_points(start_value:u32, amount_of_points: u32) -> Vec<(u32, u32)> {
    assert!(amount_of_points <= 1 || amount_of_points > start_value,
            "amount_of_points must be greater than 1 and less than or equal to number");

    let mut points: Vec<(u32,u32)> = Vec::new();
    let mut start = 0;
    let mut end = start_value / amount_of_points;
    while end < start_value {
        points.push((start, end));
        start = end;
        end += start_value / amount_of_points;
        if end > start_value {
            end = start_value;
        }
    }
    points
}