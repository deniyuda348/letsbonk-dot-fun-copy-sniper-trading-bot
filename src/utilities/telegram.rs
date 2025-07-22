use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use colored::Colorize;
use teloxide::prelude::*;
use teloxide::types::{ParseMode};
use crate::library::logger::Logger;
use crate::engine::transaction_parser::TradeInfoFromToken;
use dashmap::DashMap;

// Global state for Telegram bot
lazy_static::lazy_static! {
    static ref TELEGRAM_BOT: Arc<DashMap<(), Option<Bot>>> = Arc::new({
        let map = DashMap::new();
        map.insert((), None);
        map
    });
    static ref CHAT_ID: Arc<DashMap<(), Option<String>>> = Arc::new({
        let map = DashMap::new();
        map.insert((), None);
        map
    });
    static ref TRANSACTION_TIMESTAMPS: Arc<DashMap<String, Instant>> = Arc::new(DashMap::new());
}

/// Initialize the Telegram bot with the given token
pub async fn init() -> Result<()> {
    let logger = Logger::new("[TELEGRAM] => ".cyan().bold().to_string());
    
    // Get bot token from environment
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").map_err(|_| {
        logger.log("TELEGRAM_BOT_TOKEN not set in environment. Telegram notifications disabled.".yellow().to_string());
        anyhow!("TELEGRAM_BOT_TOKEN not set")
    })?;
    
    // Get chat ID from environment
    let chat_id = env::var("TELEGRAM_CHAT_ID").map_err(|_| {
        logger.log("TELEGRAM_CHAT_ID not set in environment. Telegram notifications disabled.".yellow().to_string());
        anyhow!("TELEGRAM_CHAT_ID not set")
    })?;
    
    // Initialize bot
    let bot = Bot::new(bot_token);
    
    // Store bot and chat ID in global state
    TELEGRAM_BOT.insert((), Some(bot));
    CHAT_ID.insert((), Some(chat_id.clone()));
    
    logger.log("Telegram bot initialized successfully".green().to_string());
    
    // Send a test message
    send_message("🤖 Copy Trading Bot started! Ready to monitor transactions.").await?;
    
    Ok(())
}

/// Send a message to the configured chat
pub async fn send_message(message: &str) -> Result<()> {
    let logger = Logger::new("[TELEGRAM] => ".cyan().bold().to_string());
    
    // Get bot and chat ID from global state
    let bot_ref = TELEGRAM_BOT.get(&());
    let chat_ref = CHAT_ID.get(&());
    
    match (bot_ref, chat_ref) {
        (Some(bot_ref), Some(chat_ref)) => {
            if let Some(bot) = bot_ref.value() {
                match bot.send_message(
                    ChatId::from(UserId(chat_ref.value().as_ref()
                        .ok_or_else(|| anyhow!("Missing chat ID"))?
                        .parse::<i64>()
                        .map_err(|e| anyhow!("Failed to parse chat ID: {}", e))?
                        .try_into()
                        .map_err(|e| anyhow!("Failed to convert chat ID: {}", e))?
                    )),
                    message
                )
                .parse_mode(ParseMode::MarkdownV2)
                .disable_web_page_preview(true)
                .await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(anyhow!("Failed to send message: {}", e))
                }
            } else {
                Err(anyhow!("Bot not initialized"))
            }
        },
        _ => {
            logger.log("Telegram bot not initialized".yellow().to_string());
            Ok(()) // Return Ok to avoid disrupting the main flow
        }
    }
}

