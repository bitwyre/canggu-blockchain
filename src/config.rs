use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl Network {
    pub fn id(&self) -> &str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Devnet => "devnet",
        }
    }
}

/// Configuration for the blockchain node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Network identifier
    pub network_id: Network,

    /// Port for RPC server
    pub rpc_port: u16,

    /// Port for P2P communication
    pub p2p_port: u16,

    /// Maximum number of peers to connect to
    pub max_peers: usize,

    /// Target block time in seconds
    pub block_time_target: u64,

    /// Seed nodes to connect to on startup
    pub seed_nodes: Vec<String>,

    /// Maximum size of a transaction in bytes
    pub max_transaction_size: usize,

    /// Maximum number of transactions per block
    pub max_transactions_per_block: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network_id: Network::Devnet,
            rpc_port: 8899,
            p2p_port: 8900,
            max_peers: 50,
            block_time_target: 400, // milliseconds
            seed_nodes: vec![],
            max_transaction_size: 1024 * 10, // 10 KB
            max_transactions_per_block: 1000,
        }
    }
}

impl Config {
    /// Load configuration from a file, or create default if file doesn't exist
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: Config = serde_json::from_str(&contents)?;
            Ok(config)
        } else {
            let config = Config::default();
            let json = serde_json::to_string_pretty(&config)?;
            fs::write(path, json)?;
            Ok(config)
        }
    }

    /// Save configuration to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
