use cli_candlestick_chart::{Candle, Chart};
use crate::models::candlestick::Candlestick;

#[allow(unused)]
pub fn draw_candlestick_chart(candlesticks: &Vec<Candlestick>){
    let candles: Vec<Candle> = candlesticks.iter()
        .map(|candle| Candle::new(candle.open, candle.high, candle.low, candle.close, Some(candle.amount_traded), None))
        .collect();

    let mut chart = Chart::new(&candles);

    // Set the chart title
    chart.set_name(String::from("visualization"));

    // Set customs colors
    chart.set_bear_color(1, 205, 254);
    chart.set_bull_color(255, 107, 153);
    chart.set_vol_bull_color(1, 205, 254);
    chart.set_vol_bear_color(255, 107, 153);

    chart.set_volume_pane_height(6);
    chart.set_volume_pane_enabled(false);

    chart.draw();
}