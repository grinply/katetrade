mod evaluation;
mod utils;
mod strategies;
mod models;
mod services;
mod helpers;

use std::convert::Into;
use std::string::ToString;
use clap::{arg, Arg, ArgMatches, Command};
const FUTURES: &str = "FUTURES";
const SPOT: &str = "SPOT";
const MARKET: &str = "MARKET";
const LIMIT: &str = "LIMIT";

const BUY: &str = "BUY";

const SELL: &str = "SELL";

fn cli() -> Command {
    Command::new("kate")
        .about(
            "Welcome to katetrade, an algorithm trading system tailored for the Brazillian capital market",
        )
        .version("0.0.1")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("Run one or more trading strategies")
                .arg(
                    arg!(-m --mode <MODE> "Mode of execution for the trading strategy")
                        .value_parser(["live", "backtest"])
                        .num_args(0..=1)
                        .default_value("live")
                        .default_missing_value("live")
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
            Command::new("connection")
                .about("Verify remote connection with live markets and historical data storage")
                .arg(
                    Arg::new("list")
                        .long("list")
                        .short('l')
                        .help("list all available trading strategies"),
                ),
        )
        .subcommand(
            Command::new("order")
                .about("Send and view orders send to the exchange/broker")
                .arg(Arg::new("send"))
                .arg(
                    arg!(--side <SIDE> "Side that of the order")
                        .value_parser([BUY, SELL])
                        .num_args(0..=1)
                        .default_value(BUY)
                        .default_missing_value(BUY)
                )
                .arg(
                    arg!(--type <TYPE> "Type of order that will be used")
                        .value_parser([MARKET, LIMIT])
                        .num_args(0..=1)
                        .default_value(MARKET)
                        .default_missing_value(MARKET)
                )
                .arg(
                    arg!(--market <MARKET> "Market where to send the order")
                        .value_parser([FUTURES, SPOT])
                        .num_args(0..=1)
                        .default_value(FUTURES)
                        .default_missing_value(FUTURES)
                )
                .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'"))
                .arg(arg!(-a --account <ACCOUNT> "AccountId to be used when sending a order to the exchange/broker"))
                .arg(arg!(-q --qty <QUANTITY> "Amount of shares/contracts that will be requested with the order"))
            ,
        )
}

fn process_run_command(sub_matches: &ArgMatches){
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

fn process_order_command(sub_matches: &ArgMatches){
    let order_type = sub_matches
        .get_one::<String>("type")
        .expect("order type is required");

    let ticker_symbol = sub_matches.get_one::<String>("ticker").expect("ticker type is required");
    let quantity = sub_matches.get_one::<String>("qty").expect("quantity is required");
    let order_side = sub_matches.get_one::<String>("side").expect("side is required");
    let account_id = sub_matches.get_one::<String>("account").expect("account is required");

    println!(
        "kate sent a {} order to {} {} shares/contracts of {} on the accountId {}",
        order_type, order_side, quantity , ticker_symbol, account_id
    );
}

fn main() {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            process_run_command(sub_matches);
        }
        Some(("order", sub_matches)) => {
            process_order_command(sub_matches);
        }
        _ => unreachable!(), // If all subcommcaands are defined above, anything else is unreachable
    }
}
