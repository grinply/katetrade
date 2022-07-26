const { Binance } = require('./packages/exchanges')

// Binance.setup({
//   API_KEY: 'api-key-test',
//   API_SECRET: 'api-secret-test'
// })

// console.log(Binance.getActualPrice())

const apiKey = '8TKpAfkulcs91Gw5TDGKF3i0yPMU4AxI0h4NMKZloQ7oVLwaB8CDrwK4zjpHP4xq'
const apiSecret = 'uvW5M9cULbTjCsB78lKqagOlqauZHlHUmsCdQTp5mLSd9Kx0pC89aCjyQewqejtA'
const options = {
  baseURL: 'https://testnet.binance.vision'
}
// const symbol = 'XRPBNB'
const exchange = Binance(apiKey, apiSecret, options)

console.log(exchange.getExchangeName())

// exchange.exchangeInfo({ symbol }).then(response => console.log(response.data))
// exchange.avgPrice(symbol).then(response => console.log(response.data))
// exchange.exchangeInfo(response => {
//   console.log(response.data)
// }, { symbol: 'XRPBNB' })

// exchange.openMarketOrder(symbol, 'BUY', { quantity: '300' }, (response) => {
//   console.log(response.data)
// })

// exchange.account(response => {
//   console.log(response.data)
// })

// exchange.openMarketOrder(symbol, 'SELL', { quantity: '300' }, (response) => {
//   console.log(response.data)
// })

// exchange.openLimitOrder(symbol, 'BUY', { price: '0.0010', quantity: '200', timeInForce: 'GTC' }, (response) => {
//   console.log(response.data)
// })

// exchange.account(response => {
//   console.log(response.data)
// })

// exchange.openOrders().then(response => console.log(response.data))
// exchange.myTrades(symbol).then(response => console.log(response.data))
// cancelAllOrders()

// exchange.openOrders().then(response => console.log(response.data))

// function cancelAllOrders () {
//   exchange.openOrders().then(response => {
//     response.data.map(order =>
//       exchange.cancelOrder(order.symbol, { orderId: order.orderId })
//     )
//   })
// }
