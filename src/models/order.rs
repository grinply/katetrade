use crate::models::side::Side;

#[derive(Debug)]
pub struct Order<'a> {
    pub ticker: &'a str,
    pub side: Side,
}