use std::fmt::{Display};

#[derive(Debug,strum_macros::IntoStaticStr, strum_macros::Display)]
pub enum Market {
    SPOT,
    FUTURES,
    OPTIONS
}