mod constants;
mod evaluation;
mod models;
mod services;
mod strategies;
mod utils;

use crate::models::order_type::OrderType;
use crate::services::market_connector::MarketConnectorClient;
use clap::{arg, ArgMatches, Command};
use models::exchange::Exchange;
use std::str::FromStr;
use std::string::ToString;
use crate::models::market::Market;
use crate::models::side::Side;

fn cli() -> Command {
    Command::new("kate")
        .about(
            "Welcome to katetrade, an algorithm trading system tailored for the Brazilian capital market",
        )
        .version("0.0.1")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(constants::RUN)
                .about("Start trading one or more trading strategies in the real capital markets")
                .arg(arg!(-s --strategy <STRATEGY> "Identifier (id) for the strategy you want to executed'"))
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'"))
                .arg(arg!(-a --account <ACCOUNT> "AccountId used when sending trades in live mode to the market-connector'"))
                .arg(arg!(--config [configFile] "Optional json configuration file that enables running multiple strategies at once").required(false))
                .arg(arg!(-d --detach "Enables execution in the background").required(false))
                .arg(arg!(-c --connector [marketConnectorHost] "Custom URL to access the market-connector").required(false))
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
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'").required(true))
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

fn process_run_command(sub_matches: &ArgMatches) {
    let mode = sub_matches
        .get_one::<String>("mode")
        .expect("mode is required");
    let strategy = sub_matches.get_one::<String>("strategy").expect("required");
    let symbol = sub_matches.get_one::<String>("ticker").expect("required");

    println!(
        "katetrade started trading ticker {} with mode {} using strategy {}",
        symbol, mode, strategy
    );
}

fn process_order_send_command(sub_matches: &ArgMatches, client: MarketConnectorClient) {
    let ticker_symbol = sub_matches
        .get_one::<String>("ticker")
        .expect("ticker type is required");

    let quantity = sub_matches
        .get_one::<u64>("qty")
        .expect("quantity is required");

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
            *quantity,
            account_id,
            constants::MANUAL,
        )
    } else {
        let price = sub_matches
            .get_one::<f64>("price")
            .expect("price is required when sending limit orders");

        client.send_limit_order(
            ticker_symbol,
            market,
            order_side,
            *price,
            *quantity,
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

fn process_order_details_command(sub_matches: &ArgMatches, client: MarketConnectorClient){
    let account_id = sub_matches
        .get_one::<i64>("account_id")
        .expect("account id is required");

    let order_id = sub_matches
        .get_one::<i64>("order_id")
        .expect("order id is required");

    match client.get_order_details(*account_id, *order_id){
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
        Err(_) => {
            println!("failed to reach market-connector server");
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
    let client = MarketConnectorClient::new("http://market-connector.grinply.com", Exchange::B3);

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some((constants::RUN, sub_matches)) => {
            process_run_command(sub_matches);
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
        _ => unreachable!(), // If all subcommcaands are defined above, anything else is unreachable
    }
}