mod constants;
mod evaluation;
mod models;
mod services;
mod strategies;
mod utils;

use std::io::{stdin};
use crate::models::order_type::OrderType;
use crate::services::market_connector::MarketConnectorClient;
use clap::{arg, ArgMatches, Command};
use models::exchange::Exchange;
use std::str::FromStr;
use std::string::ToString;
use std::time::Instant;
use crate::models::asset::Asset;
use crate::models::candlestick::CandlestickType;
use crate::models::market::Market;
use crate::models::side::Side;
use crate::services::backtest;
use crate::services::configuration::{load_configuration, KateConfiguration};
use crate::strategies::execution_manager::{LiveStrategyExecutor, ExecutionRequest};
use crate::strategies::strategy::{get_candlestick_strategy_by_name, CandlestickConfiguration, StrategyInputData};

const VERSION: &str = env!("CARGO_PKG_VERSION");
fn cli() -> Command {
    Command::new("kate")
        .about(
            "Welcome to katetrade, your algorithm trading system tailored for the Brazilian capital market",
        )
        .version(VERSION)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(constants::RUN)
                .about("Start trading one or more trading strategies in the real capital markets")
                .arg(arg!(-s --strategy <STRATEGY> "Identifier (id) for the strategy you want to executed'").required(true))
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'").required(true))
                .arg(arg!(-a --account <ACCOUNT> "AccountId used when sending trades in live mode to the market-connector'").required(true))
                .arg(arg!(-d --data <DATA> "Type of data used as input for the trading strategy")
                    .value_parser([constants::CANDLESTICK, constants::TRADE])
                    .num_args(0..=1)
                    .default_value(constants::CANDLESTICK)
                )
                .arg(
                    arg!(--market <MARKET> "Market where to send the order")
                        .value_parser([constants::FUTURES, constants::SPOT])
                        .num_args(0..=1)
                        .required(true)
                )
            ,
        )
        .subcommand(
            Command::new(constants::ORDER)
                .about("Operations related to orders")
                .subcommand(Command::new(constants::SEND)
                    .about("Send a order to be execute in the exchange/broker")
                    .arg(
                        arg!(--side <SIDE> "Side that of the order")
                            .value_parser([constants::BUY, constants::SELL])
                            .num_args(0..=1)
                            .default_value(constants::BUY)
                            .default_missing_value(constants::BUY)
                    )
                    .arg(
                        arg!(--type <TYPE> "Type of order that will be used")
                            .value_parser([constants::MARKET, constants::LIMIT])
                            .num_args(0..=1)
                            .default_value(constants::MARKET)
                            .default_missing_value(constants::MARKET)
                    )
                    .arg(
                        arg!(--market <MARKET> "Market where to send the order")
                            .value_parser([constants::FUTURES, constants::SPOT])
                            .num_args(0..=1)
                            .required(true)
                    )
                    .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'").required(true))
                    .arg(arg!(-a --account <ACCOUNT> "AccountId to be used when sending a order to the exchange/broker").required(true))
                    .arg(arg!(-q --qty <QUANTITY> "Amount of shares/contracts that will be requested with the order").required(true))
                    .arg(arg!(--price <PRICE> "Price when sending limit orders to the order book"))
                )
                .subcommand(
                    Command::new(constants::DETAILS)
                        .about("View the details of a order given its id")
                        .arg(arg!(<ORDER_ID> "id of a order previously sent for execution").required(true))
                )
            ,
        )
        .subcommand(
            Command::new(constants::BACKTEST)
                .about("Backtest a trading strategy using historical data")
                .arg(arg!(-s --strategy <STRATEGY> "Identifier (id) for the strategy you want to executed'").required(true))
                .arg(
                    arg!(--market <MARKET> "Market where to send the order")
                        .value_parser([constants::FUTURES, constants::SPOT])
                        .num_args(0..=1)
                        .required(true)
                )
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'").required(true))
                .arg(arg!(--begin <BEGIN_DATE> "Start trading day to start the execution, in the format: yyyyMMdd"))
                .arg(arg!(--end <END_DATE> "End trading day to finish the execution, in the format: yyyyMMdd"))
                .arg(arg!(--exchange <EXCHANGE> "Exchange where the ticker belongs to")
                    .value_parser([constants::B3])
                    .default_value(constants::B3)
                    .num_args(0..=1))
                .arg(arg!(--candle_type <CANDLE_TYPE> "Type of candlestick that will be used in the backtest'")
                    .value_parser([constants::TIME, constants::TICK, constants::VOLUME])
                    .default_value(constants::TICK)
                    .num_args(0..=1))
                .arg(arg!(--candle_configuration <END_DATE> "Input configuration for the type of candlestick selected (time range in minutes, ticks, total volume)")
                    .default_value("50"))
        )
        .subcommand(
            Command::new(constants::CONNECTION)
                .about("General connection information")
                .subcommand(
                    Command::new(constants::STATUS)
                        .about("evaluate the network connection with market connector (service responsible for processing orders and providing live market data")
                )
        )
}

