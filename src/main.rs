mod evaluation;
mod utils;
mod strategies;
mod models;
mod services;
mod helpers;

use clap::{arg, Arg, Command};


fn cli() -> Command {
    Command::new("katetrade")
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
}

fn main() {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("run", sub_matches)) => {
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
        Some(("info", query_matches)) => {
            println!("info!!")
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
