#![allow(unused)]
use crate::models::candlestick::Candlestick;

struct Model {
    entry_signals_indexes: Vec<usize>,
    points_per_tick: f64,
    features: Vec<InputFeature>,
    result_labels: Vec<i8>
}

struct TripleBarrierParams {
    amount_of_candlesticks: usize,
    take_profit_ticks: f64,
    stop_loss_ticks: f64,
}

/// All input data that will be used by the model for evaluation:
/// this includes: candlesticks data, indicators and position side, and any other information that might be relevant
pub struct InputFeature {
    values: Vec<f64>
}

fn train_model(
    params: TripleBarrierParams,
    features: Vec<InputFeature>,
    labels: Vec<i8>
){

}