fn process_backtest_command(sub_matches: &ArgMatches, _: KateConfiguration) {
    let begin: u32 = sub_matches.get_one::<String>("begin").expect("required")
        .parse().expect("begin should a positive number in the format yyyyMMdd");
    let end: u32 = sub_matches.get_one::<String>("end").expect("required")
        .parse().expect("end should a positive number in the format yyyyMMdd");

    let candle_type = CandlestickType::from_str(sub_matches
        .get_one::<String>("candle_type")
        .expect("candle_type is required")
    ).unwrap();

    let candle_input_parameter: u64 = sub_matches.get_one::<String>("candle_configuration")
        .expect("required")
        .parse().expect("candle_configuration should a positive number");

    let ticker = sub_matches.get_one::<String>("ticker").expect("required");
    let market = Market::from_str(sub_matches
        .get_one::<String>("market")
        .expect("market is required")
    ).unwrap();

    let exchange = Exchange::from_str(sub_matches
        .get_one::<String>("exchange")
        .expect("exchange is required")
    ).unwrap();

    let asset = Asset::get_asset(ticker, market, exchange);

    if asset.is_none(){
        eprintln!("[ERROR] asset with ticker '{}' and market '{}' not found", ticker, market);
        return;
    }

    let strategy_name = sub_matches.get_one::<String>("strategy").expect("required");
    let strategy = get_candlestick_strategy_by_name(strategy_name.as_str());

    if strategy.is_none() {
        eprintln!("[ERROR] candlestick strategy with name '{}' not found", strategy_name);
        return;
    }

    let start_load_data = Instant::now();
    let backtester_engine = backtest::TradingBacktester::new(
        &asset.unwrap(),
        begin,
        end,
        CandlestickConfiguration{
            candle_type,
            candle_input_parameter
        },
    );
    let finish_load_data = start_load_data.elapsed();
    print!("historical data loaded in {:.2?} | ", finish_load_data);
    if backtester_engine.is_err() {
        eprintln!("failed to initialize backtest engine. err: {}",
                 backtester_engine.err().unwrap());
        return;
    }

    let backtester = backtester_engine.unwrap();
    println!("{} trading days with {} candlesticks generated from {} trades",
             backtester.qty_history_trading_days,
             backtester.qty_history_candles,
             backtester.qty_history_trades,
    );

    let now = Instant::now();
    let metrics = backtester.run_backtest(strategy.unwrap());
    let elapsed = now.elapsed();
    println!("backtest executed successfully in {:.2?}", elapsed);
    println!("{}", metrics);
}

fn process_run_command(sub_matches: &ArgMatches, client: MarketConnectorClient, configuration: KateConfiguration) {
    let strategy_id = sub_matches.get_one::<String>("strategy").expect("required");
    let ticker = sub_matches.get_one::<String>("ticker").expect("required");

    let account_id = sub_matches
        .get_one::<String>("account")
        .expect("account is required");

    let market = Market::from_str(sub_matches
        .get_one::<String>("market")
        .expect("market is required")
    ).unwrap();

    let strategy_input_data_type = StrategyInputData::from_str(sub_matches
        .get_one::<String>("data")
        .expect("account is required")
    ).unwrap();


    let mut manager = LiveStrategyExecutor::new(
        client,
        configuration,
    );

    let strategy_execution_request = ExecutionRequest::new(
        strategy_input_data_type,
        strategy_id.clone(),
        account_id.clone(),
        ticker.clone(),
        market,
    );

    let result = manager.run_strategy(strategy_execution_request);

    if result.is_err() {
        println!("Error running strategy: {}", result.err().unwrap());
        return;
    }

    println!(
        "started trading ticker {} for the account {} using strategy {}",
        ticker, account_id, strategy_id
    );

    let mut buffer = String::new();
    stdin().read_line(&mut buffer).unwrap();
}

