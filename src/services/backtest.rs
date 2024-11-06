use std::sync::Arc;
use std::thread;
use std::thread::{JoinHandle};
use chrono::Datelike;
use crate::models::candlestick::{Candlestick, CandlestickGenerator};
use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::{candlestick, trade};
use crate::services::aws::AwsClient;
use crate::services::configuration;
use crate::services::configuration::{home_dir, S3_BUCKET_HISTORICAL_DATA};
use crate::strategies::strategy::CandlestickConfiguration;
use crate::utils::date;

#[derive(Debug)]
#[allow(unused)]
pub struct TradingBacktester {
    #[allow(unused)]
    candlesticks: Vec<Candlestick>,
    pub qty_history_trading_days: u64,
    pub qty_history_candles: u64,
    pub qty_history_trades: u64,
    pub candlestick_configuration: CandlestickConfiguration,
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
        candlestick_configuration: CandlestickConfiguration,
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
        let mut candlestick_parser = candlestick::select_candlestick_generator(candlestick_configuration.clone());
        let aws_client = Arc::new(AwsClient::new("sa-east-1".to_string())?);

        let uppercase_ticker = ticker.to_ascii_uppercase();
        let mut threads_handle_download_history_files: Vec<JoinHandle<Result<(), anyhow::Error>>> = vec![];

        for year in start_date.year()..=end_date.year() {
            let begin_month = if year == start_date.year() {
                start_date.month()
            } else {
                1
            };

            let finish_month = if year == end_date.year() {
                end_date.month()
            } else {
                12
            };

            for month in begin_month..=finish_month {
                let begin_day = if year == start_date.year() && month == start_date.month() {
                    start_date.day()
                } else {
                    1
                };

                let finish_day = if year == end_date.year() && month == end_date.month() {
                    end_date.day()
                } else {
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
                    let binary_trades_file_name = format!("TRADE-{exchange}-{market}-{uppercase_ticker}-{year}{formatted_month}{formatted_day}");
                    let binary_trades_file_full_path = history_trades_folder_path.join(binary_trades_file_name.as_str());

                    //trades/IND/TRADE-B3-FUTURES-IND-20240722
                    let file_path_on_s3 = format!("trades/{}/{}", uppercase_ticker, binary_trades_file_name);

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
                                    qty_trades += 1;
                                    match candlestick_parser.parse_only_closed_candles(trade) {
                                        Some(candlestick) => {
                                            qty_candles += 1;
                                            candlesticks.push(candlestick)
                                        }
                                        None => (),
                                    }
                                });
                            }
                        }
                    } else if aws_client.file_exists_on_s3(S3_BUCKET_HISTORICAL_DATA, file_path_on_s3.as_str())? == true {
                        let client = aws_client.clone();
                        let file_path_on_s3 = file_path_on_s3.clone();
                        let output_folder_on_disk = history_trades_folder_path.clone();
                        let file_name = binary_trades_file_name.clone();

                        let thread_join_handle = thread::spawn(move || {
                            println!("downloading trades file {}", file_path_on_s3.as_str());
                            let file_output_path_on_disk = format!("{}/{}",
                                                                   output_folder_on_disk.to_str().unwrap(),
                                                                   file_name.as_str()
                            );
                            client.download_file_from_s3(S3_BUCKET_HISTORICAL_DATA,
                                                         file_path_on_s3.as_str(),
                                                         file_output_path_on_disk.as_str()
                            )
                        });
                        threads_handle_download_history_files.push(thread_join_handle);
                    }
                }
            }
        }

        for thread_handle in threads_handle_download_history_files {
            match thread_handle.join() {
                Ok(result) => {
                    if result.is_err() {
                        panic!("failed to download history file from aws s3. {}", result.err().unwrap());
                    }
                }
                Err(_) => {
                    panic!("thread failed to complete successfully.");
                }
            }
        }

        Ok(TradingBacktester {
            candlesticks,
            qty_history_trading_days: trading_days,
            qty_history_candles: qty_candles,
            qty_history_trades: qty_trades,
            candlestick_configuration,
        })
    }
}



