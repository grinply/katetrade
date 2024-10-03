use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Market {
    SPOT,
    FUTURES,
    OPTIONS
}

impl Display for Market {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}