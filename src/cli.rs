use std::io::stdin;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use clap::{arg, ArgMatches, Command};
use rayon::iter::IntoParallelIterator;
use simple_bar::ProgressBar;
use crate::{constants, VERSION};
use crate::constants::OPTIMIZE;
use crate::models::asset::Asset;
use crate::models::exchange::Exchange;
use crate::models::market::Market;
use crate::models::order_type::OrderType;
use crate::models::side::Side;
use crate::models::{candlestick, trade};
use rayon::iter::ParallelIterator;
use crate::models::candlestick::{Candlestick, TimeCandlestickGenerator, TimeUnit};
use crate::services::backtest::backtest::TradingBacktester;
use crate::services::configuration;
use crate::services::configuration::KateConfiguration;
use crate::services::market_connector::MarketConnectorClient;
use crate::services::optimization::brkga::{run_brkga_optimization, BrkgaConfig};
use crate::strategies::custom::trade_rule_strategy::optimization::fitness_optimizer_trade_rule_strategy;
use crate::strategies::execution_manager::{ExecutionRequest, LiveStrategyExecutor};
use crate::strategies::strategy;
use crate::strategies::strategy::{get_candlestick_strategy_by_name, get_high_resolution_strategy_by_name, CandlestickTradingStrategy, HighResolutionTradingStrategy, StrategyInputData};

pub fn process_matches(matches: &ArgMatches, configuration: KateConfiguration, client: MarketConnectorClient) {
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
        Some((constants::FILE, sub_matches)) => {
            match sub_matches.subcommand() {
                Some((constants::TRANSFORM, transform_match)) => {
                    parse_file_command(transform_match);
                }
                _ => unreachable!(),
            }
        }
        Some((OPTIMIZE, sub_matches)) => {
            optimize_trade_parameters_command(sub_matches);
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}

pub fn get_command_matches() -> ArgMatches {
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
                .arg(arg!(-q --qty <QUANTITY> "Amount of shares/contracts that will be requested with the order").required(true))
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
                .arg(arg!(--no_download <NO_DOWNLOAD> "block any attempt to verify/download external files")
                    .default_value("false")
                    .num_args(0..=1)
                    .required(false))
                .arg(arg!(--cache_on_memory <CACHE_ON_MEMORY> "select if the complete dataset used by the backtest should be stored in main memory ( will greatly increase execution speed at the cost of available memory)")
                    .default_value("true")
                    .num_args(0..=1)
                    .required(false))
                .arg(arg!(-q --qty_to_trade <QUANTITY_TO_TRADE> "base traded quantity to trade on each order sent to the market")
                    .default_value("1")
                    .required(false))
                .arg(arg!(--fixed_trade_qty <FIXED_TRADE_QTY> "defines if the quantity traded should increase and decrease as the current balance changes to keep the traded amount proportional")
                    .default_value("true")
                    .required(false))
        )
        .subcommand(
            Command::new(constants::FILE)
                .about("Various operations related to historical files")
                .subcommand(
                    Command::new(constants::TRANSFORM)
                        .about("transform files from a type to another")
                        .arg(arg!(-e --exchange <EXCHANGE> "Exchange where the ticker belongs to")
                            .value_parser([constants::B3])
                            .default_value(constants::B3)
                            .num_args(0..=1))
                        .arg(
                            arg!(-m --market <MARKET> "Market where to send the order")
                                .value_parser([constants::FUTURES, constants::SPOT])
                                .num_args(0..=1)
                                .required(true)
                        )
                        .arg(arg!(--begin <BEGIN_DATE> "Start trading day to reading data from, in the format: yyyyMMdd"))
                        .arg(arg!(--end <END_DATE> "End trading day to finish reading data from, in the format: yyyyMMdd"))
                        .arg(arg!(-t --ticker <TICKET_SYMBOL> "Ticker symbol for the asset to be traded'").required(true))
                        .arg(arg!(-i --input <INPUT> "Full path of the input file").required(false))
                        .arg(arg!(--frequency <frequency> "Time frequency of each candlestick").required(false))
                        .arg(arg!(-i --input <INPUT> "Full path of the input file").required(false))
                        .arg(arg!(-o --output <OUTPUT> "Full path of the output directory (if you dont provide an output folder the result will be saved to kate home folder)")
                            .required(false))
                        .arg(arg!(-r --result <RESULT> "resulting file type that will be generated from the provided input")
                            .value_parser(["csv", "binary"])
                            .default_value("binary")
                            .num_args(0..=1))
                )
        )
        .subcommand(
            Command::new(constants::CONNECTION)
                .about("General connection information")
                .subcommand(
                    Command::new(constants::STATUS)
                        .about("evaluate the network connection with market connector (service responsible for processing orders and providing live market data")
                )
        )
        .subcommand(
            Command::new(OPTIMIZE)
                .about("Optimize/train a group for best solutions for a customized trading strategy")
        ).get_matches()
}


