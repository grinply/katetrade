use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Copy, Display, EnumString)]
pub enum Exchange {
    B3,
}
