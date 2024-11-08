use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(Debug, PartialEq, Eq, IntoStaticStr, Display, EnumString, Clone)]
pub enum Side {
    BUY,
    SELL,
}
