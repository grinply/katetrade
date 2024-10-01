# Katetrade
Katetrade is  for algorithmic trading platform tailord for the Brazillian market. Using kate you can live trade, backtest and development/designed custom trading strategies, all built-in one complete package.

## Usage
Katetrade is a CLI application where you can live trade, backtest and validate trading strategies.

If you have the `katetrade` binary just run `katetrade --help` to learn more,
if you just now downloaded the source code you can execute by running `cargo run -- --help`

```bash
Welcome to katetrade, an algorithm trading system tailored for the Brazillian capital market

Usage: katetrade <COMMAND>

Commands:
  run         Run one or more trading strategies
  connection  Verify remote connection with live markets and historical data storage
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```


## Installation
+ You must first install [rust 1.76.0 or higher](https://www.rust-lang.org/) in order to **work with the source code**. Or you can download complete executable from the repository page.
+ Run `cargo build --release` to generate a production build on `target/release/katetrade` or `cargo run` to execute locally in development mode.
