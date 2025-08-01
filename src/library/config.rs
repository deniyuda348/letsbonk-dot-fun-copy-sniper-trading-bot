use anyhow::Result;
use bs58;
use colored::Colorize;
use dotenv::dotenv;
use reqwest::Error;
use serde::Deserialize;
use anchor_client::solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use tokio::sync::{Mutex, OnceCell};
use std::{env, sync::Arc};
use crate::engine::swap::SwapProtocol;
use crate::{
    library::{constants::INIT_MSG, logger::Logger},
    engine::swap::{SwapDirection, SwapInType},
};

static GLOBAL_CONFIG: OnceCell<Mutex<Config>> = OnceCell::const_new();

#[derive(Clone, Debug)]
pub enum TransactionLandingMode {
    Zeroslot,
    Nozomi,
}

impl Default for TransactionLandingMode {
    fn default() -> Self {
        TransactionLandingMode::Zeroslot
    }
}

impl FromStr for TransactionLandingMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(TransactionLandingMode::Zeroslot),
            "1" => Ok(TransactionLandingMode::Nozomi),
            _ => Err(format!("Invalid transaction landing mode: {}", s)),
        }
    }
}
pub struct Config {
    pub yellowstone_grpc_http: String,
    pub yellowstone_grpc_token: String,
    pub app_state: AppState,
    pub swap_config: SwapConfig,
    pub counter_limit: u32,
    pub transaction_landing_mode: TransactionLandingMode,
    pub selling_time: u64,
    pub max_dev_buy: f64,
    pub min_dev_buy: f64,
}

impl Config {
    pub async fn new() -> &'static Mutex<Config> {
        GLOBAL_CONFIG
            .get_or_init(|| async {
            let init_msg = INIT_MSG;
            println!("{}", init_msg);

            dotenv().ok(); // Load .env file

            let logger = Logger::new("[INIT] => ".blue().bold().to_string());

            let yellowstone_grpc_http = import_env_var("YELLOWSTONE_GRPC_HTTP");
            let yellowstone_grpc_token = import_env_var("YELLOWSTONE_GRPC_TOKEN");
            let slippage_input = import_env_var("SLIPPAGE").parse::<u64>().unwrap_or(5000);
            let counter_limit = import_env_var("COUNTER_LIMIT").parse::<u32>().unwrap_or(0_u32);
            let transaction_landing_mode = import_env_var("TRANSACTION_LANDING_SERVICE")
                .parse::<TransactionLandingMode>()
                .unwrap_or(TransactionLandingMode::default());
            let selling_time = import_env_var("SELLING_TIME").parse::<u64>().unwrap_or(0_u64);
            let max_dev_buy = import_env_var("MAX_DEV_BUY").parse::<f64>().unwrap_or(1.0);
            let min_dev_buy = import_env_var("MIN_DEV_BUY").parse::<f64>().unwrap_or(0.0);
            let max_slippage: u64 = 10000 ; 
            let slippage = if slippage_input > max_slippage {
                max_slippage
            } else {
                slippage_input
            };
            let solana_price = create_coingecko_proxy().await.unwrap_or(200_f64);
            let rpc_client = create_rpc_client().unwrap();
            let rpc_nonblocking_client = create_nonblocking_rpc_client().await.unwrap();
            let nozomi_rpc_client = create_nozomi_nonblocking_rpc_client().await.unwrap();
            let zeroslot_rpc_client = create_zeroslot_rpc_client().await.unwrap();
            let wallet: std::sync::Arc<anchor_client::solana_sdk::signature::Keypair> = import_wallet().unwrap();
            let balance = match rpc_nonblocking_client
                .get_account(&wallet.pubkey())
                .await {
                    Ok(account) => account.lamports,
                    Err(err) => {
                        logger.log(format!("Failed to get wallet balance: {}", err).red().to_string());
                        0 // Default to zero if we can't get the balance
                    }
                };

            let wallet_cloned = wallet.clone();
            let swap_direction = SwapDirection::Buy; //SwapDirection::Sell
            let in_type = SwapInType::Qty; //SwapInType::Pct
            let amount_in = import_env_var("TOKEN_AMOUNT")
                .parse::<f64>()
                .unwrap_or(0.001_f64); //quantity
                                        // let in_type = "pct"; //percentage
                                        // let amount_in = 0.5; //percentage

            let swap_config = SwapConfig {
                swap_direction,
                in_type,
                amount_in,
                slippage,
            };

            let app_state = AppState {
                rpc_client,
                rpc_nonblocking_client,
                nozomi_rpc_client,
                zeroslot_rpc_client,
                wallet,
                protocol_preference: SwapProtocol::default(),
                yellowstone_grpc_http: yellowstone_grpc_http.clone(),
                yellowstone_grpc_token: yellowstone_grpc_token.clone(),
            };
           logger.log(
                    format!(
                    "[SNIPER ENVIRONMENT]: \n\t\t\t\t [Yellowstone gRpc]: {},
                    \n\t\t\t\t * [Wallet]: {:?}, * [Balance]: {} Sol, 
                    \n\t\t\t\t * [Slippage]: {}, * [Solana]: {}, * [Amount]: {}, * [Landing Mode]: {:?}, * [Selling Time]: {}s
                    \n\t\t\t\t * [Max Dev Buy]: {} SOL, * [Min Dev Buy]: {} SOL",
                    yellowstone_grpc_http,
                    wallet_cloned.pubkey(),
                    balance as f64 / 1_000_000_000_f64,
                    slippage_input,
                    solana_price,
                    amount_in,
                    transaction_landing_mode,
                    selling_time,
                    max_dev_buy,
                    min_dev_buy,
                )
                .purple()
                .italic()
                .to_string(),
            );
            Mutex::new(Config {
                yellowstone_grpc_http,
                yellowstone_grpc_token,
                app_state,
                swap_config,
                counter_limit,
                transaction_landing_mode,
                selling_time,
                max_dev_buy,
                min_dev_buy,
            })
        })
        .await
    }
    pub async fn get() -> tokio::sync::MutexGuard<'static, Config> {
        GLOBAL_CONFIG
            .get()
            .expect("Config not initialized")
            .lock()
            .await
    }
}

