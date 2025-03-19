use crate::crypto::hash::Hash;
use crate::crypto::keys::{Keypair, PublicKey};
use crate::transaction::tx::Transaction;
use anyhow::{Result, anyhow};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Call the blockchain RPC
async fn call_rpc(url: &str, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let response = client.post(url).json(&request).send().await?;

    let json: serde_json::Value = response.json().await?;

    if let Some(error) = json.get("error") {
        Err(anyhow!("RPC error: {}", error))
    } else if let Some(result) = json.get("result") {
        Ok(result.clone())
    } else {
        Err(anyhow!("Invalid RPC response"))
    }
}

/// Get the balance of an account
pub async fn get_balance(url: &str, pubkey: &str) -> Result<u64> {
    let params = json!([pubkey]);

    let result = call_rpc(url, "getBalance", params).await?;

    // Parse balance
    result
        .as_u64()
        .ok_or_else(|| anyhow!("Invalid balance response"))
}

/// Get the transaction count of an account
pub async fn get_transaction_count(url: &str, pubkey: &str) -> Result<u64> {
    let params = json!([pubkey]);

    let result = call_rpc(url, "getTransactionCount", params).await?;

    // Parse count
    result
        .as_u64()
        .ok_or_else(|| anyhow!("Invalid transaction count response"))
}

/// Get information about a block
pub async fn get_block(url: &str, slot: u64) -> Result<serde_json::Value> {
    let params = json!([slot]);

    call_rpc(url, "getBlock", params).await
}

/// Get the current slot
pub async fn get_slot(url: &str) -> Result<u64> {
    let params = json!([]);

    let result = call_rpc(url, "getSlot", params).await?;

    // Parse slot
    result
        .as_u64()
        .ok_or_else(|| anyhow!("Invalid slot response"))
}

/// Transfer tokens from one account to another
pub async fn transfer(url: &str, from_keypair: &Keypair, to: &str, amount: u64) -> Result<String> {
    // Parse recipient public key
    let to_pubkey = PublicKey::from_string(to)?;

    // Create transaction
    let tx = Transaction::new_transfer(from_keypair, &to_pubkey, amount);

    // Submit transaction
    let tx_json = serde_json::to_string(&tx)?;
    let params = json!([tx_json]);

    let result = call_rpc(url, "sendTransaction", params).await?;

    // Parse transaction signature
    result
        .as_str()
        .ok_or_else(|| anyhow!("Invalid transaction response"))
        .map(|s| s.to_string())
}

/// Deploy a program to the blockchain
pub async fn deploy_program(url: &str, keypair: &Keypair, program_path: &Path) -> Result<String> {
    // Read program file
    let program_data = fs::read(program_path)?;

    // Create transaction
    let tx = Transaction::new_deploy_program(keypair, program_data);

    // Submit transaction
    let tx_json = serde_json::to_string(&tx)?;
    let params = json!([tx_json]);

    let result = call_rpc(url, "sendTransaction", params).await?;

    // Parse transaction signature
    result
        .as_str()
        .ok_or_else(|| anyhow!("Invalid transaction response"))
        .map(|s| s.to_string())
}

/// Call a program on the blockchain
pub async fn call_program(
    url: &str,
    keypair: &Keypair,
    program_id: &str,
    data: Vec<u8>,
) -> Result<String> {
    // Parse program ID
    let program_hash = Hash::from_hex(program_id)?;

    // Create transaction
    let tx = Transaction::new_call_program(keypair, program_hash, data);

    // Submit transaction
    let tx_json = serde_json::to_string(&tx)?;
    let params = json!([tx_json]);

    let result = call_rpc(url, "sendTransaction", params).await?;

    // Parse transaction signature
    result
        .as_str()
        .ok_or_else(|| anyhow!("Invalid transaction response"))
        .map(|s| s.to_string())
}

/// Get information about a program
pub async fn get_program_info(url: &str, program_id: &str) -> Result<serde_json::Value> {
    let params = json!([program_id]);

    call_rpc(url, "getProgramInfo", params).await
}

/// Get the account information
pub async fn get_account_info(url: &str, pubkey: &str) -> Result<serde_json::Value> {
    let params = json!([pubkey]);

    call_rpc(url, "getAccountInfo", params).await
}

/// Get the transaction status
pub async fn get_transaction_status(url: &str, signature: &str) -> Result<serde_json::Value> {
    let params = json!([signature]);

    call_rpc(url, "getTransaction", params).await
}
