use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use crate::models::action::TradeAction;
use crate::models::market::Market;
use crate::services::configuration::KateConfiguration;
use crate::services::data_feed_reader;
use crate::services::market_connector::MarketConnectorClient;
use crate::strategies::strategy::{create_strategy, TradingStrategy};
use crate::{strategy_dispatch, strategy_process_new_trade};


pub struct ExecutionManager {
    running_strategy_map: HashMap<String, Arc<AtomicBool>>, // HashMap storing strategy constructors
    client: MarketConnectorClient,
    configuration: KateConfiguration,
}

impl ExecutionManager {
    pub fn new(client: MarketConnectorClient, configuration: KateConfiguration) -> Self {
        ExecutionManager {
            running_strategy_map: HashMap::new(),
            configuration,
            client,
        }
    }

    /// Run the strategy in a thread based on the strategy id
    pub fn start_strategy(&self, strategy_name: &str, account_id: &str, ticker: &str, market: Market) -> anyhow::Result<()> {
        let running_key = Self::running_strategy_key(strategy_name, ticker, market, account_id);
        if self.running_strategy_map.contains_key(running_key.as_str()) {
            anyhow::bail!("the strategy '{}' is already running for the asset '{}' in the account '{}'",
                strategy_name, ticker, account_id);
        }

        let strategy = create_strategy(strategy_name);
        if strategy.is_none() {
            anyhow::bail!("No strategy found for the provided strategy name '{}'", strategy_name);
        }

        let trade_datafeed = data_feed_reader::TradeFeedStream::new(
            self.configuration.market_connector_socket_url.as_str(),
            ticker,
            market,
            &self.client,
        );

        let consumer_channel = trade_datafeed.start_reading()?;
        let active = Arc::new(AtomicBool::new(true));
        let running: Arc<AtomicBool> = active.clone();
        let strategy_name_clone = strategy_name.to_string();

        thread::spawn(move || {
            let strategy_name = strategy_name_clone.clone();
            let mut trading_strategy = create_strategy(strategy_name.clone().as_str()).unwrap();
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let new_trade = consumer_channel.recv().unwrap();
                let _: TradeAction = strategy_process_new_trade!(trading_strategy, new_trade);
            }
        });

        Ok(())
    }

    #[allow(unused)]
    pub fn stop_strategy(&mut self, account_id: &str, strategy_id: &str, ticker: &str, market: Market) {
        let key = Self::running_strategy_key(strategy_id, ticker, market, account_id);
        if self.running_strategy_map.contains_key(&key) {
            self.running_strategy_map.get(key.as_str()).unwrap()
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn running_strategy_key(strategy_id: &str, ticker: &str, market: Market, account_id: &str) -> String {
        format!("{}-{}-{}-{}", account_id,  ticker, market, strategy_id)
    }
}
