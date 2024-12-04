## Visualization

Visualization script to plot the results of a backtest execution from `katetrade`. 
You will be able to see candlesticks and indication of when each order is sent into the market.

## Usage
1) Build the container
```bash
docker build -t trading-visualization .
```

2) Run the visualization script to generate the `html` containing the results
```
mkdir output
docker run --rm \
  -v /path/to/your/candlestick.csv:/app/candlestick.csv:ro \
  -v /path/to/your/trades.csv:/app/trades.csv:ro \
  -v $(pwd)/output:/app/output \
  trading-visualization \
  python visualize.py /app/candlestick.csv /app/trades.csv /app/output/output.html
```

if you would like to run using the test files, assuming you are inside the `/visualization` folder, run the following:
```bash
docker run --rm \
-v $(pwd)/candlestick.csv:/app/candlestick.csv:ro \
-v $(pwd)/trades.csv:/app/trades.csv:ro \
-v $(pwd)/output:/app/output \
trading-visualization /app/candlestick.csv /app/trades.csv /app/output/trade-results.html
```


3) Open the `html` file generate in the `output` folder that exists in the current directory.
```bash
# Mac
open output/trade-results.html
# Linux
xdg-open output/trade-results.html
```


