/*
 * Copy Trading Bot with PumpSwap Notification Mode
 * 
 * Changes made:
 * - Modified PumpSwap buy/sell logic to only send notifications without executing transactions
 * - Transaction processing now runs in separate tokio tasks to ensure main monitoring continues
 * - Added placeholder for future selling strategy implementation
 * - PumpFun protocol functionality remains unchanged
 * - Added caching and batch RPC calls for improved performance
 */
use anchor_client::solana_sdk::signature::Signer;
use solana_vntr_sniper::{
    library::{config::Config, constants::RUN_MSG, cache::WALLET_TOKEN_ACCOUNTS},
    engine::{
        copy_trading::{start_copy_trading, CopyTradingConfig},
        swap::SwapProtocol,
    },
    utilities::{telegram, cache_maintenance, blockhash_processor::BlockhashProcessor},
    tx_processor::token,
};
use solana_vntr_sniper::library::config::{JUPITER_PROGRAM, OKX_DEX_PROGRAM};
use solana_program_pack::Pack;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::solana_sdk::system_instruction;
use std::str::FromStr;
use colored::Colorize;
use spl_token::instruction::sync_native;
use spl_token::ui_amount_to_amount;
use spl_associated_token_account::get_associated_token_address;

/// Initialize the wallet token account list by fetching all token accounts owned by the wallet
async fn initialize_token_account_list(config: &Config) {
    let logger = solana_vntr_sniper::library::logger::Logger::new("[INIT-TOKEN-ACCOUNTS] => ".green().to_string());
    
    if let Ok(wallet_pubkey) = config.app_state.wallet.try_pubkey() {
        logger.log(format!("Initializing token account list for wallet: {}", wallet_pubkey));
        
        // Get the token program pubkey
        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        
        // Query all token accounts owned by the wallet
        let accounts = config.app_state.rpc_client.get_token_accounts_by_owner(
            &wallet_pubkey,
            anchor_client::solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program)
        );
        match accounts {
            Ok(accounts) => {
                logger.log(format!("Found {} token accounts", accounts.len()));
                
                // Add each token account to our global cache
                for account in accounts {
                    WALLET_TOKEN_ACCOUNTS.insert(Pubkey::from_str(&account.pubkey).unwrap());
                    logger.log(format!("Added token account: {}", account.pubkey ));
                }
                
                logger.log(format!("Token account list initialized with {} accounts", WALLET_TOKEN_ACCOUNTS.size()));
            },
            Err(e) => {
                logger.log(format!("Error fetching token accounts: {}", e));
            }
        }
    } else {
        logger.log("Failed to get wallet pubkey, can't initialize token account list".to_string());
    }
}

/// Wrap SOL to Wrapped SOL (WSOL)
async fn wrap_sol(config: &Config, amount: f64) -> Result<(), String> {
    let logger = solana_vntr_sniper::library::logger::Logger::new("[WRAP-SOL] => ".green().to_string());
    
    // Get wallet pubkey
    let wallet_pubkey = match config.app_state.wallet.try_pubkey() {
        Ok(pk) => pk,
        Err(_) => return Err("Failed to get wallet pubkey".to_string()),
    };
    
    // Create WSOL account instructions
    let (wsol_account, mut instructions) = match token::create_wsol_account(wallet_pubkey) {
        Ok(result) => result,
        Err(e) => return Err(format!("Failed to create WSOL account: {}", e)),
    };
    
    logger.log(format!("WSOL account address: {}", wsol_account));
    
    // Convert UI amount to lamports (1 SOL = 10^9 lamports)
    let lamports = ui_amount_to_amount(amount, 9);
    logger.log(format!("Wrapping {} SOL ({} lamports)", amount, lamports));
    
    // Transfer SOL to the WSOL account
    instructions.push(
        system_instruction::transfer(
            &wallet_pubkey,
            &wsol_account,
            lamports,
        )
    );
    
    // Sync native instruction to update the token balance
    instructions.push(
        sync_native(
            &spl_token::id(),
            &wsol_account,
        ).map_err(|e| format!("Failed to create sync native instruction: {}", e))?
    );
    
    // Send transaction
    let recent_blockhash = config.app_state.rpc_client.get_latest_blockhash()
        .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;
    
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&wallet_pubkey),
        &[&config.app_state.wallet],
        recent_blockhash,
    );
    
    match config.app_state.rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            logger.log(format!("SOL wrapped successfully, signature: {}", signature));
            Ok(())
        },
        Err(e) => {
            Err(format!("Failed to wrap SOL: {}", e))
        }
    }
}

