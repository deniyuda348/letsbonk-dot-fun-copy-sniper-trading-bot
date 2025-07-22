# Raydium Launchpad Sniping Implementation

## Overview
This document describes the complete implementation of Raydium Launchpad token sniping functionality for the copy trading bot. The system can detect new token launches on Raydium Launchpad and automatically execute buy transactions.

## 🎯 Key Features Implemented

### 1. Token Creation Detection
- **Program ID**: `LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj`
- **Creation Signal**: `"Program log: Create"` in transaction logs
- **Buffer Size**: 146 bytes for Raydium Launchpad transactions
- **Auto-Detection**: Automatically sets `is_buy = true` when token creation is detected

### 2. Transaction Parsing (146-byte Buffer)
```rust
// Parsed fields from 146-byte buffer:
- pool_state: Pubkey (offset 16)
- total_base_sell: u64 (offset 48)
- virtual_base: u64 (offset 56)
- virtual_quote: u64 (offset 64)
- real_base_before: u64 (offset 72)
- real_quote_before: u64 (offset 80)
- real_base_after: u64 (offset 88)
- real_quote_after: u64 (offset 96)
- amount_in: u64 (offset 104)
- amount_out: u64 (offset 112)
- protocol_fee: u64 (offset 120)
- platform_fee: u64 (offset 128)
- share_fee: u64 (offset 136)
- trade_direction: u8 (offset 144) // 0=buy, 1=sell
- pool_status: u8 (offset 145)
```

### 3. Sniping Logic Flow

#### Step 1: Transaction Detection
```rust
// Monitor for Raydium Launchpad program transactions
if buffer.len() == 146 {
    // Parse 146-byte Raydium Launchpad transaction
    let parsed_data = parse_raydium_launchpad_transaction(buffer);
    
    // Check for token creation
    if log_messages.contains("Program log: Create") &&
       log_messages.contains("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj") {
        // Token creation detected - trigger sniping
        is_buy = true;
    }
}
```

#### Step 2: Buy Execution
```rust
SwapProtocol::RaydiumLaunchpad => {
    // Create Raydium instance
    let raydium = Raydium::new(wallet, rpc_client, rpc_nonblocking_client);
    
    // Build swap transaction
    let (keypair, instructions, price) = raydium
        .build_swap_from_parsed_data(&trade_info, buy_config).await?;
    
    // Execute with NOZOMI for fastest execution
    let signatures = new_signed_and_send_nozomi(
        nozomi_rpc_client,
        recent_blockhash,
        &keypair,
        instructions,
        &logger,
    ).await?;
}
```

#### Step 3: Selling Strategy Integration
```rust
// Initialize token for progressive selling
let selling_engine = SellingEngine::new(
    app_state,
    swap_config,
    SellingConfig::set_from_env(),
    true, // Enable progressive selling
);

// Calculate buy amount for Raydium
let buy_amount = (trade_info.amount.unwrap_or(0) as f64) / 1_000_000_000.0;

// Initialize selling strategy
selling_engine.initialize_token_for_selling(
    &trade_info.mint,
    price,
    buy_amount,
    &trade_info
).await?;
```

## 🚀 Implementation Details

### Enhanced Transaction Parser (`src/engine/transaction_parser.rs`)

#### Token Creation Detection Logic
```rust
// 🎯 SNIPING LOGIC: Check for token creation in log messages
let is_token_creation = if let Some(tx_inner) = &txn.transaction {
    if let Some(meta) = &tx_inner.meta {
        // Check log messages for "Program log: Create"
        meta.log_messages.iter().any(|log| {
            let contains_program_id = log.contains("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");
            let contains_create = log.contains("Program log: Create");
            
            if contains_program_id && contains_create {
                logger.log("🚀 TOKEN CREATION DETECTED!");
                true
            } else {
                false
            }
        })
    } else {
        false
    }
} else {
    false
};

// If this is a token creation, force is_buy = true for sniping
if is_token_creation {
    is_buy = true;
    logger.log("🎯 SNIPING MODE: Setting is_buy = true for token creation");
}
```

