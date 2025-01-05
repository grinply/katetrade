use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use crossbeam_channel::Receiver;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::market::Market;
use crate::services::configuration::KateConfiguration;
use crate::services::data_feed_reader;
use crate::services::market_connector::MarketConnectorClient;
use crate::strategies::strategy::{get_strategy_by_name};
use crate::models::side::Side;
use crate::models::trade::Trade;

#[allow(unused)]
pub trait OrderExecutor {
    fn execute_order_action(&self, action: TradeAction, strategy_name: &str);
}

/// responsible to keep the context and control the execution of HighResolutionTradingStrategy
/// and CandlestickTradingStrategy in live markets
pub struct LiveStrategyExecutor {
    running_strategy_map: HashMap<String, Arc<AtomicBool>>, // HashMap storing strategy constructors
    client: Arc<MarketConnectorClient>,
    configuration: KateConfiguration,
}

/// handles the execution of orders to the live markets
/// this executor has a unique instance for every running CandlestickTradingStrategy or HighResolutionTradingStrategy
pub struct LiveOrderExecutor {
    client: Arc<MarketConnectorClient>,
    ticker: String,
    market: Market,
    pub strategy_id: String,
    account_id: String,
}

pub struct ExecutionRequest {
    strategy_name: String,
    account_id: String,
    ticker: String,
    market: Market,
}

impl LiveOrderExecutor {
    pub fn new(
        client: Arc<MarketConnectorClient>,
        account_id: String,
        ticker: String,
        strategy_name: String,
        market: Market,
    ) -> LiveOrderExecutor {
        LiveOrderExecutor {
            client,
            ticker,
            market,
            strategy_id: strategy_name,
            account_id,
        }
    }
}

impl ExecutionRequest {
    pub fn new(
        strategy_name: String,
        account_id: String,
        ticker: String,
        market: Market,
    ) -> ExecutionRequest {
        ExecutionRequest {
            strategy_name,
            account_id,
            ticker,
            market,
        }
    }
}

impl LiveStrategyExecutor {
    pub fn new(client: MarketConnectorClient, configuration: KateConfiguration) -> Self {
        LiveStrategyExecutor {
            running_strategy_map: HashMap::new(),
            configuration,
            client: Arc::new(client),
        }
    }

    /// Run the strategy in a thread based on the strategy id
    pub fn run_strategy(&mut self, execution_request: ExecutionRequest) -> anyhow::Result<()> {
        let execution_key = self.can_run_strategy(&execution_request)?;

        let trade_datafeed = data_feed_reader::TradeFeedStream::new(
            self.configuration.market_connector_socket_url.as_str(),
            execution_request.ticker.as_str(),
            execution_request.market,
            &self.client,
        );

        let consumer_channel = trade_datafeed.start_reading()?;
        let active = Arc::new(AtomicBool::new(true));
        self.running_strategy_map.insert(execution_key.clone(), active.clone());

        let order_executor = LiveOrderExecutor::new(
            self.client.clone(),
            execution_request.account_id.to_string(),
            execution_request.ticker.to_string(),
            execution_request.strategy_name.to_string().to_lowercase(),
            execution_request.market.clone(),
        );

        self.start_high_resolution_strategy_execution(
            execution_key,
            order_executor,
            consumer_channel,
        );
        Ok(())
    }

    /// validate if all requirements for the provided ExecutionRequest are met
    /// if everything is ok an `execution_key` will be returned as Result
    /// else an Error will be generated with the reason why the execution isn´t valid
    fn can_run_strategy(&self, execution_request: &ExecutionRequest) -> Result<String, anyhow::Error> {
        let running_key = Self::running_strategy_key(
            execution_request.strategy_name.to_lowercase().as_str(),
            execution_request.ticker.as_str(),
            execution_request.market,
            execution_request.account_id.as_str(),
        );

        if self.running_strategy_map.contains_key(running_key.as_str()) {
            anyhow::bail!("the strategy '{}' is already running for the asset '{}' in the account '{}'",
                execution_request.strategy_name.as_str(),
                execution_request.ticker.as_str(),
                execution_request.account_id.as_str()
            );
        }

        let strategy_name = execution_request.strategy_name
            .to_lowercase();

        if get_strategy_by_name(strategy_name.as_str()).is_none() {
            anyhow::bail!("no strategy found for the provided strategy name '{}'", execution_request.strategy_name);
        }
        Ok(running_key)
    }

    fn start_high_resolution_strategy_execution(&self, execution_key: String,
                                                order_executor: LiveOrderExecutor,
                                                consumer_channel: Receiver<Trade>) {
        let running = self.running_strategy_map.get(execution_key.as_str()).unwrap().clone();
        thread::spawn(move || {
            let strategy_name = order_executor.strategy_id.as_str();
            let mut trading_strategy = get_strategy_by_name(strategy_name).unwrap();

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let new_trade = consumer_channel.recv().unwrap(); // this will block the thread until a new trade is available
                let order_action: TradeAction = trading_strategy.process_new_trade(&new_trade);
                order_executor.execute_order_action(order_action, strategy_name);
            }
        });
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
        format!("{}-{}-{}-{}", account_id, ticker, market, strategy_id)
    }
}

impl OrderExecutor for LiveOrderExecutor {
    fn execute_order_action(&self, action: TradeAction, strategy_name: &str) {
        for order_option in action.orders {
            if let Some(order) = order_option {
                self.execute_order(order, strategy_name);
            }
        }
    }
}


impl LiveOrderExecutor {
    fn execute_order(&self, order: OrderAction, strategy_name: &str) {
        let order_id_result = match order {
            OrderAction::MarketLong(qty) => {
                self.client.send_market_order(
                    self.ticker.as_str(),
                    self.market,
                    Side::BUY,
                    qty,
                    self.account_id.as_str(),
                    self.strategy_id.as_str(),
                )
            }
            OrderAction::MarketShort(qty) => {
                self.client.send_market_order(
                    self.ticker.as_str(),
                    self.market,
                    Side::SELL,
                    qty,
                    self.account_id.as_str(),
                    self.strategy_id.as_str(),
                )
            }
            OrderAction::LimitLong(price, qty) => {
                self.client.send_limit_order(
                    self.ticker.as_str(),
                    self.market,
                    Side::BUY,
                    price,
                    qty,
                    self.account_id.as_str(),
                    self.strategy_id.as_str(),
                )
            }
            OrderAction::LimitShort(price, qty) => {
                self.client.send_limit_order(
                    self.ticker.as_str(),
                    self.market,
                    Side::SELL,
                    price,
                    qty,
                    self.account_id.as_str(),
                    self.strategy_id.as_str(),
                )
            }
            OrderAction::Abort => {
                panic!("received an abort command we should understand why")
            }
        };

        if order_id_result.is_ok() {
            let order_id = order_id_result.unwrap();
            println!(
                "{} sent a {:?} order to be executed. resulting id {order_id}",
                strategy_name,
                order,
            );
            //TODO: we should validate that the order execution and also keep track of the position
        } else {
            println!("error sending {:?} order on ticker {}.  {}",
                     order,
                     self.ticker,
                     order_id_result.err().unwrap()
            );
        }
    }
}