/// Format a trade notification message
pub fn format_trade_notification(
    trade_info: &TradeInfoFromToken,
    protocol: &str,
    action: &str,
) -> String {
    // Escape special characters for Markdown V2
    let escape_md = |s: &str| -> String {
        s.replace('_', "\\_")
         .replace('*', "\\*")
         .replace('[', "\\[")
         .replace(']', "\\]")
         .replace('(', "\\(")
         .replace(')', "\\)")
         .replace('~', "\\~")
         .replace('`', "\\`")
         .replace('>', "\\>")
         .replace('#', "\\#")
         .replace('+', "\\+")
         .replace('-', "\\-")
         .replace('=', "\\=")
         .replace('|', "\\|")
         .replace('{', "\\{")
         .replace('}', "\\}")
         .replace('.', "\\.")
         .replace('!', "\\!")
    };
    
    // Determine emoji based on action
    let emoji = match action {
        "DETECTED" => "👀",
        "BOUGHT" => "✅",
        "SOLD" => "💰",
        "ERROR" => "❌",
        _ => "ℹ️",
    };
    
    // Format token amount with K suffix for thousands
    let token_amount_str = if trade_info.token_amount_f64 >= 1000.0 {
        format!("{:.2}K", trade_info.token_amount_f64 / 1000.0)
    } else {
        format!("{:.2}", trade_info.token_amount_f64)
    };
    
    // Format the instruction type
    let instruction_type = match trade_info.dex_type {
        crate::engine::transaction_parser::DexType::PumpFun => "Buy",
        crate::engine::transaction_parser::DexType::PumpSwap => "Swap",
        crate::engine::transaction_parser::DexType::RaydiumLaunchpad => "Raydium",
        crate::engine::transaction_parser::DexType::Unknown => "Unknown",
    };
    
    // Build the message with full addresses
    format!(
        "{} *{}* on *{}*\n\
        \n\
        Token: `{}`\n\
        Amount: `{}`\n\
        Target: `{}`\n\
        TX: `{}`\n\
        [View on Solscan](https://solscan.io/tx/{})\n\
        Action: *{}*",
        emoji,
        escape_md(instruction_type),
        escape_md(protocol),
        escape_md(&trade_info.mint),
        escape_md(&token_amount_str),
        escape_md(&trade_info.target),
        escape_md(&trade_info.signature),
        trade_info.signature,
        escape_md(action)
    )
}

/// Send a trade notification
pub async fn send_trade_notification(
    trade_info: &TradeInfoFromToken,
    protocol: &str,
    action: &str,
) -> Result<()> {
    let message = format_trade_notification(trade_info, protocol, action);
    send_message(&message).await
}

/// Send an error notification
pub async fn send_error_notification(error: &str) -> Result<()> {
    // Escape special characters for Markdown V2
    let escape_md = |s: &str| -> String {
        s.replace('_', "\\_")
         .replace('*', "\\*")
         .replace('[', "\\[")
         .replace(']', "\\]")
         .replace('(', "\\(")
         .replace(')', "\\)")
         .replace('~', "\\~")
         .replace('`', "\\`")
         .replace('>', "\\>")
         .replace('#', "\\#")
         .replace('+', "\\+")
         .replace('-', "\\-")
         .replace('=', "\\=")
         .replace('|', "\\|")
         .replace('{', "\\{")
         .replace('}', "\\}")
         .replace('.', "\\.")
         .replace('!', "\\!")
    };

    let escaped_error = escape_md(error);
    let message = format!("❌ *ERROR*\n\n{}", escaped_error);
    send_message(&message).await
}

/// Record the timestamp when a target transaction is detected
pub async fn record_target_transaction(trade_info: &TradeInfoFromToken) {
    TRANSACTION_TIMESTAMPS.insert(trade_info.mint.clone(), Instant::now());
}

