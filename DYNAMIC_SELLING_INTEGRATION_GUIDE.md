# Dynamic Selling Strategy Integration Guide

## Overview
This guide explains how to integrate the new **Dynamic Selling Strategy** that fixes all the issues in your current progressive selling logic and makes it profitable.

## Key Improvements

### 🔥 **Fixed Issues**
1. **Real-time Price Updates**: Continuous price monitoring with market condition detection
2. **Dynamic Timing**: Adaptive intervals based on market volatility and liquidity
3. **Smart Chunk Sizing**: Market-aware chunk sizes that maximize profits
4. **Liquidity Monitoring**: Real-time liquidity tracking with rug pull detection
5. **Market Condition Awareness**: Bull/bear market detection with strategy adjustment
6. **Profit Maximization**: Momentum-based selling with advanced risk management

### 🎯 **New Features**
- **6 Market Conditions**: BullRun, BullTrend, Sideways, BearTrend, BearDump, HighVolatility
- **Dynamic Profit Targets**: 5% to 50% based on market conditions
- **Smart Stop Losses**: -5% to -20% based on risk levels
- **Trailing Stops**: Adaptive trailing with tight/loose modes
- **Progressive Selling**: 3-stage selling with market-aware chunk sizes
- **Emergency Modes**: Liquidity crisis and massive loss protection

## Integration Steps

### 1. **Replace Old Selling Logic**

In your `copy_trading.rs`, replace the old selling initialization:

```rust
// OLD CODE - Remove this:
// let selling_config = SellingConfig::set_from_env();
// let selling_engine = SellingEngine::new(...);

// NEW CODE - Add this:
use crate::engine::dynamic_selling_strategy::{DynamicSellingEngine, DynamicSellingConfig};

// Initialize the new dynamic selling engine
let dynamic_config = DynamicSellingConfig {
    base_profit_target: 15.0,           // 15% base profit
    minimum_profit_target: 5.0,         // 5% minimum profit
    maximum_profit_target: 50.0,        // 50% maximum profit
    base_stop_loss: -10.0,              // -10% base stop loss
    tight_stop_loss: -5.0,              // -5% tight stop loss
    loose_stop_loss: -20.0,             // -20% loose stop loss
    trailing_activation_threshold: 10.0, // 10% activation
    trailing_distance: 5.0,             // 5% trailing distance
    min_liquidity_threshold: 10.0,      // 10 SOL minimum liquidity
    emergency_liquidity_threshold: 2.0, // 2 SOL emergency threshold
    max_hold_time_seconds: 1800,        // 30 minutes max hold
    min_hold_time_seconds: 60,          // 1 minute min hold
    profit_lock_time_seconds: 300,      // 5 minutes profit lock
    use_progressive_selling: true,
    max_progressive_attempts: 3,
    progressive_chunk_sizes: vec![0.4, 0.4, 0.2], // 40%, 40%, 20%
    progressive_interval_seconds: 30,    // 30 seconds base interval
    bull_run_profit_multiplier: 1.5,    // 1.5x profit in bull runs
    bear_dump_stop_multiplier: 0.5,     // 0.5x stop loss in dumps
    high_volatility_speed_multiplier: 0.5, // 0.5x interval in volatility
    ..Default::default()
};

let mut dynamic_selling_engine = DynamicSellingEngine::new(dynamic_config);
```

### 2. **Track Token Purchases**

When buying tokens, initialize tracking:

```rust
// In your buy execution logic:
pub async fn execute_buy(
    trade_info: TradeInfoFromToken,
    // ... other parameters
) -> Result<(), String> {
    // ... existing buy logic ...
    
    // After successful buy, track the token
    dynamic_selling_engine.track_token_purchase(
        trade_info.mint.clone(),
        entry_price,
        token_amount
    );
    
    Ok(())
}
```

### 3. **Real-time Price Updates**

Add price monitoring (replace your existing price update logic):

```rust
// In your monitoring loop:
pub async fn monitor_token_prices(
    dynamic_selling_engine: &mut DynamicSellingEngine,
    token_mint: &str,
) -> Result<(), String> {
    loop {
        // Get current price from your price source
        let current_price = get_current_price(token_mint).await?;
        let volume = get_current_volume(token_mint).await.ok();
        let liquidity = get_current_liquidity(token_mint).await.ok();
        
        // Update the dynamic engine
        dynamic_selling_engine.update_token_price(
            token_mint,
            current_price,
            volume,
            liquidity
        );
        
        // Check for sell conditions
        match dynamic_selling_engine.evaluate_sell_decision(token_mint) {
            Ok(decision) if decision.should_sell => {
                println!("🚨 SELL DECISION: {}", decision.sell_reason);
                
                // Execute the sell
                execute_dynamic_sell(token_mint, decision, &dynamic_selling_engine).await?;
            },
            Ok(_) => {
                // No sell needed, continue monitoring
            },
            Err(e) => {
                eprintln!("❌ Error evaluating sell decision: {}", e);
            }
        }
        
        // Wait before next check (dynamic interval)
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
```