fn process_order_send_command(sub_matches: &ArgMatches, client: MarketConnectorClient) {
    let ticker_symbol = sub_matches
        .get_one::<String>("ticker")
        .expect("ticker type is required");

    let quantity: i32 = sub_matches
        .get_one::<String>("qty")
        .expect("quantity is required").parse().expect("quantity should be a valid integer");

    let account_id = sub_matches
        .get_one::<String>("account")
        .expect("account is required");

    let order_side = Side::from_str(sub_matches
        .get_one::<String>("side")
        .expect("side is required")
    ).unwrap();

    let market = Market::from_str(sub_matches
        .get_one::<String>("market")
        .expect("market is required")
    ).unwrap();

    let order_type = OrderType::from_str(
        sub_matches
            .get_one::<String>("type")
            .expect("order type is required"),
    ).unwrap();

    let side_str = order_side.to_string();

    let order_id = if order_type == OrderType::MARKET {
        client.send_market_order(
            ticker_symbol,
            market,
            order_side,
            quantity,
            account_id,
            constants::MANUAL,
        )
    } else {
        let price: f64 = sub_matches
            .get_one::<String>("price")
            .expect("price is required when sending limit orders")
            .parse().expect("price should be a valid number");

        client.send_limit_order(
            ticker_symbol,
            market,
            order_side,
            price,
            quantity,
            account_id,
            constants::MANUAL,
        )
    };

    println!(
        "kate sent a {} order to {} {} shares/contracts of {} on the accountId {}",
        order_type.to_string(),
        side_str,
        quantity,
        ticker_symbol,
        account_id
    );

    match order_id {
        Ok(id) => {
            println!("successfully sent order. the id for the order is {}", id);
        }
        Err(e) => {
            println!("the order sent failed to be processed. {}", e);
        }
    }
}

fn process_order_details_command(sub_matches: &ArgMatches, client: MarketConnectorClient) {
    let account_id = sub_matches
        .get_one::<i64>("account_id")
        .expect("account id is required");

    let order_id = sub_matches
        .get_one::<i64>("order_id")
        .expect("order id is required");

    match client.get_order_details(*account_id, *order_id) {
        Ok(order) => {
            println!("successfully processed order details. {} {}", order.side, order.ticker);
        }
        Err(e) => {
            println!("error getting order details {}", e);
        }
    };
}

fn process_connection_status_command(client: MarketConnectorClient) {
    println!("checking connection status with market-connector");
    let status_result = client.health_status();
    match status_result {
        Err(e) => {
            println!("failed to reach market-connector server. error: {}", e);
        }
        Ok(is_healthy) => {
            if !is_healthy {
                println!("connected market-connector server, but the server is NOT healthy");
            } else {
                println!("successfully connected to market-connector, and the server is up and running");
            }
        }
    }
}

fn main() {
    let configuration: KateConfiguration = load_configuration();
    let client = MarketConnectorClient::new(
        configuration.market_connector_url.clone(),
        Exchange::B3);

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some((constants::RUN, sub_matches)) => {
            process_run_command(sub_matches, client, configuration);
        }
        Some((constants::BACKTEST, sub_matches)) => {
            process_backtest_command(sub_matches, configuration.clone());
        }
        Some((constants::ORDER, sub_matches)) => {
            match sub_matches.subcommand() {
                Some((constants::SEND, send_sub_matches)) => {
                    process_order_send_command(send_sub_matches, client);
                }
                Some((constants::DETAILS, details_sub_matches)) => {
                    process_order_details_command(details_sub_matches, client);
                }
                _ => unreachable!()
            }
        }
        Some((constants::CONNECTION, sub_matches)) => {
            match sub_matches.subcommand() {
                Some((constants::STATUS, _)) => {
                    process_connection_status_command(client);
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
