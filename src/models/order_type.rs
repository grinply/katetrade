use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(Debug, PartialEq, Eq, IntoStaticStr, Display, EnumString)]
pub enum OrderType {
    LIMIT,
    MARKET,
}
