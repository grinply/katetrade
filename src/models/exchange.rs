use std::fmt::Display;

#[derive(Debug)]
pub enum Exchange {
    B3,
}

impl Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}