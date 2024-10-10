use std::fmt::Display;

#[derive(Debug)]
pub enum Side {
    BUY,
    SELL
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}