use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(Copy, Clone, Debug, IntoStaticStr, Display, EnumString)]
pub enum Market {
    SPOT,
    FUTURES,
    OPTIONS,
}
