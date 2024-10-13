#[derive(Debug)]
#[allow(unused)]
pub enum OrderStatus{
    SentToExchange, // sent to exchange, no feed available yet
    ReceivedPendingValidation, // received by the exchange/broker, but wasn't validated yet
    Filled,
    PartiallyFilled,
    Booked,   // placed in the order book
    Canceled, // canceled by the user
    Rejected, // not accepted by the exchange/broker
    Expired,
    Unknown
}