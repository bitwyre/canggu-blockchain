use crate::blockchain::chain::Blockchain;
use crate::crypto::hash::Hash;
use crate::crypto::keys::PublicKey;
use crate::transaction::mempool::TransactionPool;
use crate::transaction::tx::Transaction;
use anyhow::{Result, anyhow};
use jsonrpc_http_server::{ServerBuilder, Server};
use jsonrpc_core::{IoHandler, Params, Value, Error as RpcError};
use log::{info, error};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

/// RPC server for blockchain interaction
pub struct RpcServer {
    /// HTTP server instance
    server: Server,
}

impl RpcServer {
    /// Create and start a new RPC server
    pub fn start(
        blockchain: Arc<Blockchain>,
        mempool: Arc<RwLock<TransactionPool>>,
        addr: SocketAddr,
    ) -> Result<Self> {
        let mut io = IoHandler::default();
        
        // Register RPC methods
        Self::register_methods(&mut io, blockchain, mempool);
        
        // Create server
        let server = ServerBuilder::new(io)
            .threads(4)
            .start_http(&addr)
            .map_err(|e| anyhow!("Failed to start RPC server: {}", e))?;
            
        info!("RPC server listening on {}", addr);
        
        Ok(Self { server })
    }
    
    /// Register RPC methods
    fn register_methods(
        io: &mut IoHandler,
        blockchain: Arc<Blockchain>,
        mempool: Arc<RwLock<TransactionPool>>,
    ) {
        // Get balance
        let blockchain_clone = blockchain.clone();
        io.add_method("getBalance", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<String> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: account"));
                }
                
                let account_str = &params[0];
                let pubkey = PublicKey::from_string(account_str).map_err(|e| {
                    RpcError::invalid_params(format!("Invalid public key: {}", e))
                })?;
                
                // Get account balance
                let state = blockchain.state.read().unwrap();
                let balance = state.accounts.get(&pubkey)
                    .map(|account| account.balance)
                    .unwrap_or(0);
                
                Ok(Value::Number(balance.into()))
            }
        });
        
        // Get transaction count
        let blockchain_clone = blockchain.clone();
        io.add_method("getTransactionCount", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<String> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: account"));
                }
                
                let account_str = &params[0];
                let pubkey = PublicKey::from_string(account_str).map_err(|e| {
                    RpcError::invalid_params(format!("Invalid public key: {}", e))
                })?;
                
                // Get account nonce
                let state = blockchain.state.read().unwrap();
                let nonce = state.accounts.get(&pubkey)
                    .map(|account| account.nonce)
                    .unwrap_or(0);
                
                Ok(Value::Number(nonce.into()))
            }
        });
        
        // Send transaction
        let blockchain_clone = blockchain.clone();
        io.add_method("sendTransaction", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<String> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: transaction"));
                }
                
                let tx_data = base64::decode(&params[0]).map_err(|e| {
                    RpcError::invalid_params(format!("Invalid transaction encoding: {}", e))
                })?;
                
                let tx: Transaction = bincode::deserialize(&tx_data).map_err(|e| {
                    RpcError::invalid_params(format!("Invalid transaction format: {}", e))
                })?;
                
                // Add transaction to mempool
                let tx_hash = blockchain.add_transaction(tx).map_err(|e| {
                    RpcError::invalid_request(format!("Invalid transaction: {}", e))
                })?;
                
                Ok(Value::String(tx_hash.to_hex()))
            }
        });
        
        // Get block
        let blockchain_clone = blockchain.clone();
        io.add_method("getBlock", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<u64> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: slot"));
                }
                
                let slot = params[0];
                
                // Get block
                let block = blockchain.storage.get_block_by_slot(slot).map_err(|e| {
                    RpcError::internal_error(format!("Storage error: {}", e))
                })?;
                
                if let Some(block) = block {
                    // Convert block to JSON
                    let json = serde_json::to_value(block).map_err(|e| {
                        RpcError::internal_error(format!("Serialization error: {}", e))
                    })?;
                    
                    Ok(json)
                } else {
                    Err(RpcError::invalid_params(format!("Block not found: {}", slot)))
                }
            }
        });
        
        // Get current slot
        let blockchain_clone = blockchain.clone();
        io.add_method("getSlot", move |_params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let height = blockchain.height();
                Ok(Value::Number(height.into()))
            }
        });
        
        // Get account info
        let blockchain_clone = blockchain.clone();
        io.add_method("getAccountInfo", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<String> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: account"));
                }
                
                let account_str = &params[0];
                let pubkey = PublicKey::from_string(account_str).map_err(|e| {
                    RpcError::invalid_params(format!("Invalid public key: {}", e))
                })?;
                
                // Get account
                let state = blockchain.state.read().unwrap();
                if let Some(account) = state.accounts.get(&pubkey) {
                    // Convert account to JSON
                    let json = json!({
                        "balance": account.balance,
                        "nonce": account.nonce,
                        "owner": account.owner.to_string(),
                        "executable": account.executable,
                        "data": base64::encode(&account.data),
                    });
                    
                    Ok(json)
                } else {
                    Err(RpcError::invalid_params(format!("Account not found: {}", account_str)))
                }
            }
        });
        
        // Get program info
        let blockchain_clone = blockchain.clone();
        io.add_method("getProgramInfo", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<String> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: program_id"));
                }
                
                let program_id_str = &params[0];
                let program_id = Hash::from_hex(program_id_str).map_err(|e| {
                    RpcError::invalid_params(format!("Invalid program ID: {}", e))
                })?;
                
                // Get program
                let state = blockchain.state.read().unwrap();
                if let Some(program) = state.programs.get(&program_id) {
                    // Convert program to JSON
                    let json = json!({
                        "owner": program.owner.to_string(),
                        "deployment_slot": program.deployment_slot,
                        "code_size": program.code.len(),
                    });
                    
                    Ok(json)
                } else {
                    Err(RpcError::invalid_params(format!("Program not found: {}", program_id_str)))
                }
            }
        });
        
        // Get transaction status
        let blockchain_clone = blockchain.clone();
        io.add_method("getTransaction", move |params: Params| {
            let blockchain = blockchain_clone.clone();
            
            async move {
                let params: Vec<String> = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {}", e))
                })?;
                
                if params.len() != 1 {
                    return Err(RpcError::invalid_params("Expected 1 parameter: signature"));
                }
                
                let signature_str = &params[0];
                
                // In a real implementation, we would look up the transaction by signature
                // For simplicity, we're just returning a placeholder
                
                let json = json!({
                    "signature": signature_str,
                    "slot": blockchain.height(),
                    "status": "confirmed",
                });
                
                Ok(json)
            }
        });
    }
} 