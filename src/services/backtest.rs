use chrono::Datelike;
use crate::models::candlestick::{Candlestick, CandlestickGenerator, TickCandlestickGenerator};
use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::trade;
use crate::services::configuration;
use crate::services::configuration::home_dir;
use crate::utils::date;

#[derive(Debug)]
pub struct TradingBacktester {
    #[allow(unused)]
    candlesticks: Vec<Candlestick>,
    pub qty_history_trading_days: u64,
    pub qty_history_candles: u64,
    pub qty_history_trades: u64,
}

impl TradingBacktester {
    /// Initializes the TradingBacktester
    /// The historical data will be loaded from the local disk or from AWS S3 if not available locally
    pub fn new(
        exchange: Exchange,
        market: Market,
        ticker: &str,
        start_date: u32,
        end_date: u32,
        ticks_per_candle: u64,
    ) -> Result<TradingBacktester, anyhow::Error> {
        let start_date = date::convert_to_naive_date(start_date)?;
        let end_date = date::convert_to_naive_date(end_date)?;

        if end_date < start_date {
            anyhow::bail!("invalid dates, the end_date is before the start_date");
        }

        let history_trades_folder_path = home_dir()
            .join(configuration::CONFIGURATION_FOLDER)
            .join("history_trades")
            .join(ticker);

        if !history_trades_folder_path.exists() {
            std::fs::create_dir_all(&history_trades_folder_path)?;
        }

        let mut trading_days = 0;
        let mut qty_candles = 0;
        let mut qty_trades = 0;
        let mut candlesticks: Vec<Candlestick> = vec![];
        let mut tick_candlestick_parser = TickCandlestickGenerator::new(ticks_per_candle);

        for year in start_date.year()..=end_date.year() {
            let begin_month = if year == start_date.year() {
                start_date.month()
            }else{
                1
            };

            let finish_month = if year == end_date.year() {
                end_date.month()
            }else{
                12
            };

            for month in begin_month..=finish_month {
                let begin_day = if year == start_date.year() && month == start_date.month() {
                    start_date.day()
                }else{
                    1
                };

                let finish_day = if year == end_date.year() && month == end_date.month() {
                    end_date.day()
                }else{
                    31
                };

                for day in begin_day..=finish_day {
                    let formatted_day = if day < 10 {
                        format!("0{}", day)
                    } else {
                        format!("{}", day)
                    };

                    let formatted_month = if month < 10 {
                        format!("0{}", month)
                    } else {
                        format!("{}", month)
                    };

                    //TRADE-B3-FUTURES-IND-20240722
                    let binary_trades_file_name = format!("TRADE-{exchange}-{market}-{ticker}-{year}{formatted_month}{formatted_day}");
                    let binary_trades_file_full_path = history_trades_folder_path.join(binary_trades_file_name.as_str());

                    if binary_trades_file_full_path.exists() {
                        trading_days += 1;
                        match trade::read_trades_from_binary_file(binary_trades_file_full_path.to_str().unwrap()) {
                            Err(e) => {
                                anyhow::bail!(
                                    "failed to read trades from local file {}. err: {}",
                                    binary_trades_file_full_path.to_str().unwrap(), e
                                )
                            }
                            Ok(trades) => {
                                trades.iter().for_each(|trade| {
                                    qty_trades+=1;
                                    match tick_candlestick_parser.parse_only_closed_candles(trade) {
                                        Some(candlestick) => {
                                            qty_candles+=1;
                                            candlesticks.push(candlestick)
                                        },
                                        None => (),
                                    }
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(TradingBacktester {
            candlesticks,
            qty_history_trading_days: trading_days,
            qty_history_candles: qty_candles,
            qty_history_trades: qty_trades
        })
    }
}



