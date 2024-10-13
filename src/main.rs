mod constants;
mod evaluation;
mod helpers;
mod models;
mod services;
mod strategies;
mod utils;

use crate::models::order_type::OrderType;
use crate::services::market_connector::MarketConnectorClient;
use clap::{arg, Arg, ArgMatches, Command};
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
                .about("Run one or more trading strategies")
                .arg(
                    arg!(-m --mode <MODE> "Mode of execution for the trading strategy")
                        .value_parser([constants::LIVE, constants::BACKTEST])
                        .num_args(0..=1)
                        .default_value(constants::LIVE)
                        .default_missing_value(constants::LIVE)
                )
                .arg(arg!(-s --strategy <STRATEGY> "Identifier (id) for the strategy you want to executed'"))
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'"))
                .arg(arg!(-a --account <ACCOUNT> "AccountId used when sending trades in live mode to the market-connector'"))
                .arg(arg!(--config [configFile] "Optional json configuration file that enables running multiple strategies at once").required(false))
                .arg(arg!(-d --detach "Enables execution in the background").required(false))
                .arg(arg!(-c --connector [marketConnectorHost] "Custom URL to access the market-connector").required(false))
            ,
        )
        .subcommand(
            Command::new(constants::CONNECTION)
                .about("General connection information")
                .arg(
                    Arg::new(constants::STATUS)
                        .help("evaluate the network connection with market connector"),
                ),
        )
        .subcommand(
            Command::new(constants::ORDER)
                .about("Send and view orders send to the exchange/broker")
                .arg(Arg::new(constants::SEND))
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
                        .default_value(constants::FUTURES)
                        .default_missing_value(constants::FUTURES)
                )
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'"))
                .arg(arg!(-a --account <ACCOUNT> "AccountId to be used when sending a order to the exchange/broker"))
                .arg(arg!(-q --qty <QUANTITY> "Amount of shares/contracts that will be requested with the order"))
            ,
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

fn process_order_command(sub_matches: &ArgMatches, client: MarketConnectorClient) {
    if sub_matches.contains_id(constants::SEND) {
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
                constants::MANUAL
            )
        }else{
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
                constants::MANUAL
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
}

fn process_connection_command(sub_matches: &ArgMatches, client: MarketConnectorClient) {
    if sub_matches.contains_id(constants::STATUS) {
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
}

fn main() {
    let client = MarketConnectorClient::new("http://market-connector.grinply.com", Exchange::B3);

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some((constants::RUN, sub_matches)) => {
            process_run_command(sub_matches);
        }
        Some((constants::ORDER, sub_matches)) => {
            process_order_command(sub_matches, client);
        }
        Some((constants::CONNECTION, sub_matches)) => {
            process_connection_command(sub_matches, client)
        }
        _ => unreachable!(), // If all subcommcaands are defined above, anything else is unreachable
    }
}