use anyhow::Result;
use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Signature, signer::Signer};
use yellowstone_grpc_client::{GeyserGrpcClient, GeyserGrpcClientError, ClientTlsConfig};
use yellowstone_grpc_proto::geyser::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
    SubscribeRequestFilterTransactions, SubscribeUpdate,
};
use yellowstone_grpc_proto::prelude::{Transaction, SubscribeUpdateTransactionInfo, TransactionStatusMeta as GrpcTransactionStatusMeta};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use futures_util::SinkExt;
use std::sync::Arc;
use crate::common::{config::AppState, logger::Logger};
use crate::engine::transaction_parser;
use dashmap::DashMap;
use std::collections::HashMap;
use std::str::FromStr;
use colored::Colorize;
use solana_transaction_status::TransactionStatusMeta;
use maplit;

// Global state for wallet token balances
lazy_static::lazy_static! {
    static ref WALLET_TOKEN_BALANCES: DashMap<String, u64> = DashMap::new();
    static ref WALLET_SOL_BALANCE: DashMap<String, u64> = DashMap::new();
}

pub struct WalletMonitoringGrpc {
    app_state: Arc<AppState>,
    logger: Logger,
}

impl WalletMonitoringGrpc {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
            logger: Logger::new("[WALLET-GRPC] => ".cyan().bold().to_string()),
        }
    }

    pub async fn start_monitoring(&self) -> Result<(), String> {
        self.logger.log("Starting wallet gRPC monitoring...".green().to_string());
        
        let wallet_pubkey = self.app_state.wallet.pubkey();
        
        // Build gRPC client
        let mut client = GeyserGrpcClient::build_from_shared(self.app_state.yellowstone_grpc_http.clone())
            .map_err(|e| format!("Failed to build client: {}", e))?
            .x_token::<String>(Some(self.app_state.yellowstone_grpc_token.clone()))
            .map_err(|e| format!("Failed to set x_token: {}", e))?
            .tls_config(ClientTlsConfig::new().with_native_roots())
            .map_err(|e| format!("Failed to set tls config: {}", e))?
            .connect()
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        // Subscribe to wallet transactions
        let filter = SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            signature: None,
            account_include: vec![wallet_pubkey.to_string()],
            account_exclude: vec![],
            account_required: vec![wallet_pubkey.to_string()],
        };

        // Set up subscribe
        let (mut subscribe_tx, mut stream) = client.subscribe().await
            .map_err(|e| format!("Failed to subscribe: {}", e))?;

        // Send subscription request
        let request = SubscribeRequest {
            transactions: maplit::hashmap! {
                "Wallet".to_owned() => filter
            },
            commitment: Some(CommitmentLevel::Processed as i32),
            ..Default::default()
        };

        subscribe_tx.send(request).await
            .map_err(|e| format!("Failed to send subscription: {:?}", e))?;

        self.logger.log(format!("Monitoring wallet: {}", wallet_pubkey).green().to_string());

        while let Some(msg_result) = stream.next().await {
            match msg_result {
                Ok(msg) => {
                    if let Some(update) = msg.update_oneof {
                        match update {
                            UpdateOneof::Transaction(txn) => {
                                if let Some(transaction) = &txn.transaction {
                                    if let Some(meta) = &transaction.meta {
                                        if let Err(e) = self.process_wallet_transaction(transaction, meta, txn.slot, 0).await {
                                            self.logger.log(format!("Error processing wallet message: {}", e).red().to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    self.logger.log(format!("Stream error: {:?}", e).red().to_string());
                }
            }
        }

        Ok(())
    }

    async fn process_wallet_transaction(
        &self,
        transaction: &SubscribeUpdateTransactionInfo,
        meta: &GrpcTransactionStatusMeta,
        _slot: u64,
        _index: u32,
    ) -> Result<(), String> {
        let signature = transaction.signature.clone();
        let signature_str = bs58::encode(&signature).into_string();
        self.logger.log(format!("Wallet transaction detected: {}", signature_str).blue().to_string());

        // Process token balance changes
        self.process_token_balance_changes(meta, &signature_str).await?;
        
        // Process SOL balance changes
        self.process_sol_balance_changes(meta, &signature_str).await?;

        Ok(())
    }

    async fn process_token_balance_changes(&self, meta: &GrpcTransactionStatusMeta, signature: &str) -> Result<(), String> {
        // Process token balance changes from meta
        let pre_token_balances = &meta.pre_token_balances;
        let post_token_balances = &meta.post_token_balances;
        
        for (i, pre_balance) in pre_token_balances.iter().enumerate() {
            if let Some(post_balance) = post_token_balances.get(i) {
                let mint = pre_balance.mint.clone();
                let pre_amount = pre_balance.ui_token_amount.as_ref()
                    .map(|amount| amount.amount.parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);
                let post_amount = post_balance.ui_token_amount.as_ref()
                    .map(|amount| amount.amount.parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);
                let change = post_amount as i64 - pre_amount as i64;
                
                if change != 0 {
                    // Update global balance cache
                    WALLET_TOKEN_BALANCES.insert(mint.clone(), post_amount);
                    
                    // Only log significant changes (more than 1000 tokens)
                    if change.abs() > 1000 {
                        self.logger.log(format!(
                            "Token: {} | Change: {} {}",
                            mint,
                            if change > 0 { "RECEIVED" } else { "SENT" },
                            change.abs()
                        ).green().to_string());
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_sol_balance_changes(&self, meta: &GrpcTransactionStatusMeta, signature: &str) -> Result<(), String> {
        // Process SOL balance changes from meta
        for (i, pre_balance) in meta.pre_balances.iter().enumerate() {
            if let Some(post_balance) = meta.post_balances.get(i) {
                let change = *post_balance as i64 - *pre_balance as i64;
                
                if change != 0 {
                    // Update global SOL balance cache
                    WALLET_SOL_BALANCE.insert("SOL".to_string(), *post_balance);
                    
                    // Only log significant SOL changes (more than 0.01 SOL)
                    let change_sol = change.abs() as f64 / 1_000_000_000.0;
                    if change_sol > 0.01 {
                        self.logger.log(format!(
                            "SOL: {} {:.4} SOL",
                            if change > 0 { "RECEIVED" } else { "SENT" },
                            change_sol
                        ).green().to_string());
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn process_transaction_for_balance_update(
        &self,
        parsed_data: &transaction_parser::TradeInfoFromToken,
        signature: &str,
    ) -> Result<(), String> {
        let token_balance_change = parsed_data.token_amount.unwrap_or(0) as i64;
        let sol_balance_change = parsed_data.sol_amount.unwrap_or(0) as i64;
        
        // Update token balance
        if let Some(current_balance) = WALLET_TOKEN_BALANCES.get(&parsed_data.mint) {
            let new_balance = if parsed_data.is_buy {
                *current_balance + token_balance_change as u64
            } else {
                current_balance.saturating_sub(token_balance_change as u64)
            };
            WALLET_TOKEN_BALANCES.insert(parsed_data.mint.clone(), new_balance);
        }
        
        // Update SOL balance
        if let Some(current_sol) = WALLET_SOL_BALANCE.get("SOL") {
            let new_sol_balance = if parsed_data.is_buy {
                current_sol.saturating_sub(sol_balance_change as u64)
            } else {
                *current_sol + sol_balance_change as u64
            };
            WALLET_SOL_BALANCE.insert("SOL".to_string(), new_sol_balance);
        }
        
        // Calculate and log PnL for sells
        if !parsed_data.is_buy {
            let token_price = parsed_data.price as f64 / 1_000_000_000.0; // Convert to SOL
            let pnl_sol = sol_balance_change as f64 / 1_000_000_000.0;
            
            self.logger.log(format!(
                "SELL: {} | Amount: {} | Price: {:.6} SOL | PnL: {:.4} SOL",
                parsed_data.mint,
                token_balance_change.abs(),
                token_price,
                pnl_sol
            ).yellow().to_string());
        } else {
            // For buys, just log the basic info
            self.logger.log(format!(
                "BUY: {} | Amount: {} | Price: {:.6} SOL",
                parsed_data.mint,
                token_balance_change.abs(),
                parsed_data.price as f64 / 1_000_000_000.0
            ).blue().to_string());
        }
        
        Ok(())
    }
}

// Public API functions for other modules to access wallet balances
pub async fn get_cached_token_balance(token_mint: &str) -> Option<u64> {
    WALLET_TOKEN_BALANCES.get(token_mint).map(|balance| *balance)
}

pub async fn get_cached_sol_balance() -> Option<u64> {
    WALLET_SOL_BALANCE.get("SOL").map(|balance| *balance)
}

pub async fn force_update_token_balance(
    _app_state: &AppState,
    token_mint: &str,
    signature: String,
) -> Result<(), String> {
    // This would typically fetch from RPC and update cache
    // For now, we'll just log the update
    println!("Force updating token balance for {} with signature {}", token_mint, signature);
    Ok(())
}

pub async fn get_rpc_token_balance(
    _app_state: &AppState,
    token_mint: &str,
) -> Result<u64, String> {
    // This would fetch the actual balance from RPC
    // For now, return cached value or 0
    match get_cached_token_balance(token_mint).await {
        Some(balance) => Ok(balance),
        None => Ok(0),
    }
} 