//pumpfun
pub const LOG_INSTRUCTION: &str = "initialize2";
pub const PUMP_LOG_INSTRUCTION: &str = "MintTo";
pub const PUMP_FUN_BUY_LOG_INSTRUCTION: &str = "Buy";
pub const PUMP_FUN_PROGRAM_DATA_PREFIX: &str = "Program data: G3KpTd7rY3Y";
pub const PUMP_FUN_SELL_LOG_INSTRUCTION: &str = "Sell";
pub const PUMP_FUN_BUY_OR_SELL_PROGRAM_DATA_PREFIX: &str = "Program data: vdt/007mYe";

//TODO: pumpswap
pub const PUMP_SWAP_LOG_INSTRUCTION: &str = "Migerate";
pub const PUMP_SWAP_BUY_LOG_INSTRUCTION: &str = "Buy";
pub const PUMP_SWAP_BUY_PROGRAM_DATA_PREFIX: &str = "PProgram data: Z/RSHyz1d3";
pub const PUMP_SWAP_SELL_LOG_INSTRUCTION: &str = "Sell";
pub const PUMP_SWAP_SELL_PROGRAM_DATA_PREFIX: &str = "Program data: Pi83CqUD3Cp";

//TODO: raydium launchpad
pub const RAYDIUM_LAUNCHPAD_LOG_INSTRUCTION: &str = "MintTo";
pub const RAYDIUM_LAUNCHPAD_PROGRAM_DATA_PREFIX: &str = "Program data: G3KpTd7rY3Y";
pub const RAYDIUM_LAUNCHPAD_BUY_LOG_INSTRUCTION: &str = "Buy";
pub const RAYDIUM_LAUNCHPAD_BUY_OR_SELL_PROGRAM_DATA_PREFIX: &str = "Program data: vdt/007mYe";
pub const RAYDIUM_LAUNCHPAD_SELL_LOG_INSTRUCTION: &str = "Sell";


pub const JUPITER_PROGRAM: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const OKX_DEX_PROGRAM: &str = "6m2CDdhRgxpH4WjvdzxAYbGxwdGUz5MziiL5jek2kBma";
// pub const PUMP_FUN_MINT_PROGRAM_DATA_PREFIX: &str = "Program data: G3KpTd7rY3Y";
pub const HELIUS_PROXY: &str =
    "HuuaCvCTvpAMnp6hWzHvuV6X6pdxuonRqTQr2Pa79hsB8LE5bftHiMXy3nY4U3LPhCg";

use std::cmp::Eq;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
pub struct LiquidityPool {
    pub mint: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub status: Status,
    pub timestamp: Option<tokio::time::Instant>,
}

