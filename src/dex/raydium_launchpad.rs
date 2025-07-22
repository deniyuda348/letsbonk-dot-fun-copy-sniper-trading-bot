use std::{str::FromStr, sync::Arc};
use solana_program_pack::Pack;
use anchor_client::solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use anchor_client::solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_account_decoder::UiAccountEncoding;
use anyhow::{anyhow, Result};
use colored::Colorize;
use anchor_client::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    system_program,
    signer::Signer,
};
use crate::engine::transaction_parser::DexType;
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::create_associated_token_account_idempotent
};
use spl_token::ui_amount_to_amount;


use crate::{
    library::{config::SwapConfig, logger::Logger, cache::WALLET_TOKEN_ACCOUNTS},
    tx_processor::token,
    engine::swap::{SwapDirection, SwapInType},
};

// Constants - moved to lazy_static for single initialization
lazy_static::lazy_static! {
    static ref TOKEN_PROGRAM: Pubkey = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    static ref TOKEN_2022_PROGRAM: Pubkey = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();
    static ref ASSOCIATED_TOKEN_PROGRAM: Pubkey = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();
    static ref RAYDIUM_LAUNCHPAD_PROGRAM: Pubkey = Pubkey::from_str("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj").unwrap();
    static ref RAYDIUM_LAUNCHPAD_AUTHORITY: Pubkey = Pubkey::from_str("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh").unwrap();
    static ref RAYDIUM_GLOBAL_CONFIG: Pubkey = Pubkey::from_str("6s1xP3hpbAfFoNtUNF8mfHsjr2Bd97JxFJRWLbL6aHuX").unwrap();
    static ref RAYDIUM_PLATFORM_CONFIG: Pubkey = Pubkey::from_str("FfYek5vEz23cMkWsdJwG2oa6EphsvXSHrGpdALN4g6W1").unwrap();
    static ref EVENT_AUTHORITY: Pubkey = Pubkey::from_str("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr").unwrap();
    static ref SOL_MINT: Pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    static ref BUY_DISCRIMINATOR: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236]; //buy_exact_in discriminator
    static ref SELL_DISCRIMINATOR: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26]; //sell_exact_in discriminator
}

const TEN_THOUSAND: u64 = 10000;
const POOL_VAULT_SEED: &[u8] = b"pool_vault";



/// A struct to represent the Raydium pool which uses constant product AMM
#[derive(Debug, Clone)]
pub struct RaydiumPool {
    pub pool_id: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_account: Pubkey,
    pub pool_quote_account: Pubkey,
    pub virtual_base_reserve: u64,
    pub virtual_quote_reserve: u64,
    pub real_base_reserve: u64,
    pub real_quote_reserve: u64,
}

pub struct Raydium {
    pub keypair: Arc<Keypair>,
    pub rpc_client: Option<Arc<anchor_client::solana_client::rpc_client::RpcClient>>,
    pub rpc_nonblocking_client: Option<Arc<anchor_client::solana_client::nonblocking::rpc_client::RpcClient>>,
}
