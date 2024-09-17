#![allow(unused)]
pub mod evaluation;
pub mod models;
pub mod statistics;
pub mod utils;

use std::io;
use utils::file;

fn main() -> io::Result<()> {
    println!("hello world.");
    if utils::file::read_trades_from_csv("myfile").is_err() {
        println!("failed to load trades file");
    }
    Ok(())
}
