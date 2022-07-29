const { Binance } = require('../packages/exchanges')
const chai = require('chai')
const sinon = require('sinon')
const { describe, it, before } = require('mocha')
const expect = chai.expect

// const apiKey = 'api_key'
// const apiSecret = 'api_secret'
const apiKey = 'api_key'
const apiSecret = 'secret_key'
const options = { baseURL: 'https://testnet.binance.vision' }
const exchange = Binance(apiKey, apiSecret, options)
const connector = exchange._getConnector()

describe('Binance connector', () => {
  describe('basic functions', () => {
    it('should return exchangeName as binance', () => {
      expect(exchange.getExchangeName()).to.equal('binance')
    })
  })

  describe('information functions', () => {
    before(() => {
      sinon
        .stub(connector, 'account')
        .resolves({ data: stubAccount })
    })

    it('Should return account information', () => {
      const promise = exchange.accountInfo()
      return promise.then(response => {
        expect(response.data).to.have.property('balances')
          .that.to.be.an('array')
          .to.deep.include({ asset: 'BNB', free: '1001.62570000', locked: '0.00000000' })
        expect(response.data).to.have.property('accountType').that.to.equal('SPOT')
      })
    })

    it('should return avarage price from BNB', () => {

    })
  })
})

const stubAccount = {
  makerCommission: 0,
  takerCommission: 0,
  buyerCommission: 0,
  sellerCommission: 0,
  canTrade: true,
  canWithdraw: false,
  canDeposit: false,
  accountType: 'SPOT',
  balances: [
    { asset: 'BNB', free: '1001.62570000', locked: '0.00000000' }
  ],
  permissions: ['SPOT']
}
