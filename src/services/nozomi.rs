use crate::common::config::import_env_var;
use anyhow::{anyhow, Result};
use rand::{seq::IteratorRandom, thread_rng};
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    system_instruction,
    signature::Signature,
    message::Message,
    instruction::{Instruction, AccountMeta},
};
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use std::{str::FromStr, sync::LazyLock};
use spl_token::ui_amount_to_amount;

pub static NOZOMI_URL: LazyLock<String> = LazyLock::new(|| import_env_var("NOZOMI_URL"));
pub fn get_nozomi_tip() -> f64 {
    import_env_var("NOZOMI_TIP_VALUE").parse::<f64>().unwrap_or(0.001)
}

pub fn get_tip_account() -> Result<Pubkey> {
    let accounts = [
        "TEMPaMeCRFAS9EKF53Jd6KpHxgL47uWLcpFArU1Fanq".to_string(),
        "noz3jAjPiHuBPqiSPkkugaJDkJscPuRhYnSpbi8UvC4".to_string(),
        "noz3str9KXfpKknefHji8L1mPgimezaiUyCHYMDv1GE".to_string(),
        "noz6uoYCDijhu1V7cutCpwxNiSovEwLdRHPwmgCGDNo".to_string(),
        "noz9EPNcT7WH6Sou3sr3GGjHQYVkN3DNirpbvDkv9YJ".to_string(),
        "nozc5yT15LazbLTFVZzoNZCwjh3yUtW86LoUyqsBu4L".to_string(),
        "nozFrhfnNGoyqwVuwPAW4aaGqempx4PU6g6D9CJMv7Z".to_string(),
        "nozievPk7HyK1Rqy1MPJwVQ7qQg2QoJGyP71oeDwbsu".to_string(),
        "noznbgwYnBLDHu8wcQVCEw6kDrXkPdKkydGJGNXGvL7".to_string(),
        "nozNVWs5N8mgzuD3qigrCG2UoKxZttxzZ85pvAQVrbP".to_string(),
        "nozpEGbwx4BcGp6pvEdAh1JoC2CQGZdU6HbNP1v2p6P".to_string(),
        "nozrhjhkCr3zXT3BiT4WCodYCUFeQvcdUkM7MqhKqge".to_string(),
        "nozrwQtWhEdrA6W8dkbt9gnUaMs52PdAv5byipnadq3".to_string(),
        "nozUacTVWub3cL4mJmGCYjKZTnE9RbdY5AP46iQgbPJ".to_string(),
        "nozWCyTPppJjRuw2fpzDhhWbW355fzosWSzrrMYB1Qk".to_string(),
        "nozWNju6dY353eMkMqURqwQEoM3SFgEKC6psLCSfUne".to_string(),
        "nozxNBgWohjR75vdspfxR5H9ceC7XXH99xpxhVGt3Bb".to_string(),
    ];
    let mut rng = thread_rng();
    let tip_account = match accounts.iter().choose(&mut rng) {
        Some(acc) => Ok(Pubkey::from_str(acc).inspect_err(|err| {
            println!("nozomi: failed to parse Pubkey: {:?}", err);
        })?),
        None => Err(anyhow!("nozomi: no tip accounts available")),
    };

    let tip_account = tip_account?;
    Ok(tip_account)
}

/// Send a transaction with a tip to a randomly selected Nozomi tip account
pub async fn send_transaction_with_tip(
    rpc_client: &RpcClient,
    transaction: &Transaction,
    tip_lamports: f64,
) -> Result<Vec<Signature>> {
    // Convert tip amount to lamports
    let tip_amount = ui_amount_to_amount(tip_lamports, 9);
    
    // Get random tip account
    let tip_account = get_tip_account()?;
    
    // Create tip instruction
    let tip_ix = system_instruction::transfer(
        &transaction.message.account_keys[0], // First signer is fee payer
        &tip_account,
        tip_amount,
    );
    
    // Get all instructions from the original transaction
    let mut instructions = transaction.message.instructions.iter()
        .map(|compiled_ix| {
            let accounts: Vec<_> = compiled_ix.accounts.iter()
                .map(|&idx| {
                    let pubkey = &transaction.message.account_keys[idx as usize];
                    if transaction.message.is_signer(idx as usize) {
                        AccountMeta::new(*pubkey, true)
                    } else if transaction.message.is_maybe_writable(idx as usize, None) {
                        AccountMeta::new(*pubkey, false)
                    } else {
                        AccountMeta::new_readonly(*pubkey, false)
                    }
                })
                .collect();

            Instruction {
                program_id: transaction.message.account_keys[compiled_ix.program_id_index as usize],
                accounts,
                data: compiled_ix.data.clone(),
            }
        })
        .collect::<Vec<Instruction>>();
    
    // Add tip instruction
    instructions.push(tip_ix);
    
    // Create new message with all instructions
    let message = Message::new(
        &instructions,
        Some(&transaction.message.account_keys[0]), // Fee payer
    );
    
    // Create new transaction with updated message
    let mut final_tx = Transaction::new_unsigned(message);
    final_tx.signatures = transaction.signatures.clone();
    
    // Send transaction
    let signature = rpc_client.send_transaction(&final_tx)
        .await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
        
    Ok(vec![signature])
}


