const { EventEmitter } = require('events');

class Candlestick {
  constructor(open, high, low, close, volume) {
    this.open = open;
    this.high = high;
    this.low = low;
    this.close = close;
    this.volume = volume;
  }
}

const orderListener = new EventEmitter();

orderListener.addListener('limitbuy', function (price) {
  //handle limit buy orders
});

orderListener.addListener('limitbuy', function (price) {
  //handle limit sell orders
});

function backtest(candlesticks) {
  //create event emitter for candlestick prices
  const prices = new EventEmitter();
  for (let i = 0; i < candlesticks.length; i++) {
    let currentCandlestick = candlesticks[i];
    prices.emit('newcandle', currentCandlestick);
  }
}

function strategy() {
  //create event emitter for candlestick prices
  const prices = new EventEmitter();
  prices.on('newcandle', function (candlestick) {
    if (candlestick.close > candlestick.open) {
      //emit signal to buy
      prices.emit('limitbuy', candlestick.close);
    } else {
      console.log('limitsell', candlestick.close);
    }
  });
}