/// Unwrap SOL from Wrapped SOL (WSOL) account
async fn unwrap_sol(config: &Config) -> Result<(), String> {
    let logger = solana_vntr_sniper::library::logger::Logger::new("[UNWRAP-SOL] => ".green().to_string());
    
    // Get wallet pubkey
    let wallet_pubkey = match config.app_state.wallet.try_pubkey() {
        Ok(pk) => pk,
        Err(_) => return Err("Failed to get wallet pubkey".to_string()),
    };
    
    // Get the WSOL ATA address
    let wsol_account = get_associated_token_address(
        &wallet_pubkey,
        &spl_token::native_mint::id()
    );
    
    logger.log(format!("WSOL account address: {}", wsol_account));
    
    // Check if WSOL account exists
    match config.app_state.rpc_client.get_account(&wsol_account) {
        Ok(_) => {
            logger.log(format!("Found WSOL account: {}", wsol_account));
        },
        Err(_) => {
            return Err(format!("WSOL account does not exist: {}", wsol_account));
        }
    }
    
    // Close the WSOL account to recover SOL
    let close_instruction = token::close_account(
        wallet_pubkey,
        wsol_account,
        wallet_pubkey,
        wallet_pubkey,
        &[&wallet_pubkey],
    ).map_err(|e| format!("Failed to create close account instruction: {}", e))?;
    
    // Send transaction
    let recent_blockhash = config.app_state.rpc_client.get_latest_blockhash()
        .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;
    
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&wallet_pubkey),
        &[&config.app_state.wallet],
        recent_blockhash,
    );
    
    match config.app_state.rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            logger.log(format!("WSOL unwrapped successfully, signature: {}", signature));
            Ok(())
        },
        Err(e) => {
            Err(format!("Failed to unwrap WSOL: {}", e))
        }
    }
}

/// Close all token accounts owned by the wallet
async fn close_all_token_accounts(config: &Config) -> Result<(), String> {
    let logger = solana_vntr_sniper::library::logger::Logger::new("[CLOSE-TOKEN-ACCOUNTS] => ".green().to_string());
    
    // Get wallet pubkey
    let wallet_pubkey = match config.app_state.wallet.try_pubkey() {
        Ok(pk) => pk,
        Err(_) => return Err("Failed to get wallet pubkey".to_string()),
    };
    
    // Get the token program pubkey
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    
    // Query all token accounts owned by the wallet
    let accounts = config.app_state.rpc_client.get_token_accounts_by_owner(
        &wallet_pubkey,
        anchor_client::solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program)
    ).map_err(|e| format!("Failed to get token accounts: {}", e))?;
    
    if accounts.is_empty() {
        logger.log("No token accounts found to close".to_string());
        return Ok(());
    }
    
    logger.log(format!("Found {} token accounts to close", accounts.len()));
    
    let mut closed_count = 0;
    let mut failed_count = 0;
    
    // Close each token account
    for account_info in accounts {
        let token_account = Pubkey::from_str(&account_info.pubkey)
            .map_err(|_| format!("Invalid token account pubkey: {}", account_info.pubkey))?;
        
        // Skip WSOL accounts with non-zero balance (these need to be unwrapped first)
        let account_data = match config.app_state.rpc_client.get_account(&token_account) {
            Ok(data) => data,
            Err(e) => {
                logger.log(format!("Failed to get account data for {}: {}", token_account, e).red().to_string());
                failed_count += 1;
                continue;
            }
        };
        
        // Check if this is a WSOL account with balance
        if let Ok(token_data) = spl_token::state::Account::unpack(&account_data.data) {
            if token_data.mint == spl_token::native_mint::id() && token_data.amount > 0 {
                logger.log(format!("Skipping WSOL account with non-zero balance: {} ({})", 
                                 token_account, 
                                 token_data.amount as f64 / 1_000_000_000.0));
                continue;
            }
        }
        
        // Create close instruction
        let close_instruction = token::close_account(
            wallet_pubkey,
            token_account,
            wallet_pubkey,
            wallet_pubkey,
            &[&wallet_pubkey],
        ).map_err(|e| format!("Failed to create close instruction for {}: {}", token_account, e))?;
        
        // Send transaction
        let recent_blockhash = config.app_state.rpc_client.get_latest_blockhash()
            .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[close_instruction],
            Some(&wallet_pubkey),
            &[&config.app_state.wallet],
            recent_blockhash,
        );
        
        match config.app_state.rpc_client.send_and_confirm_transaction(&transaction) {
            Ok(signature) => {
                logger.log(format!("Closed token account {}, signature: {}", token_account, signature));
                closed_count += 1;
            },
            Err(e) => {
                logger.log(format!("Failed to close token account {}: {}", token_account, e).red().to_string());
                failed_count += 1;
            }
        }
    }
    
    logger.log(format!("Closed {} token accounts, {} failed", closed_count, failed_count));
    
    if failed_count > 0 {
        Err(format!("Failed to close {} token accounts", failed_count))
    } else {
        Ok(())
    }
}

