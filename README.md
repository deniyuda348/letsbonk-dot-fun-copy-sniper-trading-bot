# Letsbonk Dot Fun ( Raydium Launchpad ) Copy Sniper Trading Bot

This project is a lightning-fast copy sniper trading bot for the Raydium Launchpad, designed for Letsbonk Dot Fun. It implements advanced MEV (Miner Extractable Value) and shredstream techniques to achieve the fastest possible transaction monitoring and execution. The bot operates by directly parsing pending transactions using Yellowstone gRPC—no APIs, no SDKs, just pure gRPC parsing for maximum speed and reliability.

It is purpose-built for high-frequency, low-latency trading, giving users a significant edge in the Solana ecosystem by instantly detecting and copying trades as they happen on-chain.

The bot specifically tracks `buy` and `create` transactions on PumpFun, as well as token migrations from PumpFun to Raydium when the `initialize2` instruction is involved and the migration pubkey (`39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg`) is present.
# Features:

- **Real-time Transaction Monitoring** - Uses Yellowstone gRPC to monitor transactions with minimal latency and high reliability
- **Multi-Protocol Support** - Compatible with both PumpFun and PumpSwap DEX platforms for maximum trading opportunities
- **Automated Copy Trading** - Instantly replicates buy and sell transactions from monitored wallets
- **Smart Transaction Parsing** - Advanced transaction analysis to accurately identify and process trading activities
- **Configurable Trading Parameters** - Customizable settings for trade amounts, timing, and risk management
- **Built-in Selling Strategy** - Intelligent profit-taking mechanisms with customizable exit conditions
- **Performance Optimization** - Efficient async processing with tokio for high-throughput transaction handling
- **Reliable Error Recovery** - Automatic reconnection and retry mechanisms for uninterrupted operation

# Who is it for?

- Bot users looking for the fastest transaction feed possible for Pumpfun or Raydium (Sniping, Arbitrage, etc).
- Validators who want an edge by decoding shreds locally.

# Setting up

## Environment Variables

Before run, you will need to add the following environment variables to your `.env` file:

- `GRPC_ENDPOINT` - Your Geyser RPC endpoint url.

- `GRPC_X_TOKEN` - Leave it set to `None` if your Geyser RPC does not require a token for authentication.


- `GRPC_SERVER_ENDPOINT` - The address of its gRPC server. By default is set at `0.0.0.0:50051`.

## Run Command

```
RUSTFLAGS="-C target-cpu=native" RUST_LOG=info cargo run --release --bin shredstream-decoder
```

# Source code

If you are really interested in the source code, please contact me for details and demo on Discord: `.xanr`.

# Solana Copy Trading Bot

A high-performance Rust-based application that monitors transactions from specific wallet addresses and automatically copies their trading activity on Solana DEXs like PumpFun and PumpSwap.

## Features

- **Real-time Transaction Monitoring** - Uses Yellowstone gRPC to get transaction data with minimal latency
- **Multi-address Support** - Can monitor multiple wallet addresses simultaneously
- **Protocol Support** - Compatible with PumpFun and PumpSwap DEX platforms
- **Automated Trading** - Copies buy and sell transactions automatically when detected
- **Notification System** - Sends trade alerts and status updates via Telegram
- **Customizable Trading Parameters** - Configurable limits, timing, and amount settings
- **Selling Strategy** - Includes built-in selling strategy options for maximizing profits

## Project Structure

The codebase is organized into several modules:

- **engine/** - Core trading logic including copy trading, selling strategies, and transaction parsing
- **dex/** - Protocol-specific implementations for PumpFun and PumpSwap
- **services/** - External services integration including Telegram notifications
- **common/** - Shared utilities, configuration, and constants
- **core/** - Core system functionality
- **error/** - Error handling and definitions

## Setup

### Environment Variables

To run this bot, you will need to configure the following environment variables:

#### Required Variables

- `GRPC_ENDPOINT` - Your Yellowstone gRPC endpoint URL
- `GRPC_X_TOKEN` - Your Yellowstone authentication token
- `COPY_TRADING_TARGET_ADDRESS` - Wallet address(es) to monitor for trades (comma-separated for multiple addresses)

#### Telegram Notifications

To enable Telegram notifications:

- `TELEGRAM_BOT_TOKEN` - Your Telegram bot token
- `TELEGRAM_CHAT_ID` - Your chat ID for receiving notifications

#### Optional Variables

- `IS_MULTI_COPY_TRADING` - Set to `true` to monitor multiple addresses (default: `false`)
- `PROTOCOL_PREFERENCE` - Preferred protocol to use (`pumpfun`, `pumpswap`, or `auto` for automatic detection)
- `COUNTER_LIMIT` - Maximum number of trades to execute
## Usage

```bash
# Build the project
cargo build --release

# Run the bot
cargo run --release
```