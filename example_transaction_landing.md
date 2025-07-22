# Simplified Transaction Landing System Usage

## Overview
The transaction landing system provides a streamlined solution for sending Solana transactions through Zeroslot and Nozomi services with health monitoring and automatic failover.

## Environment Variables

Add these to your `.env` file:

```env
# Transaction Landing Service Configuration
TRANSACTION_LANDING_SERVICE=0  # 0=Zeroslot, 1=Nozomi

# Service URLs
ZERO_SLOT_URL=https://api.zeroslot.io
NOZOMI_URL=https://nozomi.rpc.endpoint

# Tip Values
NOZOMI_TIP_VALUE=0.0015
ZEROSLOT_TIP_VALUE=0.001

# Health Check Configuration
ZERO_SLOT_HEALTH=true
```

## Basic Usage

### 1. Initialize the System

```rust
use crate::common::config::Config;
use crate::services::health_check;

// Initialize configuration
let config = Config::new().await;

// Initialize health check manager
health_check::initialize_health_check_manager().await?;
```

### 2. Send a Transaction

```rust
use crate::core::tx::{new_signed_and_send_nozomi, new_signed_and_send_zeroslot};
use crate::common::config::TransactionLandingMode;

// Get the configuration
let config_guard = Config::get().await;
let app_state = &config_guard.app_state;
let transaction_landing_mode = config_guard.transaction_landing_mode.clone();
drop(config_guard);

// Get recent blockhash
let recent_blockhash = match crate::services::blockhash_processor::BlockhashProcessor::get_latest_blockhash().await {
    Some(hash) => hash,
    None => return Err("Failed to get blockhash".to_string()),
};

// Send transaction using configured landing mode
match transaction_landing_mode {
    TransactionLandingMode::Zeroslot => {
        new_signed_and_send_zeroslot(
            app_state.zeroslot_rpc_client.clone(),
            recent_blockhash,
            &keypair,
            instructions,
            &logger,
        ).await
    },
    TransactionLandingMode::Nozomi => {
        new_signed_and_send_nozomi(
            app_state.nozomi_rpc_client.clone(),
            recent_blockhash,
            &keypair,
            instructions,
            &logger,
        ).await
    },
}
```

## Health Check Usage

### Check Service Health

```rust
use crate::services::health_check::HEALTH_CHECK_MANAGER;

// Check if a specific service is healthy
let is_zeroslot_healthy = HEALTH_CHECK_MANAGER.is_service_healthy("zeroslot").await;
let is_nozomi_healthy = HEALTH_CHECK_MANAGER.is_service_healthy("nozomi").await;

// Get the healthiest service
let healthiest = HEALTH_CHECK_MANAGER.get_healthiest_service(&TransactionLandingMode::Zeroslot).await;

// Get all service health statuses
let all_health = HEALTH_CHECK_MANAGER.get_all_service_health().await;
for (service, health) in all_health {
    println!("{}: {:?} ({}ms)", service, health.status, health.response_time.as_millis());
}
```

## Service-Specific Features

### ZeroSlot
- **Keepalive**: Automatic keepalive every 60 seconds to maintain connection (65-second timeout)
- **Tip**: Configurable tip value (upper limit: 0.1 SOL)
- **Health Check**: JSON-RPC `getHealth` method
- **Fast Execution**: Optimized for speed with MEV protection

### Nozomi
- **Fast RPC**: High-performance RPC endpoint
- **Tip Support**: Configurable tip value for prioritization
- **Health Check**: JSON-RPC `getHealth` method
- **Reliable**: Proven track record for transaction landing

## Error Handling

The system provides comprehensive error handling:

```rust
match new_signed_and_send_zeroslot(/* ... */).await {
    Ok(signatures) => {
        // Transaction successful
        println!("Signatures: {:?}", signatures);
    }
    Err(e) => {
        // Handle different error types
        if e.to_string().contains("service unhealthy") {
            // Try alternative service or retry
        } else if e.to_string().contains("insufficient funds") {
            // Handle funding issues
        } else {
            // General error handling
        }
    }
}
```

## Best Practices

1. **Use Health Checks**: Always check service health before sending critical transactions
2. **Configure Timeouts**: Set appropriate timeouts for your use case
3. **Monitor Performance**: Track response times and success rates
4. **Fallback Strategy**: Have a fallback plan when primary service fails
5. **Tip Management**: Set appropriate tip values based on network conditions
6. **ZeroSlot Keepalive**: The system automatically maintains ZeroSlot connection

## Integration Example

Here's how to integrate the simplified transaction landing system into your trading bot:

```rust
// In your main.rs or initialization code
pub async fn initialize_bot() -> Result<()> {
    // Initialize configuration
    let config = Config::new().await;
    
    // Start health check manager
    health_check::initialize_health_check_manager().await?;
    
    // Start blockhash processor
    let blockhash_processor = BlockhashProcessor::new(
        config.lock().await.app_state.rpc_client.clone()
    ).await?;
    blockhash_processor.start().await?;
    
    Ok(())
}

// In your trading logic
pub async fn execute_trade(
    instructions: Vec<Instruction>,
    keypair: &Keypair,
) -> Result<Vec<String>> {
    let config_guard = Config::get().await;
    let app_state = &config_guard.app_state;
    let transaction_landing_mode = config_guard.transaction_landing_mode.clone();
    drop(config_guard);
    
    let recent_blockhash = crate::services::blockhash_processor::BlockhashProcessor::get_latest_blockhash()
        .await
        .ok_or("Failed to get blockhash")?;
    
    let logger = Logger::new("[TRADE] => ".to_string());
    
    match transaction_landing_mode {
        TransactionLandingMode::Zeroslot => {
            new_signed_and_send_zeroslot(
                app_state.zeroslot_rpc_client.clone(),
                recent_blockhash,
                &keypair,
                instructions,
                &logger,
            ).await
        },
        TransactionLandingMode::Nozomi => {
            new_signed_and_send_nozomi(
                app_state.nozomi_rpc_client.clone(),
                recent_blockhash,
                &keypair,
                instructions,
                &logger,
            ).await
        },
    }
}
```

This simplified system focuses on the two most reliable and essential transaction landing services with comprehensive health monitoring and automatic ZeroSlot keepalive. 