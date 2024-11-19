use crate::strategies::custom::trade_rule_strategy::trade_rule::TradeRule;

pub struct TradeEquation {
    equation_number: u8,
    logic_operators:[bool;4],
}

impl TradeEquation {
    fn new(equation_number: u8, rsi1: bool, pd: bool, rr: bool, rsi2: bool) -> Self{
        assert!(equation_number < 3, "valid equations are between 0 and 3");
        TradeEquation {
            equation_number,
            logic_operators: [rsi1, pd, rr, rsi2]
        }
    }

    pub fn should_open_trade(rule :&TradeRule, rsi1: f64, pd: f64, rr: f64, rsi2: f64) -> bool{
        true
    }
}