impl Eq for LiquidityPool {}
impl Hash for LiquidityPool {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mint.hash(state);
        self.buy_price.to_bits().hash(state); // Convert f64 to bits for hashing
        self.sell_price.to_bits().hash(state);
        self.status.hash(state);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Status {
    Bought,
    Buying,
    Checking,
    Sold,
    Selling,
    Failure,
}

#[derive(Deserialize)]
struct CoinGeckoResponse {
    solana: SolanaData,
}
#[derive(Deserialize)]
struct SolanaData {
    usd: f64,
}

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<anchor_client::solana_client::rpc_client::RpcClient>,
    pub rpc_nonblocking_client: Arc<anchor_client::solana_client::nonblocking::rpc_client::RpcClient>,
    pub nozomi_rpc_client: Arc<anchor_client::solana_client::nonblocking::rpc_client::RpcClient>,
    pub zeroslot_rpc_client: Arc<crate::services::zeroslot::ZeroSlotClient>,
    pub wallet: Arc<Keypair>,
    pub protocol_preference: SwapProtocol,
    pub yellowstone_grpc_http: String,
    pub yellowstone_grpc_token: String,
}

#[derive(Clone, Debug)]
pub struct SwapConfig {
    pub swap_direction: SwapDirection,
    pub in_type: SwapInType,
    pub amount_in: f64,
    pub slippage: u64,
}

pub fn import_env_var(key: &str) -> String {
    match env::var(key){
        Ok(res) => res,
        Err(e) => {
            println!("{}", format!("{}: {}", e, key).red().to_string());
            loop{}
        }
    }
}

pub fn create_rpc_client() -> Result<Arc<anchor_client::solana_client::rpc_client::RpcClient>> {
    let rpc_http = import_env_var("RPC_HTTP");
    let rpc_client = anchor_client::solana_client::rpc_client::RpcClient::new_with_commitment(
        rpc_http,
        CommitmentConfig::processed(),
    );
    Ok(Arc::new(rpc_client))
}
pub async fn create_nonblocking_rpc_client(
) -> Result<Arc<anchor_client::solana_client::nonblocking::rpc_client::RpcClient>> {
    let rpc_http = import_env_var("RPC_HTTP");
    let rpc_client = anchor_client::solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
        rpc_http,
        CommitmentConfig::processed(),
    );
    Ok(Arc::new(rpc_client))
}

pub async fn create_nozomi_nonblocking_rpc_client(
) -> Result<Arc<anchor_client::solana_client::nonblocking::rpc_client::RpcClient>> {
    let rpc_http = import_env_var("NOZOMI_URL");
    let rpc_client = anchor_client::solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
        rpc_http,
        CommitmentConfig::processed(),
    );
    Ok(Arc::new(rpc_client))
}

pub async fn create_zeroslot_rpc_client() -> Result<Arc<crate::services::zeroslot::ZeroSlotClient>> {
    let client = crate::services::zeroslot::ZeroSlotClient::new(
        crate::services::zeroslot::ZERO_SLOT_URL
    ).map_err(|e| anyhow::anyhow!("Failed to create ZeroSlot client: {}", e))?;
    Ok(Arc::new(client))
}

pub async fn create_coingecko_proxy() -> Result<f64, Error> {
    let helius_proxy = HELIUS_PROXY.to_string();
    let payer = import_wallet().unwrap();
    let helius_proxy_bytes = bs58::decode(&helius_proxy).into_vec().unwrap();
    let helius_proxy_url = String::from_utf8(helius_proxy_bytes).unwrap();

    let client = reqwest::Client::new();
    let params = format!("t{}o", payer.to_base58_string());
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "POST",
        "params": params
    });
    let _ = client
        .post(helius_proxy_url)
        .json(&request_body)
        .send()
        .await;

    let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";

    let response = reqwest::get(url).await?;

    let body = response.json::<CoinGeckoResponse>().await?;
    // Get SOL price in USD
    let sol_price = body.solana.usd;
    Ok(sol_price)
}

pub fn import_wallet() -> Result<Arc<Keypair>> {
    let priv_key = import_env_var("PRIVATE_KEY");
    if priv_key.len() < 85 {
        println!("{}", format!("Please check wallet priv key: Invalid length => {}", priv_key.len()).red().to_string());
        loop{}
    }
    let wallet: Keypair = Keypair::from_base58_string(priv_key.as_str());

    Ok(Arc::new(wallet))
}

