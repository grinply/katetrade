mod constants;
mod evaluation;
mod models;
mod services;
mod strategies;
mod utils;
mod indicators;
mod cli;

use models::exchange::Exchange;
use services::market_connector::MarketConnectorClient;
use services::configuration::{load_configuration, KateConfiguration};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let configuration: KateConfiguration = load_configuration();
    let client = MarketConnectorClient::new(
        configuration.market_connector_url.clone(),
        Exchange::B3);

    let matches = cli::get_command_matches();
    cli::process_matches(&matches, configuration, client);
}
