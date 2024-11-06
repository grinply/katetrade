use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
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

/// responsible for keep the context and control all TradingStrategies running in the real markets
pub struct LiveTradingManager {
    running_strategy_map: HashMap<String, Arc<AtomicBool>>, // HashMap storing strategy constructors
    client: Arc<MarketConnectorClient>,
    configuration: KateConfiguration,
}

pub struct RealExecutionManager {
    client: Arc<MarketConnectorClient>,
    ticker: String,
    market: Market,
    strategy_id: String,
    account_id: String,
}

impl RealExecutionManager {
    pub fn new_real_trading_manager(
        client: Arc<MarketConnectorClient>,
        account_id: String,
        ticker: String,
        strategy_name: String,
        market: Market) -> RealExecutionManager {
        RealExecutionManager {
            client,
            ticker,
            market,
            strategy_id: strategy_name,
            account_id,
        }
    }

    fn process_order_execution(&self, action: TradeAction) {
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

pub struct ExecutionRequest {
    strategy_input_data: StrategyInputData,
    strategy_name: String,
    account_id: String,
    ticker: String,
    market: Market,
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


impl LiveTradingManager {
    pub fn new(client: MarketConnectorClient, configuration: KateConfiguration) -> Self {
        LiveTradingManager {
            running_strategy_map: HashMap::new(),
            configuration,
            client: Arc::new(client),
        }
    }

    /// Run the strategy in a thread based on the strategy id
    pub fn start_strategy(&mut self, execution_request: ExecutionRequest) -> anyhow::Result<()> {
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

        let trade_datafeed = data_feed_reader::TradeFeedStream::new(
            self.configuration.market_connector_socket_url.as_str(),
            execution_request.ticker.as_str(),
            execution_request.market,
            &self.client,
        );

        let consumer_channel = trade_datafeed.start_reading()?;
        let active = Arc::new(AtomicBool::new(true));
        self.running_strategy_map.insert(running_key, active.clone());

        let running: Arc<AtomicBool> = active.clone();
        let strategy_name_clone = execution_request.strategy_name.to_string();

        let client = self.client.clone();
        let ticker = execution_request.ticker.to_string();
        let market = execution_request.market.clone();
        let account_id = execution_request.account_id.to_string();
        let strategy_name = strategy_name_clone.to_lowercase().clone();
        let input_data_type = execution_request.strategy_input_data.clone();

        thread::spawn(move || {
            let position_manager = RealExecutionManager::new_real_trading_manager(
                client.clone(),
                account_id.to_string(),
                ticker.to_string(),
                strategy_name.to_string(),
                market,
            );

            match input_data_type {
                StrategyInputData::TRADE => {
                    let mut trading_strategy = get_high_resolution_strategy_by_name(strategy_name.as_str()).unwrap();

                    while running.load(std::sync::atomic::Ordering::Relaxed) {
                        let new_trade = consumer_channel.recv().unwrap();
                        let order_action: TradeAction = trading_strategy.process_new_trade(new_trade);
                        position_manager.process_order_execution(order_action);
                    }
                }
                StrategyInputData::CANDLESTICK => {
                    let mut trading_strategy = get_candlestick_strategy_by_name(strategy_name.as_str()).unwrap();
                    let mut candlestick_parser = candlestick::select_candlestick_generator(
                        trading_strategy.candlestick_configuration());

                    while running.load(std::sync::atomic::Ordering::Relaxed) {
                        let new_trade = consumer_channel.recv().unwrap();
                        match candlestick_parser.parse_only_closed_candles(&new_trade) {
                            Some(closed_candle) => {
                                let order_action: TradeAction = trading_strategy.process_closed_candlestick(closed_candle);
                                position_manager.process_order_execution(order_action);
                            }
                            None => ()
                        }
                    }
                }
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
        format!("{}-{}-{}-{}", account_id, ticker, market, strategy_id)
    }
}