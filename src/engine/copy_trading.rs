use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use crate::library::config::import_env_var;
use anyhow::Result;
use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::instruction::Instruction;
use spl_associated_token_account::get_associated_token_address;
use colored::Colorize;
use tokio::time;
use tokio::time::sleep;
use futures_util::stream::StreamExt;
use futures_util::{SinkExt, Sink};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
    SubscribeRequestFilterTransactions,  SubscribeUpdate,
};
use solana_transaction_status::TransactionConfirmationStatus;
use crate::engine::transaction_parser;
use crate::library::{
    config::{AppState, SwapConfig, JUPITER_PROGRAM, OKX_DEX_PROGRAM},
    logger::Logger,
    cache::WALLET_TOKEN_ACCOUNTS,
};
use crate::engine::monitor::TokenTrackingInfo;
use crate::engine::swap::{SwapDirection, SwapProtocol};
use crate::engine::comprehensive_selling::ComprehensiveSelling;

use crate::utilities::wallet_monitoring_grpc;
use tokio_util::sync::CancellationToken;
use dashmap::DashMap;

// Global state for copy trading
lazy_static::lazy_static! {
    static ref COUNTER: Arc<DashMap<(), u64>> = Arc::new(DashMap::new());
    static ref SOLD_TOKENS: Arc<DashMap<(), u64>> = Arc::new(DashMap::new());
    static ref BOUGHT_TOKENS: Arc<DashMap<(), u64>> = Arc::new(DashMap::new());
    static ref LAST_BUY_TIME: Arc<DashMap<(), Option<Instant>>> = Arc::new(DashMap::new());
    static ref BUYING_ENABLED: Arc<DashMap<(), bool>> = Arc::new(DashMap::new());
    static ref TOKEN_TRACKING: Arc<DashMap<String, TokenTrackingInfo>> = Arc::new(DashMap::new());
    // Global registry for monitoring task cancellation tokens
    static ref MONITORING_TASKS: Arc<DashMap<String, CancellationToken>> = Arc::new(DashMap::new());
    // Cache for target buy tokens to prevent duplicate buys
    static ref TARGET_BUY_TOKENS: Arc<DashMap<String, Instant>> = Arc::new(DashMap::new());
    
    // Bought token list for comprehensive selling
    static ref BOUGHT_TOKEN_LIST: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
}

// Initialize the global counters with default values
fn init_global_state() {
    COUNTER.insert((), 0);
    SOLD_TOKENS.insert((), 0);
    BOUGHT_TOKENS.insert((), 0);
    LAST_BUY_TIME.insert((), None);
    BUYING_ENABLED.insert((), true);
}

// Clean up old target buy tokens to prevent unbounded growth
fn cleanup_target_buy_tokens() {
    let now = Instant::now();
    let timeout = Duration::from_secs(3600); // 1 hour timeout
    
    TARGET_BUY_TOKENS.retain(|_, &mut timestamp| {
        now.duration_since(timestamp) < timeout
    });
}