### 4. **Execute Dynamic Selling**

Replace your progressive selling logic:

```rust
async fn execute_dynamic_sell(
    token_mint: &str,
    decision: SellDecision,
    dynamic_selling_engine: &mut DynamicSellingEngine,
) -> Result<(), String> {
    match decision.urgency_level {
        SellUrgency::Critical => {
            // Emergency sell - use high slippage, sell all immediately
            println!("🚨 EMERGENCY SELL: {}", decision.sell_reason);
            execute_emergency_sell(token_mint, decision.recommended_slippage).await?;
        },
        SellUrgency::High => {
            // High urgency - sell all but with lower slippage
            println!("⚠️ HIGH URGENCY SELL: {}", decision.sell_reason);
            execute_full_sell(token_mint, decision.recommended_slippage).await?;
        },
        SellUrgency::Medium | SellUrgency::Low => {
            if decision.use_progressive {
                // Use progressive selling
                println!("🔄 PROGRESSIVE SELL: {}", decision.sell_reason);
                dynamic_selling_engine.execute_progressive_sell(
                    token_mint,
                    &create_trade_info_from_metrics(token_mint)?,
                    SwapProtocol::Auto
                ).await?;
            } else {
                // Sell the specified percentage
                println!("💰 PARTIAL SELL: {}", decision.sell_reason);
                execute_partial_sell(token_mint, decision.sell_percentage, decision.recommended_slippage).await?;
            }
        }
    }
    
    // Remove token from tracking if fully sold
    if decision.sell_percentage >= 1.0 {
        dynamic_selling_engine.remove_token(token_mint);
    }
    
    Ok(())
}
```

### 5. **Environment Configuration**

Add these environment variables to your `.env` file:

```env
# Dynamic Selling Configuration
DYNAMIC_SELLING_BASE_PROFIT_TARGET=15.0
DYNAMIC_SELLING_MIN_PROFIT_TARGET=5.0
DYNAMIC_SELLING_MAX_PROFIT_TARGET=50.0
DYNAMIC_SELLING_BASE_STOP_LOSS=-10.0
DYNAMIC_SELLING_TIGHT_STOP_LOSS=-5.0
DYNAMIC_SELLING_LOOSE_STOP_LOSS=-20.0
DYNAMIC_SELLING_TRAILING_ACTIVATION=10.0
DYNAMIC_SELLING_TRAILING_DISTANCE=5.0
DYNAMIC_SELLING_MIN_LIQUIDITY=10.0
DYNAMIC_SELLING_EMERGENCY_LIQUIDITY=2.0
DYNAMIC_SELLING_MAX_HOLD_TIME=1800
DYNAMIC_SELLING_MIN_HOLD_TIME=60
DYNAMIC_SELLING_PROFIT_LOCK_TIME=300
DYNAMIC_SELLING_USE_PROGRESSIVE=true
DYNAMIC_SELLING_MAX_ATTEMPTS=3
DYNAMIC_SELLING_CHUNK_SIZES=0.4,0.4,0.2
DYNAMIC_SELLING_INTERVAL=30
DYNAMIC_SELLING_BULL_MULTIPLIER=1.5
DYNAMIC_SELLING_BEAR_MULTIPLIER=0.5
DYNAMIC_SELLING_VOLATILITY_MULTIPLIER=0.5
```

### 6. **Portfolio Monitoring**

Add portfolio tracking:

```rust
// In your main monitoring loop:
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        // Log portfolio summary
        let summary = dynamic_selling_engine.get_portfolio_summary();
        println!("{}", summary);
        
        // Check for any tokens that need attention
        for token_mint in dynamic_selling_engine.get_tracked_tokens() {
            if let Ok(decision) = dynamic_selling_engine.evaluate_sell_decision(&token_mint) {
                if decision.should_sell {
                    println!("⚠️ {} needs attention: {}", token_mint, decision.sell_reason);
                }
            }
        }
    }
});
```

## Usage Examples

