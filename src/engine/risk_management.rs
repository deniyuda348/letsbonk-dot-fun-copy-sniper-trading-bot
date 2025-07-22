use std::collections::HashMap;
use std::time::Instant;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use colored::Colorize;
use crate::library::logger::Logger;
// Removed dynamic_selling_strategy imports

/// Simple market condition enum
#[derive(Debug, Clone, PartialEq)]
pub enum MarketCondition {
    Normal,
    BearDump,
    BullRun,
}

/// Simple token metrics for risk management
#[derive(Debug, Clone)]
pub struct RealTimeTokenMetrics {
    pub cost_basis: f64,
    pub unrealized_pnl_usd: f64,
    pub volatility_score: f64,
    pub market_condition: MarketCondition,
}

impl Default for RealTimeTokenMetrics {
    fn default() -> Self {
        Self {
            cost_basis: 0.0,
            unrealized_pnl_usd: 0.0,
            volatility_score: 0.0,
            market_condition: MarketCondition::Normal,
        }
    }
}

/// Portfolio risk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagementConfig {
    pub max_position_size_percentage: f64,        // 20.0 = 20% of portfolio
    pub max_total_exposure_percentage: f64,       // 80.0 = 80% of portfolio
    pub max_daily_loss_percentage: f64,           // -5.0 = -5% daily loss limit
    pub max_drawdown_percentage: f64,             // -25.0 = -25% max drawdown
    pub high_volatility_threshold: f64,           // 30.0 = 30% volatility
    pub volatility_position_reducer: f64,         // 0.5 = 50% reduction in high volatility
    pub max_positions_per_hour: usize,            // 10 positions per hour
    pub emergency_stop_loss_percentage: f64,     // -30.0 = -30% emergency stop
}

impl Default for RiskManagementConfig {
    fn default() -> Self {
        Self {
            max_position_size_percentage: 20.0,
            max_total_exposure_percentage: 80.0,
            max_daily_loss_percentage: -5.0,
            max_drawdown_percentage: -25.0,
            high_volatility_threshold: 30.0,
            volatility_position_reducer: 0.5,
            max_positions_per_hour: 10,
            emergency_stop_loss_percentage: -30.0,
        }
    }
}

/// Portfolio risk metrics
#[derive(Debug, Clone)]
pub struct PortfolioRiskMetrics {
    pub total_portfolio_value: f64,
    pub total_exposure: f64,
    pub current_drawdown: f64,
    pub daily_pnl: f64,
    pub portfolio_volatility: f64,
    pub liquidity_risk_score: f64,
}

/// Trade performance tracking
#[derive(Debug, Clone)]
pub struct TradePerformance {
    pub consecutive_wins: usize,
    pub consecutive_losses: usize,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub last_trade_time: Instant,
    pub hourly_trade_count: usize,
}

/// Risk management engine
pub struct RiskManagementEngine {
    config: RiskManagementConfig,
    portfolio_metrics: PortfolioRiskMetrics,
    trade_performance: TradePerformance,
    logger: Logger,
}

impl RiskManagementEngine {
    pub fn new(config: RiskManagementConfig) -> Self {
        Self {
            config,
            portfolio_metrics: PortfolioRiskMetrics {
                total_portfolio_value: 0.0,
                total_exposure: 0.0,
                current_drawdown: 0.0,
                daily_pnl: 0.0,
                portfolio_volatility: 0.0,
                liquidity_risk_score: 0.0,
            },
            trade_performance: TradePerformance {
                consecutive_wins: 0,
                consecutive_losses: 0,
                win_rate: 0.0,
                profit_factor: 1.0,
                last_trade_time: Instant::now(),
                hourly_trade_count: 0,
            },
            logger: Logger::new("[RISK-MGMT] => ".red().to_string()),
        }
    }
    
    /// Calculate optimal position size for a new trade
    pub fn calculate_position_size(
        &mut self,
        _token_mint: &str,
        portfolio_value: f64,
        token_metrics: &RealTimeTokenMetrics,
    ) -> Result<f64> {
        self.logger.log(format!(
            "🎯 Calculating position size | Portfolio: ${:.2}",
            portfolio_value
        ).blue().to_string());
        
        // Base position size
        let base_size = portfolio_value * (self.config.max_position_size_percentage / 100.0);
        
        // Adjust for volatility
        let volatility_adjustment = if token_metrics.volatility_score > self.config.high_volatility_threshold {
            self.config.volatility_position_reducer
        } else {
            1.0
        };
        
        // Adjust for performance
        let performance_adjustment = if self.trade_performance.consecutive_losses >= 3 {
            0.5  // Reduce size after losses
        } else if self.trade_performance.consecutive_wins >= 5 {
            1.2  // Increase size after wins
        } else {
            1.0
        };
        
        // Calculate final position size
        let final_size = base_size * volatility_adjustment * performance_adjustment;
        
        self.logger.log(format!(
            "📊 Position size: Base ${:.2} -> Final ${:.2} (Vol: {:.2}x, Perf: {:.2}x)",
            base_size, final_size, volatility_adjustment, performance_adjustment
        ).cyan().to_string());
        
        Ok(final_size.max(0.0))
    }
    
