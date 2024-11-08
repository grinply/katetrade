use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use crossbeam_channel::Receiver;
use crate::models::action::{OrderAction, TradeAction};
use crate::models::candlestick;
use crate::models::market::Market;
use crate::services::configuration::KateConfiguration;
use crate::services::data_feed_reader;
use crate::services::market_connector::MarketConnectorClient;
use crate::strategies::strategy::{get_candlestick_strategy_by_name, get_high_resolution_strategy_by_name, CandlestickTradingStrategy};
use crate::strategies::strategy::{HighResolutionTradingStrategy, StrategyInputData};
use crate::models::candlestick::{CandlestickGenerator};
use crate::models::side::Side;
use crate::models::trade::Trade;

#[allow(unused)]
pub trait OrderExecutor {
    fn execute_order_action(&self, action: TradeAction);
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
    strategy_input_data: StrategyInputData,
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
        market: Market) -> LiveOrderExecutor {
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
        input_data: StrategyInputData,
        strategy_name: String,
        account_id: String,
        ticker: String,
        market: Market,
    ) -> ExecutionRequest {
        ExecutionRequest {
            strategy_input_data: input_data,
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

        match execution_request.strategy_input_data {
            StrategyInputData::TRADE => self.start_high_resolution_strategy_execution(
                execution_key,
                order_executor,
                consumer_channel,
            ),
            StrategyInputData::CANDLESTICK => self.start_candlestick_strategy_execution(
                execution_key,
                order_executor,
                consumer_channel,
            )
        }

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

        match execution_request.strategy_input_data {
            StrategyInputData::CANDLESTICK => {
                let strategy = get_candlestick_strategy_by_name(strategy_name.as_str());
                if strategy.is_none() {
                    anyhow::bail!("No candlestick strategy found for the provided strategy name '{}'", execution_request.strategy_name);
                }
            }
            StrategyInputData::TRADE => {
                let strategy = get_high_resolution_strategy_by_name(strategy_name.as_str());
                if strategy.is_none() {
                    anyhow::bail!("No high resolution strategy found for the provided strategy name '{}'", execution_request.strategy_name);
                }
            }
        }

        Ok(running_key)
    }

    fn start_candlestick_strategy_execution(&self, execution_key: String,
                                            order_executor: LiveOrderExecutor,
                                            consumer_channel: Receiver<Trade>) {
        let running = self.running_strategy_map.get(execution_key.as_str()).unwrap().clone();

        thread::spawn(move || {
            let mut trading_strategy = get_candlestick_strategy_by_name(order_executor.strategy_id.as_str()).unwrap();
            let mut candlestick_parser = candlestick::select_candlestick_generator(
                trading_strategy.candlestick_configuration());

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let new_trade = consumer_channel.recv().unwrap(); // this will block the thread until a new trade is available
                if let Some(closed_candle) = candlestick_parser.parse_only_closed_candles(&new_trade) {
                    let action: TradeAction = trading_strategy.process_closed_candlestick(&closed_candle);
                    order_executor.execute_order_action(action);
                }
            }
        });
    }

    fn start_high_resolution_strategy_execution(&self, execution_key: String,
                                                order_executor: LiveOrderExecutor,
                                                consumer_channel: Receiver<Trade>) {
        let running = self.running_strategy_map.get(execution_key.as_str()).unwrap().clone();
        thread::spawn(move || {
            let mut trading_strategy = get_high_resolution_strategy_by_name(order_executor.strategy_id.as_str()).unwrap();

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let new_trade = consumer_channel.recv().unwrap(); // this will block the thread until a new trade is available
                let order_action: TradeAction = trading_strategy.process_new_trade(new_trade);
                order_executor.execute_order_action(order_action);
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
    fn execute_order_action(&self, action: TradeAction) {
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
                Some(OrderAction::MarketLong) => self.client.
                    send_market_order(self.ticker.as_str(), self.market, Side::BUY, 1, self.account_id.as_str(), self.strategy_id.as_str()),
                Some(OrderAction::MarketShort) => self.client.
                    send_market_order(self.ticker.as_str(), self.market, Side::SELL, 1, self.account_id.as_str(), self.strategy_id.as_str()),
                Some(OrderAction::LimitLong(price_requested)) => self.client.
                    send_limit_order(self.ticker.as_str(), self.market, Side::BUY, price_requested, 1, self.account_id.as_str(), self.strategy_id.as_str()),
                Some(OrderAction::LimitShort(price_requested)) => self.client.
                    send_limit_order(self.ticker.as_str(), self.market, Side::SELL, price_requested, 1, self.account_id.as_str(), self.strategy_id.as_str()),
                None => unreachable!()
            };

            let order_type = order_action.unwrap();
            if order_id_result.is_ok() {
                let order_id = order_id_result.unwrap();
                println!("received order_id {} from executing {:?} of {} contracts for the ticker {}",
                         order_id, order_type, 1, self.ticker);
                //TODO: we must validate the order execution and also keep track of the position
            } else {
                println!("{}", order_id_result.err().unwrap());
            }
        }
    }
}