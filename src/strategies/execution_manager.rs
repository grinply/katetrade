use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::market::Market;
use crate::services::configuration::KateConfiguration;
use crate::services::data_feed_reader;
use crate::services::market_connector::MarketConnectorClient;
use crate::strategies::strategy::{create_strategy, TradingStrategy};
use crate::{strategy_dispatch, strategy_process_new_trade};
use crate::models::side::Side;

pub struct ExecutionManager {
    running_strategy_map: HashMap<String, Arc<AtomicBool>>, // HashMap storing strategy constructors
    client: Arc<MarketConnectorClient>,
    configuration: KateConfiguration,
}

impl ExecutionManager {
    pub fn new(client: MarketConnectorClient, configuration: KateConfiguration) -> Self {
        ExecutionManager {
            running_strategy_map: HashMap::new(),
            configuration,
            client: Arc::new(client),
        }
    }

    /// Run the strategy in a thread based on the strategy id
    pub fn start_strategy(&self, strategy_name: &str, account_id: &str, ticker: &str, market: Market) -> anyhow::Result<()> {
        let running_key = Self::running_strategy_key(strategy_name.to_lowercase().as_str(), ticker, market, account_id);
        if self.running_strategy_map.contains_key(running_key.as_str()) {
            anyhow::bail!("the strategy '{}' is already running for the asset '{}' in the account '{}'",
                strategy_name, ticker, account_id);
        }

        let strategy = create_strategy(strategy_name.to_lowercase().as_str());
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

        let client = self.client.clone();
        let ticker = ticker.to_string();
        let market = market.clone();
        let account_id = account_id.to_string();
        let strategy_name = strategy_name_clone.to_lowercase().clone();

        thread::spawn(move || {
            let account = account_id.as_str();
            let ticker_symbol = ticker.as_str();
            let strategy_id = strategy_name.as_str();

            let mut trading_strategy = create_strategy(strategy_id).unwrap();

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let new_trade = consumer_channel.recv().unwrap();
                let order_action: TradeAction = strategy_process_new_trade!(trading_strategy, new_trade);
                Self::process_order_execution(&client, order_action, account, ticker_symbol, strategy_id, market);
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

    fn process_order_execution(client: &Arc<MarketConnectorClient>,
                               action: TradeAction,
                               account_id: &str,
                               ticker: &str,
                               strategy_name: &str,
                               market: Market
    ){
        let actions: [Option<OrderAction>; 3] = [
            action.enter_market_action,
            action.stop_loss_action,
            action.take_profit_action
        ];

        for order_action in actions {
            if order_action.is_none() {
                continue;
            }

            let order_id_result = match order_action {
                Some(OrderAction::MarketLong) => client.
                    send_market_order(ticker, market, Side::BUY, 1, account_id, strategy_name),
                Some(OrderAction::MarketShort) => client.
                    send_market_order(ticker, market, Side::SELL, 1, account_id, strategy_name),
                Some(OrderAction::LimitLong(price_requested)) => client.
                    send_limit_order(ticker, market, Side::BUY, price_requested, 1, account_id, strategy_name),
                Some(OrderAction::LimitShort(price_requested)) => client.
                    send_limit_order(ticker, market, Side::SELL, price_requested, 1, account_id, strategy_name),
                None => unreachable!()
            };

            let order_type = order_action.unwrap();
            if order_id_result.is_ok() {
                let order_id = order_id_result.unwrap();
                println!("received order_id {} from executing {:?} of {} contracts for the ticker {}",
                         order_id, order_type, 1, ticker);
                //TODO: we must validate the order execution and also keep track of the position
            }else{
                println!("failed when trying to execute order {:?} for ticker {} in the account {}",
                         order_type, ticker, account_id);
            }
        }
    }
}
