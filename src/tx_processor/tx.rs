use std::sync::Arc;
use std::str::FromStr;
use anyhow::{Result, anyhow};
use colored::Colorize;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    system_instruction,
    transaction::Transaction,
    hash::Hash,
    signature::Signature,
};
use std::env;
use anchor_client::solana_sdk::pubkey::Pubkey;
use spl_token::ui_amount_to_amount;
use solana_sdk::signature::Signer;
use tokio::time::{Instant, sleep};
use once_cell::sync::Lazy;
use reqwest::{Client, ClientBuilder};
use base64;
use std::time::Duration;

use crate::{
    library::logger::Logger,
    utilities::{
        nozomi,
        zeroslot,
    },
};
use dotenv::dotenv;

// prioritization fee = UNIT_PRICE * UNIT_LIMIT
fn get_unit_price() -> u64 {
    env::var("UNIT_PRICE")
        .ok()
        .and_then(|v| u64::from_str(&v).ok())
        .unwrap_or(20000)
}

fn get_unit_limit() -> u32 {
    env::var("UNIT_LIMIT")
        .ok()
        .and_then(|v| u32::from_str(&v).ok())
        .unwrap_or(200_000)
}

// Cache the tip value for better performance
static NOZOMI_TIP_VALUE: Lazy<f64> = Lazy::new(|| {
    std::env::var("NOZOMI_TIP_VALUE")
        .ok()
        .and_then(|v| f64::from_str(&v).ok())
        .unwrap_or(0.0015)
});

// Cache the FlashBlock API key
static FLASHBLOCK_API_KEY: Lazy<String> = Lazy::new(|| {
    std::env::var("FLASHBLOCK_API_KEY")
        .ok()
        .unwrap_or_else(|| "da07907679634859".to_string())
});

// Create a static HTTP client with optimized configuration for FlashBlock API
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
   let client = reqwest::Client::new();
   client
});

// Get nozomi tip value from env
pub fn get_nozomi_tip() -> f64 {
    dotenv().ok();
    std::env::var("NOZOMI_TIP_VALUE")
        .ok()
        .and_then(|v| f64::from_str(&v).ok())
        .unwrap_or(0.0015)
}