/// Format a copy trade notification with time elapsed
pub fn format_copy_trade_notification(
    target_trade: &TradeInfoFromToken,
    my_signature: &str,
    protocol: &str,
    action: &str,
    elapsed: Option<Duration>,
) -> String {
    // Escape special characters for Markdown V2
    let escape_md = |s: &str| -> String {
        s.replace('_', "\\_")
         .replace('*', "\\*")
         .replace('[', "\\[")
         .replace(']', "\\]")
         .replace('(', "\\(")
         .replace(')', "\\)")
         .replace('~', "\\~")
         .replace('`', "\\`")
         .replace('>', "\\>")
         .replace('#', "\\#")
         .replace('+', "\\+")
         .replace('-', "\\-")
         .replace('=', "\\=")
         .replace('|', "\\|")
         .replace('{', "\\{")
         .replace('}', "\\}")
         .replace('.', "\\.")
         .replace('!', "\\!")
    };
    
    // Determine emoji based on action
    let emoji = match action {
        "COPIED" => "🔄",
        "BOUGHT" => "✅",
        "SOLD" => "💰",
        "ERROR" => "❌",
        _ => "ℹ️",
    };
    
    // Format token amount with K suffix for thousands
    let token_amount_str = if target_trade.token_amount_f64 >= 1000.0 {
        format!("{:.2}K", target_trade.token_amount_f64 / 1000.0)
    } else {
        format!("{:.2}", target_trade.token_amount_f64)
    };
    
    // Format the elapsed time if available
    let elapsed_str = match elapsed {
        Some(elapsed) => {
            let ms = elapsed.as_millis();
            if ms < 1000 {
                format!("{} ms", ms)
            } else {
                format!("{:.2} s", elapsed.as_secs_f64())
            }
        },
        None => "Unknown".to_string(),
    };
    
    // Format the instruction type
    let instruction_type = match target_trade.dex_type {
        crate::engine::transaction_parser::DexType::PumpFun => "Buy",
        crate::engine::transaction_parser::DexType::PumpSwap => "Swap",
        crate::engine::transaction_parser::DexType::RaydiumLaunchpad => "Raydium",
        crate::engine::transaction_parser::DexType::Unknown => "Unknown",
    };
    
    // Build the message with full addresses
    format!(
        "{} *COPY TRADE* \\- *{}* on *{}*\n\
        \n\
        Token: `{}`\n\
        Amount: `{}`\n\
        Target TX: `{}`\n\
        My TX: `{}`\n\
        Time Elapsed: `{}`\n\
        [View Target TX](https://solscan.io/tx/{})\n\
        [View My TX](https://solscan.io/tx/{})\n\
        Action: *{}*",
        emoji,
        escape_md(instruction_type),
        escape_md(protocol),
        escape_md(&target_trade.mint),
        escape_md(&token_amount_str),
        escape_md(&target_trade.signature),
        escape_md(my_signature),
        escape_md(&elapsed_str),
        target_trade.signature,
        my_signature,
        escape_md(action)
    )
}

/// Send a copy trade notification with elapsed time
pub async fn send_copy_trade_notification(
    target_trade: &TradeInfoFromToken,
    my_signature: &str,
    protocol: &str,
    action: &str,
) -> Result<()> {
    // Get elapsed time if available
    let elapsed = TRANSACTION_TIMESTAMPS
        .get(&target_trade.signature)
        .map(|entry| entry.elapsed());

    let message = format_copy_trade_notification(target_trade, my_signature, protocol, action, elapsed);
    send_message(&message).await
}

/// Format and send a summary notification
pub async fn send_summary_notification(
    bought: u64,
    sold: u64,
    active_tokens: Vec<String>,
    total_pnl: f64,
) -> Result<()> {
    // Escape special characters for Markdown V2
    let escape_md = |s: &str| -> String {
        s.replace('_', "\\_")
         .replace('*', "\\*")
         .replace('[', "\\[")
         .replace(']', "\\]")
         .replace('(', "\\(")
         .replace(')', "\\)")
         .replace('~', "\\~")
         .replace('`', "\\`")
         .replace('>', "\\>")
         .replace('#', "\\#")
         .replace('+', "\\+")
         .replace('-', "\\-")
         .replace('=', "\\=")
         .replace('|', "\\|")
         .replace('{', "\\{")
         .replace('}', "\\}")
         .replace('.', "\\.")
         .replace('!', "\\!")
    };

    // Format active tokens list
    let tokens_str = if active_tokens.is_empty() {
        "None".to_string()
    } else {
        active_tokens.join(", ")
    };

    // Format PNL with color indicator
    let pnl_str = if total_pnl > 0.0 {
        format!("+{:.4} SOL", total_pnl)
    } else {
        format!("{:.4} SOL", total_pnl)
    };

    // Build the summary message
    let message = format!(
        "📊 *Copy Trading Summary*\n\
        \n\
        Tokens Bought: `{}`\n\
        Tokens Sold: `{}`\n\
        Active Tokens: `{}`\n\
        Total PNL: `{}`\n\
        ",
        escape_md(&bought.to_string()),
        escape_md(&sold.to_string()),
        escape_md(&tokens_str),
        escape_md(&pnl_str)
    );

    send_message(&message).await
} 