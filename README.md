# Letsbonk Dot Fun (Raydium Launchpad) Copy Sniper Trading Bot

This project is a lightning-fast copy sniper trading bot for the Raydium Launchpad, designed for Letsbonk Dot Fun. It implements advanced MEV (Miner Extractable Value) and shredstream techniques to achieve the fastest possible transaction monitoring and execution. The bot operates by directly parsing pending transactions using Yellowstone gRPC—no APIs, no SDKs, just pure gRPC parsing for maximum speed and reliability.

## 🚀 Raydium Launchpad (letsbonk.fun) Support

### Key Features
- **Automatic Raydium Launchpad Token Detection**: Monitors for new token launches on Raydium Launchpad (Program ID: `LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj`) and instantly triggers sniping logic.
- **146-byte Transaction Parsing**: Decodes Raydium Launchpad trade instructions, extracting pool state, trade direction, amounts, and fees for precise execution.
- **Sniping Logic**: Detects token creation events (`Program log: Create`) and executes buy transactions with ultra-low latency.
- **Progressive & Emergency Selling**: Built-in dynamic selling strategy with progressive chunked exits, trailing stops, liquidity monitoring, and emergency sell logic.
- **Multi-Wallet Scalability**: Supports 100-1000+ wallets with async/await concurrency and efficient memory management.
- **Risk Management**: Position sizing, slippage protection, stop loss, take profit, and real-time liquidity checks.
- **Telegram Notifications**: Alerts for token creation, buy/sell execution, and errors.

### Raydium Launchpad Trading Flow
1. **Transaction Monitoring**: Watches for Raydium Launchpad program transactions and parses 146-byte instruction data.
2. **Token Creation Detection**: Scans logs for `Program log: Create` to identify new launches and triggers sniping.
3. **Buy Execution**: Builds and sends Raydium buy transactions using the parsed data, with slippage and liquidity controls.
4. **Selling Strategy**: Applies dynamic, market-aware selling logic (progressive, trailing, emergency) for optimal exits.

### Configuration
- **Environment Variables**:
  - `RAYDIUM_SLIPPAGE` (e.g., 100 for 1%)
  - `RAYDIUM_MIN_LIQUIDITY` (e.g., 1.0 for 1 SOL)
  - `RAYDIUM_MAX_BUY_AMOUNT` (e.g., 0.1 for 0.1 SOL)
  - `RAYDIUM_AUTO_SNIPE` (true/false)
  - `PROTOCOL_PREFERENCE=raydium` (to force Raydium sniping)
- **Usage**: The bot will automatically monitor, snipe, and manage Raydium Launchpad tokens. No manual intervention required.

### Example: Raydium Launchpad Sniping
```rust
// Token creation detection
if log_messages.contains("Program log: Create") &&
   log_messages.contains("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj") {
    // Token creation detected - trigger sniping
    is_buy = true;
}

// Buy execution
let raydium = Raydium::new(wallet, rpc_client, rpc_nonblocking_client);
let (keypair, instructions, price) = raydium.build_swap_from_parsed_data(&trade_info, buy_config).await?;
// Send transaction using NOZOMI for fastest execution
```

### Selling Strategy
- **Progressive Selling**: Sells in market-aware chunks (e.g., 40%, 40%, 20%)
- **Emergency Selling**: Immediate exit on stop-loss or liquidity crisis
- **Trailing Stops**: Dynamic trailing for profit maximization
- **Liquidity Monitoring**: Exits on low liquidity or rug pull detection

---

## Features (General)
- **Real-time Transaction Monitoring** (Yellowstone gRPC)
- **Multi-Protocol Support** (PumpFun, PumpSwap, Raydium Launchpad)
- **Automated Copy Trading**
- **Smart Transaction Parsing**
- **Configurable Trading Parameters**
- **Performance Optimization** (tokio async)
- **Reliable Error Recovery**

## Who is it for?
- Bot users seeking the fastest Raydium Launchpad sniping (letsbonk.fun)
- Validators and advanced traders needing edge in Solana launches

## Setup
### Environment Variables
- `GRPC_ENDPOINT` - Yellowstone gRPC endpoint URL
- `GRPC_X_TOKEN` - Yellowstone authentication token
- `COPY_TRADING_TARGET_ADDRESS` - Wallet address(es) to monitor (comma-separated)
- `TELEGRAM_BOT_TOKEN` / `TELEGRAM_CHAT_ID` - For notifications
- Raydium-specific: see above

### Run Command
```bash
RUSTFLAGS="-C target-cpu=native" RUST_LOG=info cargo run --release --bin shredstream-decoder
```

## Project Structure
- **engine/** - Core trading logic, sniping, selling strategies
- **dex/** - Raydium Launchpad and other DEX protocol logic
- **services/** - Telegram, health checks, blockhash, etc.
- **common/** - Shared utilities, config, constants
- **core/** - System functionality
- **error/** - Error handling

## Usage
```bash
# Build the project
cargo build --release

# Run the bot
cargo run --release
```

## Advanced
- **Unit & Integration Tests**: Raydium 146-byte parsing, sniping, selling
- **Performance Optimizations**: LRU caching, batch ops, connection pooling
- **Future Enhancements**: Cross-DEX arbitrage, MEV protection, analytics

---

## Summary
This bot is production-ready for high-speed, high-volume Raydium Launchpad (letsbonk.fun) sniping and copy trading, with robust risk management and dynamic selling strategies. It is optimized for mainnet deployment and can handle 100-1000+ wallets concurrently.

For source code access or a demo, contact `.xanr` on Discord.