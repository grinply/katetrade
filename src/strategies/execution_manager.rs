use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use crate::models::market::Market;
use crate::services::configuration::KateConfiguration;
use crate::services::data_feed_reader;
use crate::services::market_connector::MarketConnectorClient;

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
    pub fn start_strategy(&self, strategy_name: &str, ticker: &str, market: Market) -> anyhow::Result<()> {
        let trade_datafeed = data_feed_reader::TradeFeedStream::new(
            self.configuration.market_connector_socket_url.as_str(),
            ticker,
            market,
            &self.client,
        );

        let consumer_channel = trade_datafeed.start_reading()?;
        let active = Arc::new(AtomicBool::new(true));
        let running: Arc<AtomicBool> = active.clone();
        //
        // if !STRATEGY_MAP.contains_key(strategy_name) {
        //     return anyhow::bail!("no strategy available with the name '{}'", strategy_name);
        // }

        if self.running_strategy_map.contains_key(strategy_name) {
            return anyhow::bail!("the strategy '{}' is already running for the asset '{}'", strategy_name, ticker);
        }

        thread::spawn(move || {
            // let mut trading_strategy = STRATEGY_MAP.get(strategy_name).unwrap();
            // while running.load(std::sync::atomic::Ordering::Relaxed) {
            //     let new_trade = consumer_channel.recv().unwrap(); // this should block
            //     trading_strategy.process_new_trade(new_trade);
            // }
        });

        Ok(())
    }

    pub fn stop_strategy(&mut self, strategy_id: String, ticker: &str, market: Market) {
        let key = Self::running_strategy_key(strategy_id, ticker, market);
        if self.running_strategy_map.contains_key(&key) {
            self.running_strategy_map.get(key.as_str()).unwrap()
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn running_strategy_key(strategy_id: String, ticker: &str, market: Market) -> String {
        format!("{}-{}-{}", ticker, market, strategy_id)
    }
}
