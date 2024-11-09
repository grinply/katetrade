use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Display, EnumString)]
pub enum Exchange {
    B3,
}