/// Initialize target wallet token list by fetching all token accounts owned by the target wallet
async fn initialize_target_wallet_token_list(config: &Config, target_addresses: &[String]) -> Result<(), String> {
    let logger = solana_vntr_sniper::library::logger::Logger::new("[INIT-TARGET-TOKENS] => ".green().to_string());
    
    // Check if we should initialize
    let should_check = std::env::var("IS_CHECK_TARGET_WALLET_TOKEN_ACCOUNT")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);
        
    if !should_check {
        logger.log("Skipping target wallet token check as IS_CHECK_TARGET_WALLET_TOKEN_ACCOUNT is not true".to_string());
        return Ok(());
    }
    
    // Get the token program pubkey
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    
    for target_address in target_addresses {
        // Parse target wallet address
        let target_pubkey = match Pubkey::from_str(target_address) {
            Ok(pk) => pk,
            Err(e) => {
                logger.log(format!("Invalid target address {}: {}", target_address, e).red().to_string());
                continue;
            }
        };
        
        // Query all token accounts owned by the target wallet
        match config.app_state.rpc_client.get_token_accounts_by_owner(
            &target_pubkey,
            anchor_client::solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program)
        ) {
            Ok(accounts) => {
                logger.log(format!("Found {} token accounts for target {}", accounts.len(), target_address));
                
                // Add each token's mint to our global cache
                for account in accounts {
                    if let Ok(token_account) = config.app_state.rpc_client.get_account(&Pubkey::from_str(&account.pubkey).unwrap()) {
                        if let Ok(parsed) = spl_token::state::Account::unpack(&token_account.data) {
                            solana_vntr_sniper::library::cache::TARGET_WALLET_TOKENS.insert(parsed.mint.to_string());
                            logger.log(format!("Added token mint {} to target wallet list", parsed.mint));
                        }
                    }
                }
            },
            Err(e) => {
                logger.log(format!("Error fetching token accounts for target {}: {}", target_address, e).red().to_string());
            }
        }
    }
    
    logger.log(format!(
        "Target wallet token list initialized with {} tokens",
        solana_vntr_sniper::library::cache::TARGET_WALLET_TOKENS.size()
    ));
    
    Ok(())
}