    /// Check if we should allow a new position
    pub fn should_allow_new_position(&mut self, token_metrics: &RealTimeTokenMetrics) -> Result<bool> {
        // Check hourly trade limit
        if self.trade_performance.hourly_trade_count >= self.config.max_positions_per_hour {
            self.logger.log(format!(
                "🚫 BLOCKED: Hourly trade limit reached ({}/{})",
                self.trade_performance.hourly_trade_count, 
                self.config.max_positions_per_hour
            ).red().to_string());
            return Ok(false);
        }
        
        // Check portfolio risk limits
        if self.portfolio_metrics.daily_pnl < self.config.max_daily_loss_percentage {
            self.logger.log(format!(
                "🚫 BLOCKED: Daily loss limit reached ({:.2}%)",
                self.portfolio_metrics.daily_pnl
            ).red().to_string());
            return Ok(false);
        }
        
        if self.portfolio_metrics.current_drawdown < self.config.max_drawdown_percentage {
            self.logger.log(format!(
                "🚫 BLOCKED: Max drawdown reached ({:.2}%)",
                self.portfolio_metrics.current_drawdown
            ).red().to_string());
            return Ok(false);
        }
        
        // Check market conditions
        if token_metrics.market_condition == MarketCondition::BearDump {
            self.logger.log("🚫 BLOCKED: Bear dump detected".red().to_string());
            return Ok(false);
        }
        
        self.logger.log("✅ Position allowed".green().to_string());
        Ok(true)
    }
    
    /// Update portfolio metrics
    pub fn update_portfolio_metrics(&mut self, portfolio_value: f64, positions: &HashMap<String, RealTimeTokenMetrics>) {
        self.portfolio_metrics.total_portfolio_value = portfolio_value;
        
        // Calculate total exposure
        self.portfolio_metrics.total_exposure = positions.values()
            .map(|metrics| metrics.cost_basis)
            .sum();
        
        // Calculate current PnL
        let total_unrealized_pnl: f64 = positions.values()
            .map(|metrics| metrics.unrealized_pnl_usd)
            .sum();
        
        // Update drawdown
        let current_total = portfolio_value + total_unrealized_pnl;
        self.portfolio_metrics.current_drawdown = if portfolio_value > 0.0 {
            ((current_total - portfolio_value) / portfolio_value) * 100.0
        } else {
            0.0
        };
        
        self.logger.log(format!(
            "📊 Portfolio update: Value: ${:.2} | Exposure: {:.1}% | Drawdown: {:.2}%",
            portfolio_value,
            (self.portfolio_metrics.total_exposure / portfolio_value) * 100.0,
            self.portfolio_metrics.current_drawdown
        ).cyan().to_string());
    }
    
    /// Record a trade result
    pub fn record_trade_result(&mut self, pnl_percentage: f64) {
        // Update consecutive counters
        if pnl_percentage > 0.0 {
            self.trade_performance.consecutive_wins += 1;
            self.trade_performance.consecutive_losses = 0;
        } else {
            self.trade_performance.consecutive_losses += 1;
            self.trade_performance.consecutive_wins = 0;
        }
        
        // Update trade time and counter
        self.trade_performance.last_trade_time = Instant::now();
        self.trade_performance.hourly_trade_count += 1;
        
        self.logger.log(format!(
            "📈 Trade recorded: PnL: {:+.2}% | Consecutive W/L: {}/{}",
            pnl_percentage,
            self.trade_performance.consecutive_wins,
            self.trade_performance.consecutive_losses
        ).blue().to_string());
    }
    
    /// Get risk management status
    pub fn get_risk_status(&self) -> String {
        format!(
            "🛡️ Risk Status: Portfolio: ${:.2} | Drawdown: {:.2}% | W/L: {}/{}",
            self.portfolio_metrics.total_portfolio_value,
            self.portfolio_metrics.current_drawdown,
            self.trade_performance.consecutive_wins,
            self.trade_performance.consecutive_losses
        )
    }
    
    /// Reset hourly counters
    pub fn reset_hourly_counters(&mut self) {
        self.trade_performance.hourly_trade_count = 0;
        self.logger.log("⏰ Hourly counters reset".blue().to_string());
    }
} 