### Example 1: Bull Market Configuration
```rust
let bull_market_config = DynamicSellingConfig {
    base_profit_target: 25.0,           // Higher profits in bull markets
    bull_run_profit_multiplier: 2.0,    // 2x multiplier
    trailing_activation_threshold: 15.0, // Activate trailing at 15%
    trailing_distance: 3.0,             // Tight trailing at 3%
    progressive_chunk_sizes: vec![0.3, 0.3, 0.4], // Keep more for longer
    ..Default::default()
};
```

### Example 2: Bear Market Configuration
```rust
let bear_market_config = DynamicSellingConfig {
    base_profit_target: 8.0,            // Lower targets in bear markets
    base_stop_loss: -8.0,               // Tighter stop losses
    bear_dump_stop_multiplier: 0.3,     // Very tight stops in dumps
    progressive_chunk_sizes: vec![0.6, 0.3, 0.1], // Sell more upfront
    ..Default::default()
};
```

### Example 3: High Volatility Configuration
```rust
let volatility_config = DynamicSellingConfig {
    base_profit_target: 12.0,           // Medium targets
    trailing_distance: 8.0,             // Wider trailing stops
    high_volatility_speed_multiplier: 0.3, // Much faster selling
    progressive_interval_seconds: 15,    // Faster intervals
    ..Default::default()
};
```

## Performance Monitoring

### Key Metrics to Track
```rust
// Track these metrics for optimization:
struct PerformanceMetrics {
    total_trades: usize,
    winning_trades: usize,
    losing_trades: usize,
    average_profit_per_trade: f64,
    average_hold_time: f64,
    max_drawdown: f64,
    sharpe_ratio: f64,
    profit_factor: f64,
}
```

### Optimization Tips
1. **Monitor Win Rate**: Aim for 60%+ win rate
2. **Track Average Profit**: Should be 2x average loss
3. **Analyze Hold Times**: Optimize based on performance
4. **Adjust for Market Conditions**: Use different configs for different markets
5. **Backtest Changes**: Test on historical data before going live

## Common Issues and Solutions

### Issue 1: Too Many Small Profits
**Solution**: Increase `base_profit_target` and `minimum_profit_target`

### Issue 2: Large Losses
**Solution**: Tighten `base_stop_loss` and enable `tight_stop_loss`

### Issue 3: Missing Big Moves
**Solution**: Increase `bull_run_profit_multiplier` and adjust `trailing_distance`

### Issue 4: Too Slow in Volatile Markets
**Solution**: Decrease `high_volatility_speed_multiplier` and `progressive_interval_seconds`

### Issue 5: Liquidity Issues
**Solution**: Increase `min_liquidity_threshold` and `emergency_liquidity_threshold`

## Next Steps

1. **Implement the Integration**: Follow the steps above
2. **Test with Small Amounts**: Start with small position sizes
3. **Monitor Performance**: Track all metrics
4. **Optimize Parameters**: Adjust based on results
5. **Scale Up**: Increase position sizes as performance improves

## Advanced Features

### Custom Market Condition Detection
```rust
// You can extend the market condition detection:
impl MarketCondition {
    pub fn detect_from_indicators(
        rsi: f64,
        volume_ratio: f64,
        price_change_24h: f64,
    ) -> Self {
        match (rsi, volume_ratio, price_change_24h) {
            (rsi, vol, change) if rsi > 70.0 && vol > 2.0 && change > 20.0 => MarketCondition::BullRun,
            (rsi, vol, change) if rsi < 30.0 && vol > 2.0 && change < -20.0 => MarketCondition::BearDump,
            (_, vol, _) if vol > 3.0 => MarketCondition::HighVolatility,
            (rsi, _, change) if rsi > 55.0 && change > 5.0 => MarketCondition::BullTrend,
            (rsi, _, change) if rsi < 45.0 && change < -5.0 => MarketCondition::BearTrend,
            _ => MarketCondition::Sideways,
        }
    }
}
```

### Risk Management Integration
```rust
// Add position sizing based on portfolio risk:
impl DynamicSellingEngine {
    pub fn calculate_position_risk(&self, token_mint: &str) -> f64 {
        let portfolio_value = self.get_total_portfolio_value();
        let position_value = self.get_position_value(token_mint);
        position_value / portfolio_value
    }
    
    pub fn adjust_sell_percentage_for_risk(&self, base_percentage: f64, risk_level: f64) -> f64 {
        if risk_level > 0.1 {
            // If position is more than 10% of portfolio, sell more aggressively
            base_percentage * 1.5
        } else {
            base_percentage
        }
    }
}
```

This new dynamic selling strategy should significantly improve your bot's profitability by:
- Capturing more profits in bull markets
- Limiting losses in bear markets  
- Adapting to changing market conditions
- Providing better risk management
- Optimizing timing and position sizing

**Test thoroughly before going live with large amounts!** 