#### Comprehensive Logging
```rust
logger.log(format!(
    "📊 RAYDIUM PARSED DATA:\n\
     🏊 Pool: {}\n\
     💰 Amount In: {} | Amount Out: {}\n\
     📈 Virtual Reserves - Base: {} | Quote: {}\n\
     🔢 Real Reserves - Before: Base {} | Quote {}\n\
     🔢 Real Reserves - After: Base {} | Quote {}\n\
     💸 Fees - Protocol: {} | Platform: {} | Share: {}\n\
     🔄 Is Buy: {} | Token Creation: {}",
    pool_state, amount_in, amount_out,
    virtual_base, virtual_quote,
    real_base_before, real_quote_before,
    real_base_after, real_quote_after,
    protocol_fee, platform_fee, share_fee,
    is_buy, is_token_creation
));
```

### Copy Trading Integration (`src/engine/copy_trading.rs`)

#### Protocol Support Added
```rust
// Protocol string for notifications
let protocol_str = match protocol {
    SwapProtocol::PumpSwap => "PumpSwap",
    SwapProtocol::PumpFun => "PumpFun",
    SwapProtocol::RaydiumLaunchpad => "Raydium", // ✅ Added
    _ => "Unknown",
};

// Buy amount calculation
let buy_amount = match trade_info.dex_type {
    DexType::PumpSwap => (trade_info.base_amount_in.unwrap_or(0) as f64) / 1e9,
    DexType::PumpFun => (trade_info.token_amount.unwrap_or(0) as f64) / 1e9,
    DexType::RaydiumLaunchpad => (trade_info.amount.unwrap_or(0) as f64) / 1e9, // ✅ Added
    _ => trade_info.token_amount_f64,
};
```

#### Enhanced Buy Execution
```rust
SwapProtocol::RaydiumLaunchpad => {
    logger.log("🎯 RAYDIUM LAUNCHPAD SNIPING - Using Raydium protocol");
    
    let raydium = Raydium::new(wallet, rpc_client, rpc_nonblocking_client);
    
    match raydium.build_swap_from_parsed_data(&trade_info, buy_config).await {
        Ok((keypair, instructions, price)) => {
            logger.log("🚀 Generated Raydium buy instruction");
            
            // Use NOZOMI for fastest execution
            match new_signed_and_send_nozomi(
                nozomi_rpc_client, recent_blockhash, &keypair, instructions, &logger
            ).await {
                Ok(signatures) => {
                    logger.log("🎉 Raydium sniping transaction sent!");
                    
                    // Send notifications
                    send_copy_trade_notification(&trade_info, &signature, "Raydium", "RAYDIUM_SNIPED").await;
                    
                    // Initialize selling strategy
                    let selling_engine = SellingEngine::new(/* ... */);
                    selling_engine.initialize_token_for_selling(/* ... */).await;
                    
                    Ok(())
                },
                Err(e) => Err(format!("Raydium transaction error: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed to build Raydium instruction: {}", e)),
    }
},
```

### Selling Strategy Integration (`src/engine/selling_strategy.rs`)

#### Complete Protocol Support
- ✅ **Price Retrieval**: `get_current_price()` supports RaydiumLaunchpad
- ✅ **Liquidity Monitoring**: `update_metrics()` handles Raydium pools
- ✅ **Progressive Selling**: Full support for Raydium tokens
- ✅ **Emergency Selling**: NOZOMI integration for fast exits
- ✅ **Token Initialization**: Proper metrics setup for Raydium tokens

### Raydium DEX Module (`src/dex/raydium_launchpad.rs`)

#### Optimized for Sniping
- ✅ **Fast Pool Info Retrieval**: Efficient RPC calls
- ✅ **Transaction Building**: `build_swap_from_parsed_data()` integration
- ✅ **Price Calculation**: Constant product AMM formula
- ✅ **Slippage Protection**: Built-in slippage management

## 📊 Monitoring and Logging

### Real-time Detection Logs
```
🎯 PARSING RAYDIUM LAUNCHPAD TRANSACTION (146 bytes)
🚀 TOKEN CREATION DETECTED! Program log: Create found
📝 Creation log: Program LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj: Create
🎯 SNIPING MODE: Setting is_buy = true for token creation
📊 RAYDIUM PARSED DATA:
  🏊 Pool: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkJ
  💰 Amount In: 1000000000 | Amount Out: 500000000000
  📈 Virtual Reserves - Base: 1000000000000 | Quote: 30000000000
  🔄 Is Buy: true | Token Creation: true
```

