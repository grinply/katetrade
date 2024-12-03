# Katetrade
Katetrade is a bateries included quantitative trading platform, ready to operate in any market or asset. Using kate you can live trade, backtest and development/designed custom trading strategies, all built-in one complete package.

## Installation
### Manually
+ You must first install [rust 1.82.0 or higher](https://www.rust-lang.org/) in order to **work with the source code**. Or download a complete executable for your operating system directly from the repository page.
+ Once you have rust installed and configured, run `cargo build --release` to generate a production build on `target/release/katetrade` or `cargo run` to execute locally in development mode.

### Docker
+ You must have `Docker` installed in your system
+ Inside the project run `docker build -t katetrade.` this will build the rust application inside the container image

## Usage
Katetrade is a CLI application where you can **live trade**, **backtest** and **validate trading strategies**.

+ If you have the `katetrade` binary just run `katetrade --help` to learn more,
+ if you just now downloaded the source code you can execute by running `cargo run -- --help`
+ If you built the docker container `docker run katetrade --help` 

```bash
Usage: katetrade <COMMAND>

Commands:
  run         Start trading one or more trading strategies in the real capital markets
  order       Operations related to orders
  backtest    Backtest a trading strategy using historical data
  file        Various operations related to historical files
  connection  General connection information
  optimize    Optimize/train a group for best solutions for a customized trading strategy
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
