use std::fmt::{Display, Formatter};

#[derive(Debug,strum_macros::IntoStaticStr)]
pub enum OrderType {
    LIMIT,
    MARKET,
}