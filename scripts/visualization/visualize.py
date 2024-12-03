import argparse
import pandas as pd
import plotly.graph_objects as go

def main(candlestick_file, trades_file, output_file):
    # Read candlestick data
    df_candles = pd.read_csv(candlestick_file, parse_dates=['time'])
    df_candles.sort_values('time', inplace=True)

    # Read trades data
    df_trades = pd.read_csv(trades_file, parse_dates=['time'])
    df_trades.sort_values('time', inplace=True)

    # Calculate cumulative positions
    df_trades['quantity'] = df_trades['quantity'].astype(float)
    df_trades['signed_quantity'] = df_trades.apply(
        lambda row: row['quantity'] if row['action'] == 'entry' else -row['quantity'], axis=1
    )
    df_trades['position_size'] = df_trades['signed_quantity'].cumsum()

    # Identify position changes
    position_changes = []
    current_position = None

    for idx, row in df_trades.iterrows():
        position_size = row['position_size']
        action = row['action']
        time = row['time']
        price = row['price']
        position_type = row['position']

        if current_position is None and position_size != 0:
            current_position = {
                'start_time': time,
                'start_price': price,
                'position_type': position_type,
                'entries': [(time, price, row['quantity'])],
                'exits': []
            }
        elif current_position is not None:
            if action == 'entry':
                current_position['entries'].append((time, price, row['quantity']))
            elif action == 'exit':
                current_position['exits'].append((time, price, row['quantity']))
                if position_size == 0:
                    current_position['end_time'] = time
                    current_position['end_price'] = price
                    # Calculate profit or loss
                    entry_prices = [e[1] for e in current_position['entries']]
                    exit_prices = [e[1] for e in current_position['exits']]
                    total_entry_qty = sum([e[2] for e in current_position['entries']])
                    total_exit_qty = sum([e[2] for e in current_position['exits']])

                    avg_entry_price = sum([e[1]*e[2] for e in current_position['entries']]) / total_entry_qty
                    avg_exit_price = sum([e[1]*e[2] for e in current_position['exits']]) / total_exit_qty

                    if current_position['position_type'] == 'long':
                        profit = avg_exit_price - avg_entry_price
                    else:
                        profit = avg_entry_price - avg_exit_price

                    current_position['profit'] = profit
                    position_changes.append(current_position)
                    current_position = None

    # Create candlestick chart with custom colors
    fig = go.Figure()

    # Define custom candlestick colors
    increasing_color = 'white'  # Long (increasing) candlesticks
    decreasing_color = 'red'    # Short (decreasing) candlesticks

    fig.add_trace(go.Candlestick(
        x=df_candles['time'],
        open=df_candles['open'],
        high=df_candles['high'],
        low=df_candles['low'],
        close=df_candles['close'],
        name='Candlesticks',
        increasing_line_color=increasing_color,
        decreasing_line_color=decreasing_color,
        # increasing_fillcolor=increasing_color,
        # decreasing_fillcolor=decreasing_color
    ))

    # Add background shading for profit/loss periods
    shapes = []

    for position in position_changes:
        start_time = position['start_time']
        end_time = position.get('end_time', df_candles['time'].iloc[-1])
        profit = position.get('profit', 0)
        position_type = position['position_type']

        # Determine color based on profit/loss
        if profit > 0:
            fill_color = 'rgba(0, 255, 0, 0.4)'  # Greenish for profit
        else:
            fill_color = 'rgba(255, 0, 0, 0.4)'  # Reddish for loss

        # Add a rectangle shape covering the position duration
        shapes.append({
            'type': 'rect',
            'xref': 'x',
            'yref': 'paper',
            'x0': start_time,
            'x1': end_time,
            'y0': 0,
            'y1': 1,
            'fillcolor': fill_color,
            'opacity': 0.3,
            'layer': 'below',
            'line_width': 0,
        })

        # Plot position duration lines
        color = 'green' if position_type == 'long' else 'magenta'

        fig.add_trace(go.Scatter(
            x=[start_time, end_time],
            y=[position['start_price'], position.get('end_price', position['start_price'])],
            mode='lines',
            line=dict(color=color, width=2, dash='solid'),
            name=f'{position_type.capitalize()} Position',
            showlegend=False
        ))

        # Plot entry markers
        for entry_time, entry_price, qty in position['entries']:
            fig.add_trace(go.Scatter(
                x=[entry_time],
                y=[entry_price],
                mode='markers',
                marker=dict(color=color, symbol='circle', size=10, line=dict(width=1, color='white')),
                name=f'{position_type.capitalize()} Entry',
                hovertemplate=f'<b>{position_type.capitalize()} Entry</b><br>Time: {entry_time}<br>Price: {entry_price}<br>Quantity: {qty}<extra></extra>',
                showlegend=False
            ))

        # Plot exit markers
        for exit_time, exit_price, qty in position['exits']:
            fig.add_trace(go.Scatter(
                x=[exit_time],
                y=[exit_price],
                mode='markers',
                marker=dict(color=color, symbol='x', size=10, line=dict(width=1, color='white')),
                name=f'{position_type.capitalize()} Exit',
                hovertemplate=f'<b>{position_type.capitalize()} Exit</b><br>Time: {exit_time}<br>Price: {exit_price}<br>Quantity: {qty}<extra></extra>',
                showlegend=False
            ))

    # Update layout for dark mode
    fig.update_layout(
        title='Trading Visualization',
        xaxis_title='Time',
        yaxis_title='Price',
        xaxis_rangeslider_visible=False,
        template='plotly_dark',  # Dark mode template
        hovermode='x unified',
        plot_bgcolor='rgba(50, 50, 50, 1)',
        paper_bgcolor='rgba(50, 50, 50, 1)',
        font=dict(color='white'),
        shapes=shapes  # Add the shapes to the layout
    )

    # Save the figure as an HTML file
    fig.write_html(output_file)
    print(f"Visualization saved to {output_file}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Visualize trading data with profit/loss background shading.')
    parser.add_argument('candlestick_file', help='Path to the candlestick CSV file.')
    parser.add_argument('trades_file', help='Path to the trades CSV file.')
    parser.add_argument('output_file', help='Path for the output HTML file.')
    args = parser.parse_args()

    main(args.candlestick_file, args.trades_file, args.output_file)
