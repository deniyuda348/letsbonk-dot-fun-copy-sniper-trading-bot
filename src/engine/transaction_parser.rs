use bs58;
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use colored::Colorize;
use crate::library::logger::Logger;
use lazy_static;
use yellowstone_grpc_proto::geyser::SubscribeUpdateTransaction;
use std::time::Instant;
use crate::dex::pump_fun::PUMP_PROGRAM;
// Create a static logger for this module
lazy_static::lazy_static! {
    static ref LOGGER: Logger = Logger::new("[PARSER] => ".blue().to_string());
}

#[derive(Clone, Debug, PartialEq)]
pub enum DexType {
    PumpSwap,
    PumpFun,
    RaydiumLaunchpad,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct ParsedData {
    pub sol_change: f64,
    pub token_change: f64,
    pub is_buy: bool,
    pub user: String,
    pub mint: Option<String>,
    pub timestamp: Option<u64>,
    pub real_sol_reserves: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct PumpSwapData {
    pub timestamp: u64,
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_out: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_out_without_lp_fee: u64,
    pub user_quote_amount_out: u64,
    pub pool: String,
    pub user: String,
    pub target_user_base_token_account: String,
    pub target_user_quote_token_account: String,
    pub protocol_fee_recipient: String,
    pub protocol_fee_recipient_token_account: String,
}

#[derive(Clone, Debug)]
pub struct PumpFunData {
    pub mint: String,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: String,
    pub timestamp: u64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
}

#[derive(Clone, Debug)]
pub struct TransactionOutput {
    pub sol_changes: f64,
    pub token_changes: f64,
    pub is_buy: bool,
    pub user: String,
    pub instruction_type: DexType,
    pub timestamp: u64,
    pub mint: String,
    pub signature: String,
}

#[derive(Clone, Debug)]
pub struct TradeInfoFromToken {
    // Common fields
    pub dex_type: DexType,
    pub slot: u64,
    pub signature: String,
    pub target: String,
    // Fields from both PumpSwapData and PumpFunData
    pub mint: String,
    pub user: String,
    pub timestamp: u64,
    pub is_buy: bool,
    // New fields for price and reverse case
    pub price: u64,
    pub is_reverse_when_pump_swap: bool,
    // PumpSwapData fields
    pub base_amount_in_or_base_amount_out: Option<u64>, // renamed from base_amount_in
    pub min_quote_amount_out: Option<u64>,
    pub user_base_token_reserves: Option<u64>,
    pub user_quote_token_reserves: Option<u64>,
    pub pool_base_token_reserves: Option<u64>,
    pub pool_quote_token_reserves: Option<u64>,
    pub quote_amount_out: Option<u64>, //  quote_amount_out in case of sell , quote_amount_in in case of buy
    pub lp_fee_basis_points: Option<u64>,
    pub lp_fee: Option<u64>,
    pub protocol_fee_basis_points: Option<u64>,
    pub protocol_fee: Option<u64>,
    pub quote_amount_out_without_lp_fee: Option<u64>,
    pub user_quote_amount_out: Option<u64>, // user_quote_amount_out in case of sell , user_base_amount_in in case of buy
    pub pool: Option<String>,
    pub user_base_token_account: Option<String>,
    pub user_quote_token_account: Option<String>,
    pub protocol_fee_recipient: Option<String>,
    pub protocol_fee_recipient_token_account: Option<String>,
    pub coin_creator: Option<String>,
    pub coin_creator_fee_basis_points: Option<u64>,
    pub coin_creator_fee: Option<u64>,
    
    // PumpFunData fields
    pub sol_amount: Option<u64>,
    pub token_amount: Option<u64>,
    pub virtual_sol_reserves: Option<u64>,
    pub virtual_token_reserves: Option<u64>,
    pub real_sol_reserves: Option<u64>,
    pub real_token_reserves: Option<u64>,
    
    // Additional fields from original TradeInfoFromToken
    pub bonding_curve: String,
    pub volume_change: i64,
    pub bonding_curve_info: Option<crate::engine::monitor::BondingCurveInfo>,
    pub pool_info: Option<crate::engine::monitor::PoolInfo>,
    pub token_amount_f64: f64,
    pub amount: Option<u64>,
    pub max_sol_cost: Option<u64>,
    pub min_sol_output: Option<u64>,
    pub base_amount_out: Option<u64>,
    pub max_quote_amount_in: Option<u64>,
}
/// Helper function to check if transaction contains MintTo instruction
/// NOTE: This function is no longer used - we now process all transactions regardless of MintTo
fn _has_mint_to_instruction(txn: &SubscribeUpdateTransaction) -> bool {
    if let Some(tx_inner) = &txn.transaction {
        if let Some(meta) = &tx_inner.meta {
            // Check log messages for "Program log: Instruction: MintTo"
            return meta.log_messages.iter().any(|log| {
                log.contains("Program log: Instruction: MintTo")
            });
        }
    }
    false
}

/// Helper function to check if transaction contains Buy instruction
fn has_buy_instruction(txn: &SubscribeUpdateTransaction) -> bool {
    if let Some(tx_inner) = &txn.transaction {
        if let Some(meta) = &tx_inner.meta {
            return meta.log_messages.iter().any(|log| {
                log.contains("Program log: Instruction: Buy")
            });
        }
    }
    false
}

/// Helper function to check if transaction contains Sell instruction
fn has_sell_instruction(txn: &SubscribeUpdateTransaction) -> bool {
    if let Some(tx_inner) = &txn.transaction {
        if let Some(meta) = &tx_inner.meta {
            return meta.log_messages.iter().any(|log| {
                log.contains("Program log: Instruction: Sell")
            });
        }
    }
    false
}
