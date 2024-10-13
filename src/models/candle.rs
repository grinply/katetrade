#[derive(Debug)]
pub struct Candle {
    pub lowest_price: f64,
    pub highest_price: f64,
    pub closed_price: f64,
    pub volume: f32,
    pub initial_date: i32,
    pub initial_time: i32,
    pub period: i64,
    pub closed: bool,
}