### Execution Logs
```
🎯 RAYDIUM LAUNCHPAD SNIPING - Using Raydium Launchpad protocol for buy
🚀 Generated Raydium Launchpad buy instruction at price: 0.00000003
📋 Copy Raydium transaction: 5KJp8...xwZ2M
🔥 Using NOZOMI for Raydium Launchpad sniping >>>>>>>>>>
🎉 Raydium Launchpad buy transaction sent: 2uF7g...mN3K
✅ Raydium Launchpad buy transaction verified successfully
📝 Added Raydium token account ABC123...789XYZ to global list
🎯 Raydium token successfully initialized for progressive selling
```

## 🔧 Configuration

### Environment Variables
```bash
# Raydium-specific settings
RAYDIUM_SLIPPAGE=100              # 1% slippage for sniping
RAYDIUM_MIN_LIQUIDITY=1.0         # 1 SOL minimum liquidity
RAYDIUM_MAX_BUY_AMOUNT=0.1        # 0.1 SOL max buy per snipe
RAYDIUM_AUTO_SNIPE=true           # Enable automatic sniping

# Copy trading settings
PROTOCOL_PREFERENCE=raydium       # Use Raydium as preferred protocol
YELLOWSTONE_GRPC_HTTP=...         # Yellowstone gRPC endpoint
YELLOWSTONE_GRPC_TOKEN=...        # API token
```

### Target Addresses Setup
```rust
let config = CopyTradingConfig {
    target_addresses: vec![
        "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj".to_string(), // Raydium Launchpad
        // Add target wallets to copy
    ],
    protocol_preference: SwapProtocol::RaydiumLaunchpad,
    // ... other config
};
```

## 🎯 Usage Flow

### 1. Start Monitoring
```bash
# The bot automatically monitors for Raydium Launchpad transactions
# When a 146-byte transaction is detected, it's parsed for token creation
```

### 2. Token Creation Detection
```
📡 Monitoring transaction: 5KJp8...xwZ2M
🎯 Found 146-byte buffer - Raydium Launchpad transaction detected
🔍 Checking log messages for token creation...
🚀 TOKEN CREATION DETECTED! Program log: Create found
```

### 3. Automatic Sniping
```
🎯 SNIPING MODE ACTIVATED
🚀 Building Raydium buy transaction...
⚡ Executing with NOZOMI for fastest speed...
🎉 SNIPING SUCCESS! Transaction: 2uF7g...mN3K
📝 Token added to portfolio for selling strategy
```

### 4. Selling Strategy Activation
```
🎯 Token initialized for progressive selling
📊 Monitoring price movements...
💰 Progressive selling chunks: 25%, 25%, 25%, 25%
🛑 Stop loss: -30% | Take profit: +15%
```

## 🚀 Performance Optimizations

### 1. Fast Execution
- **NOZOMI Integration**: Fastest transaction execution
- **Real-time Blockhash**: Latest blockhash for immediate processing
- **Parallel Processing**: Concurrent transaction building and sending

### 2. Efficient Parsing
- **146-byte Buffer Detection**: Immediate Raydium transaction identification
- **Log Message Scanning**: Fast token creation detection
- **Minimal RPC Calls**: Optimized data retrieval

### 3. Multi-Wallet Support
- **100-1000 Wallets**: Scalable architecture
- **Concurrent Operations**: Parallel sniping across multiple tokens
- **Memory Optimization**: Efficient data structures

## ✅ Testing and Validation

### Unit Tests
- ✅ 146-byte buffer parsing
- ✅ Token creation detection
- ✅ Protocol matching
- ✅ Buy amount calculation

### Integration Tests
- ✅ End-to-end sniping flow
- ✅ Selling strategy integration
- ✅ Error handling and recovery

## 🎯 Summary

The Raydium Launchpad sniping implementation provides:

🎯 **Automatic Token Detection** - Monitors for new token launches  
⚡ **Ultra-Fast Execution** - NOZOMI integration for speed  
🛡️ **Risk Management** - Stop loss and take profit automation  
📊 **Progressive Selling** - Smart exit strategy  
🔧 **Multi-Wallet Support** - Scalable for 100-1000 wallets  
📱 **Real-time Notifications** - Telegram integration  
🚀 **Production Ready** - Complete error handling and logging  

The system is now ready for mainnet deployment and can handle high-volume Raydium Launchpad token sniping operations with comprehensive risk management and selling strategies. 