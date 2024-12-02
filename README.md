# Katetrade
Katetrade is a bateries included quantitative trading platform, ready to operate in any market or asset. Using kate you can live trade, backtest and development/designed custom trading strategies, all built-in one complete package.

## Installation
+ You must first install [rust 1.82.0 or higher](https://www.rust-lang.org/) in order to **work with the source code**. Or download a complete executable for your operating system directly from the repository page.
+ Once you have rust installed and configured, run `cargo build --release` to generate a production build on `target/release/katetrade` or `cargo run` to execute locally in development mode.

## Usage
Katetrade is a CLI application where you can **live trade**, **backtest** and **validate trading strategies**.

If you have the `katetrade` binary just run `katetrade --help` to learn more,
if you just now downloaded the source code you can execute by running `cargo run -- --help`

```bash
Welcome to katetrade, your algorithm trading system tailored for the Brazilian capital market

Usage: katetrade <COMMAND>

Commands:
  run         Start trading one or more trading strategies in the real capital markets
  order       Operations related to orders
  backtest    Backtest a trading strategy using historical data
  file        Various operations related to historical files
  connection  General connection information
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