fn parse_file_command(sub_matches: &ArgMatches) {
    let ticker = sub_matches.get_one::<String>("ticker").expect("required");
    let market = Market::from_str(sub_matches
        .get_one::<String>("market")
        .expect("market is required")
    ).unwrap();

    let exchange = Exchange::from_str(sub_matches
        .get_one::<String>("exchange")
        .expect("exchange is required")
    ).unwrap();

    let result = sub_matches.get_one::<String>("result").expect("required");
    let output = sub_matches.get_one::<String>("output");


    if output.unwrap().ends_with(".csv") && result == "csv" {
        let begin: u32 = sub_matches.get_one::<String>("begin").expect("it is required to provide the begin date to read data from")
            .parse().expect("begin should a positive number in the format yyyyMMdd");
        let end: u32 = sub_matches.get_one::<String>("end").expect("it is required to provide the end date to read data from")
            .parse().expect("end should a positive number in the format yyyyMMdd");

        println!("parsing data from {} until {}", begin, end);
        let asset = Asset::get_asset(ticker, market, exchange).expect("should have found a asset for the ticker/market and exchange provided");

        let candle_parser = TimeCandlestickGenerator::new(TimeUnit::Seconds, 10);
        let backtester = TradingBacktester::new(
            asset,
            begin,
            end,
            true,
            Some(Box::new(candle_parser)),
            true
        ).unwrap();

        candlestick::write_to_csv_file(
            output.unwrap().as_str(),
            backtester.data_provider.candlesticks
        ).unwrap();

    }

    //converting from nelogica .csv into multiple katetrade binary files
    if result == "binary" {
        let input_file_path = sub_matches.get_one::<String>("input").expect("required");
        assert!(input_file_path.ends_with(".csv"), "when converting to binary the only supported input format is csv");

        eprintln!("currently only output as binary is supported");
        println!("evaluating how many trading days are available on the csv file");
        let trading_days_present_in_csv = trade::get_available_trading_days_from_csv(input_file_path).unwrap();
        println!("{} trading days found in the csv, starting from {} and ending in {}",
                 trading_days_present_in_csv.len(),
                 trading_days_present_in_csv.first().unwrap(),
                 trading_days_present_in_csv.last().unwrap()
        );

        let output_file_path = if output.is_some() {
            output.unwrap().to_string()
        } else {
            configuration::kate_configuration_directory()
                .join("history_trades")
                .join(ticker).to_str().unwrap().to_string()
        };

        println!("parsing trades and saving one binary file for each trading day inside the folder {}", output_file_path.as_str());
        let mut bar = ProgressBar::default(trading_days_present_in_csv.len() as u32 + 1, 100, false);
        bar.update();

        let download_files_bar = Arc::new(Mutex::new(bar));

        let values_to_process: Vec<(i32, Arc<Mutex<ProgressBar>>)> = trading_days_present_in_csv.iter().map(|trading_date| {
            (*trading_date, Arc::clone(&download_files_bar))
        }).collect();

        values_to_process.into_par_iter().for_each(|tuple| {
            let trading_date = tuple.0;
            let progress_bar = tuple.1;
            let trades = trade::read_trades_for_exact_day_from_csv(trading_date, input_file_path).unwrap();
            let file_name = format!("TRADE-{exchange}-{market}-{ticker}-{trading_date}");
            let full_output_file_path = PathBuf::from(output_file_path.as_str()).join(file_name);
            trade::write_to_binary_file(trades.as_slice(), full_output_file_path.to_str().unwrap())
                .expect("should persist file correctly");
            let mut download_progress_bar = progress_bar.lock().unwrap();
            download_progress_bar.update();
        });
    }


}

fn process_backtest_command(sub_matches: &ArgMatches, _: KateConfiguration) {
    let begin: u32 = sub_matches.get_one::<String>("begin").expect("required")
        .parse().expect("begin should a positive number in the format yyyyMMdd");
    let end: u32 = sub_matches.get_one::<String>("end").expect("required")
        .parse().expect("end should a positive number in the format yyyyMMdd");

    let no_download: bool = sub_matches
        .get_one::<String>("no_download")
        .expect("no_download is required")
        .parse().expect("no_download should be a bool");

    let cache_on_memory: bool = sub_matches.get_one::<String>("cache_on_memory")
        .expect("cache_on_memory is required")
        .parse().expect("cache_on_memory should be a bool");


    let ticker = sub_matches.get_one::<String>("ticker").expect("required");
    let market = Market::from_str(sub_matches
        .get_one::<String>("market")
        .expect("market is required")
    ).unwrap();

    let exchange = Exchange::from_str(sub_matches
        .get_one::<String>("exchange")
        .expect("exchange is required")
    ).unwrap();

    let qty_to_trade: u32 = if sub_matches.get_one::<String>("qty_to_trade").is_some() {
        sub_matches.get_one::<String>("qty_to_trade").expect("required")
            .parse().expect("should be a positive number")
    } else {
        1
    };

    let fixed_trade_qty: bool = if sub_matches.get_one::<String>("fixed_trade_qty").is_some() {
        sub_matches.get_one::<String>("fixed_trade_qty").expect("required")
            .parse().expect("should be a boolean value")
    } else {
        true
    };

    let asset = Asset::get_asset(ticker, market, exchange);

    if asset.is_none() {
        eprintln!("[ERROR] asset with ticker '{}' and market '{}' not found", ticker, market);
        return;
    }

    let strategy_name = sub_matches.get_one::<String>("strategy").expect("required");
    let candlestick_strategy = get_candlestick_strategy_by_name(strategy_name.as_str());
    let high_resolution_strategy = get_high_resolution_strategy_by_name(strategy_name);

    if candlestick_strategy.is_none() && high_resolution_strategy.is_none() {
        eprintln!("[ERROR] no strategy with name '{}' not found", strategy_name);
        return;
    }

    if let Some(strategy) = high_resolution_strategy {
        run_high_resolution_backtest(
            strategy,
            begin,
            end,
            asset.unwrap(),
            qty_to_trade,
            fixed_trade_qty,
            cache_on_memory,
            no_download
        );
    }

    if let Some(strategy) = candlestick_strategy {
        run_candlestick_backtest(
            strategy,
            strategy_name,
            begin,
            end,
            asset.unwrap(),
            qty_to_trade,
            fixed_trade_qty,
            cache_on_memory,
            no_download
        );
    }
}