#[tokio::main]
async fn main() {
    /* Initial Settings */
    let config = Config::new().await;
    let config = config.lock().await;

    /* Running Bot */
    let run_msg = RUN_MSG;
    println!("{}", run_msg);
    
    // Initialize blockhash processor
    match BlockhashProcessor::new(config.app_state.rpc_client.clone()).await {
        Ok(processor) => {
            if let Err(e) = processor.start().await {
                eprintln!("Failed to start blockhash processor: {}", e);
                return;
            }
            println!("Blockhash processor started successfully");
        },
        Err(e) => {
            eprintln!("Failed to initialize blockhash processor: {}", e);
            return;
        }
    }

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        // Check for command line arguments
        if args.contains(&"--wrap".to_string()) {
            println!("Wrapping SOL to WSOL...");
            
            // Get wrap amount from .env
            let wrap_amount = std::env::var("WRAP_AMOUNT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.1);
            
            match wrap_sol(&config, wrap_amount).await {
                Ok(_) => {
                    println!("Successfully wrapped {} SOL to WSOL", wrap_amount);
                    return;
                },
                Err(e) => {
                    eprintln!("Failed to wrap SOL: {}", e);
                    return;
                }
            }
        } else if args.contains(&"--unwrap".to_string()) {
            println!("Unwrapping WSOL to SOL...");
            
            match unwrap_sol(&config).await {
                Ok(_) => {
                    println!("Successfully unwrapped WSOL to SOL");
                    return;
                },
                Err(e) => {
                    eprintln!("Failed to unwrap WSOL: {}", e);
                    return;
                }
            }
        } else if args.contains(&"--close".to_string()) {
            println!("Closing all token accounts...");
            
            match close_all_token_accounts(&config).await {
                Ok(_) => {
                    println!("Successfully closed all token accounts");
                    return;
                },
                Err(e) => {
                    eprintln!("Failed to close all token accounts: {}", e);
                    return;
                }
            }
        }
    }

    // Initialize Telegram bot
    match telegram::init().await {
        Ok(_) => println!("Telegram bot initialized successfully"),
        Err(e) => println!("Failed to initialize Telegram bot: {}. Continuing without notifications.", e),
    }
    
    // Initialize token account list
    initialize_token_account_list(&config).await;
    
    // Start cache maintenance service (clean up expired cache entries every 60 seconds)
    cache_maintenance::start_cache_maintenance(60).await;
    println!("Cache maintenance service started");

    // Get copy trading target addresses from environment
    let copy_trading_target_address = std::env::var("COPY_TRADING_TARGET_ADDRESS").ok();
    let is_multi_copy_trading = std::env::var("IS_MULTI_COPY_TRADING")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);
    let excluded_addresses_str = std::env::var("EXCLUDED_ADDRESSES").ok();
    
    // Prepare target addresses for monitoring
    let mut target_addresses = Vec::new();
    let mut excluded_addresses = Vec::new();

    // Handle multiple copy trading targets if enabled
    if is_multi_copy_trading {
        if let Some(address_str) = copy_trading_target_address {
            // Parse comma-separated addresses
            for addr in address_str.split(',') {
                let trimmed_addr = addr.trim();
                if !trimmed_addr.is_empty() {
                    target_addresses.push(trimmed_addr.to_string());
                }
            }
        }
    } else if let Some(address) = copy_trading_target_address {
        // Single address mode
        if !address.is_empty() {
            target_addresses.push(address);
        }
    }
    
    if let Some(excluded_addresses_str) = excluded_addresses_str {
        for addr in excluded_addresses_str.split(',') {
            let trimmed_addr = addr.trim();
            if !trimmed_addr.is_empty() {
                excluded_addresses.push(trimmed_addr.to_string());
            }
        }
    }

    if target_addresses.is_empty() {
        eprintln!("No COPY_TRADING_TARGET_ADDRESS specified. Please set this environment variable.");
        return;
    }
    
    // Initialize target wallet token list
    if let Err(e) = initialize_target_wallet_token_list(&config, &target_addresses).await {
        eprintln!("Failed to initialize target wallet token list: {}", e);
        return;
    }
    
    // Get protocol preference from environment
    let protocol_preference = std::env::var("PROTOCOL_PREFERENCE")
        .ok()
        .map(|p| match p.to_lowercase().as_str() {
            "pumpfun" => SwapProtocol::PumpFun,
            "pumpswap" => SwapProtocol::PumpSwap,
            _ => SwapProtocol::Auto,
        })
        .unwrap_or(SwapProtocol::Auto);
    
    // Get buy-in-sell configuration from environment
    let buy_in_sell = std::env::var("BUY_IN_SELL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.05);
    
    let buy_in_sell_limit = std::env::var("BUY_IN_SELL_LIMIT")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(10.0);
    
    // Create copy trading config
    let copy_trading_config = CopyTradingConfig {
        yellowstone_grpc_http: config.yellowstone_grpc_http.clone(),
        yellowstone_grpc_token: config.yellowstone_grpc_token.clone(),
        app_state: config.app_state.clone(),
        swap_config: config.swap_config.clone(),
        counter_limit: config.counter_limit as u64,
        target_addresses,
        excluded_addresses: vec![JUPITER_PROGRAM.to_string(), OKX_DEX_PROGRAM.to_string()],
        protocol_preference: SwapProtocol::default(),
        buy_in_sell: 0.05,
        buy_in_sell_limit: 10.0,
        selling_time: config.selling_time,
        transaction_landing_mode: config.transaction_landing_mode.clone(),
        max_dev_buy: config.max_dev_buy,
        min_dev_buy: config.min_dev_buy,
    };
    
    // Start the copy trading bot
    if let Err(e) = start_copy_trading(copy_trading_config).await {
        eprintln!("Copy trading error: {}", e);
        
        // Send error notification via Telegram
        if let Err(te) = telegram::send_error_notification(&format!("Copy trading bot crashed: {}", e)).await {
            eprintln!("Failed to send Telegram notification: {}", te);
        }
    }
}
