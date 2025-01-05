use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(Debug, PartialEq, Eq, IntoStaticStr, Display, EnumString, Clone, Copy)]
pub enum Side {
    BUY,
    SELL,
}


impl Side {
    pub fn opposite_side(&self) -> Side {
        match self {
            Side::BUY => Side::SELL,
            Side::SELL => Side::BUY
        }
    }
}