fn run_high_resolution_backtest(
    strategy: Box<dyn HighResolutionTradingStrategy>,
    begin: u32,
    end: u32,
    asset: &Asset,
    qty_to_trade: u32,
    fixed_trade_qty: bool,
    cache_on_memory: bool,
    no_download: bool
) {
    let start_load_data = Instant::now();
    let backtester_engine = TradingBacktester::new(
        &asset,
        begin,
        end,
        cache_on_memory,
        None,
        no_download
    );

    let finish_load_data = start_load_data.elapsed();
    if backtester_engine.is_err() {
        eprintln!("failed to initialize backtest engine. err: {}",
                  backtester_engine.err().unwrap());
        return;
    }

    print!("historical data loaded in {:.2?} | ", finish_load_data);
    let backtester = backtester_engine.unwrap();
    println!("{}", backtester);

    let now = Instant::now();
    let metrics = backtester.run_high_resolution_backtest(strategy, qty_to_trade, fixed_trade_qty);
    let elapsed = now.elapsed();
    println!("backtest executed successfully in {:.2?}", elapsed);
    println!("{}", metrics);
}

fn run_candlestick_backtest(
    strategy: Box<dyn CandlestickTradingStrategy>,
    strategy_name: &str,
    begin: u32,
    end: u32,
    asset: &Asset,
    qty_to_trade: u32,
    fixed_trade_qty: bool,
    cache_on_memory: bool,
    no_download: bool
) {
    let candle_parser = strategy::get_candlestick_generator_for_strategy(strategy_name)
        .unwrap();
    let start_load_data = Instant::now();
    let backtester_engine = TradingBacktester::new(
        &asset,
        begin,
        end,
        cache_on_memory,
        Some(candle_parser),
        no_download,
    );

    let finish_load_data = start_load_data.elapsed();
    if backtester_engine.is_err() {
        eprintln!("failed to initialize backtest engine. err: {}",
                  backtester_engine.err().unwrap());
        return;
    }
    print!("historical data loaded in {:.2?} | ", finish_load_data);
    let backtester = backtester_engine.unwrap();
    println!("{}", backtester);

    let now = Instant::now();
    let metrics = backtester.run_candlestick_backtest(strategy, qty_to_trade, fixed_trade_qty);
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

    let quantity_to_trade: u32 = sub_matches
        .get_one::<String>("qty")
        .expect("quantity is required")
        .parse().expect("quantity should be a valid integer");

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
        quantity_to_trade
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

    let quantity: usize = sub_matches
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

fn optimize_trade_parameters_command(_: &ArgMatches) {
    let asset = Asset::get_asset("WIN", Market::FUTURES, Exchange::B3)
        .expect("must provide an existing/registered asset to trade");

    // BRKGA optimization configuration
    // internal BRKGA parameters are hard coded
    // if you must tune this you can recompile the code
    let population_size = 15_000;
    let max_iterations = 1000;
    let fraction_top = 0.1;
    let fraction_bottom = 0.3;
    let elitism_rate = 0.6;
    let seed = 11223344;

    let training_start_date = 2022_01_01;
    let training_end_date = 2024_02_28;
    let validation_start_date = 2024_06_13;
    let validation_end_date = 2024_11_20;

    // if in the future we have another implementation that has optimizations available we can have
    // a new parameter from the CLI to define which one to choose
    let trade_rule_fitness_executor = fitness_optimizer_trade_rule_strategy(
        asset,
        training_start_date,
        training_end_date,
        validation_start_date,
        validation_end_date,
    );

    let brkga_configuration = BrkgaConfig::new(
        fraction_top,
        fraction_bottom,
        population_size,
        max_iterations,
        elitism_rate,
    );

    run_brkga_optimization(
        seed,
        brkga_configuration,
        trade_rule_fitness_executor,
        true,
    );
}
