const binance = (apiKey, apiSecret, options = {}) => {
  const Binance = this
  const { Spot } = require('@binance/connector')
  Binance.client = new Spot(apiKey, apiSecret, options)

  function _newOrder (symbol, side, type, options, callback) {
    Binance.client.newOrder(symbol, side, type, {
      ...options
    }).then(callback)
  }

  return {

    account: function (callback) {
      Binance.client.account().then(callback)
    },

    exchangeInfo: (options = {}) => {
      return Binance.client.exchangeInfo(options)
    },

    avgPrice: (symbol) => {
      return Binance.client.avgPrice(symbol)
    },

    /**
     *
     * @param {string} symbol Symbol of currency used to open an order
     * @param {string} side Order side ('BUY' or 'SELL')
     * @param {{
     *            price: string,
     *            quantity: string,
     *            timeInForce: string,
     *            ...otherParams
     *        }} options Options defined in https://binance.github.io/binance-connector-node/module-Trade.html#newOrder
     *                   (price, quantity, timeInForce is Mandatory)
     * @param {function} callback
     */
    openLimitOrder: (symbol, side, options, callback) => {
      if (!options.price || !options.quantity || !options.timeInForce) {
        throw new Error('OpenLimitOrder function need price, quantity and timeInForce parameters')
      }

      _newOrder(symbol, side, 'LIMIT', options, callback)
    },

    /**
     *
     * @param {string} symbol Symbol of currency used to open an order
     * @param {string} side Order side ('BUY' or 'SELL')
     * @param {{
     *            quantity: string,
     *            quoteOrderQty: string,
     *            ...otherParams
     *        }} options Options defined in https://binance.github.io/binance-connector-node/module-Trade.html#newOrder
     *                   (quantity or quoteOrderQty is Mandatory)
     * @param {function} callback
     */
    openMarketOrder: (symbol, side, options, callback) => {
      if (!options.quantity && !options.quoteOrderQty) {
        throw new Error('OpenMarketOrder function need quantity and quoteOrderQty parameters')
      }

      _newOrder(symbol, side, 'MARKET', options, callback)
    },

    /**
     *
     * @param {string} symbol Symbol of currency used to open an order
     * @param {string} side Order side ('BUY' or 'SELL')
     * @param {*} options Options defined in https://binance.github.io/binance-connector-node/module-Trade.html#newOrder
     * @param {*} callback
     */
    openNewOrder: (symbol, side, type, options, callback) => {
      _newOrder(symbol, side, type, options, callback)
    },

    /**
     *
     * @param {string} symbol
     * @param {object} options Either orderId or origClientOrderId must be sent. If both orderId and origClientOrderId are provided, orderId takes precedence.
     * @param {function} callback
     */
    cancelOrder: (symbol, options, callback) => {
      Binance.client.cancelOrder(symbol, options).then(callback)
    },

    openOrders: (options = {}) => {
      return Binance.client.openOrders(options)
    },

    myTrades: (symbol, options = {}) => {
      return Binance.client.myTrades(symbol, options)
    },

    getExchangeName: () => {
      return 'binance'
    }
    // openLimitOrder - Coloca uma ordem de compra ou venda no orderBook
    // openMarketOrder - Executa uma ordem de compra ou venda a preço de mercado
    // cancelLimitOrder - Cancela Limit Order
    // getCandles(qty) - Retorna um array de candles
    // getCandlesInterval(dataInicio, dataFim) - Retorna um array de candles em um intervalo
    // getTradeRules - quantidade minima, quantidade maxima e outras informações do par
    // "smart" Orders -
    // "shadow" Orders -

  }
}

module.exports = binance
