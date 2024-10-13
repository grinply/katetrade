use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(Debug, IntoStaticStr, Display, EnumString)]
pub enum OrderType {
    LIMIT,
    MARKET,
}
