use crate::blockchain::chain::Blockchain;
use crate::blockchain::storage::BlockchainStorage;
use crate::config::Config;
use crate::consensus::proof_of_history::ProofOfHistory;
use crate::crypto::keys::{Keypair, PublicKey};
use crate::network::gossip::GossipService;
use crate::network::message::Message;
use crate::transaction::mempool::TransactionPool;
use anyhow::{Result, anyhow};
use log::{info, warn, error};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Main blockchain node
pub struct Node {
    /// Node configuration
    config: Config,
    
    /// Data directory
    data_dir: PathBuf,
    
    /// RPC port
    rpc_port: u16,
    
    /// Node keypair
    keypair: Option<Keypair>,
}

impl Node {
    /// Create a new node
    pub fn new(config: Config, data_dir: PathBuf, rpc_port: u16) -> Self {
        Self {
            config,
            data_dir,
            rpc_port,
            keypair: None,
        }
    }
    
    /// Start the node
    pub async fn start(&mut self) -> Result<()> {
        // Load or create identity
        self.load_identity()?;
        
        // Create blockchain storage
        let storage = Arc::new(BlockchainStorage::new(&self.data_dir)?);
        
        // Create transaction pool
        let mempool = Arc::new(RwLock::new(TransactionPool::new(10000)));
        
        // Create PoH generator
        let poh = Arc::new(RwLock::new(ProofOfHistory::new(400_000))); // 400 microseconds per hash
        
        // Get validator key if we're a validator
        let validator_key = self.keypair.as_ref().map(|kp| kp.public());
        
        // Create blockchain
        let (blockchain, mut block_rx) = Blockchain::new(
            storage.clone(),
            mempool.clone(),
            poh.clone(),
            validator_key,
        )?;
        
        // Create network channels
        let (network_tx, mut network_rx) = mpsc::channel(100);
        
        // Start gossip service
        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", self.config.p2p_port);
        let gossip = GossipService::new(
            &listen_addr,
            self.config.max_peers,
            network_tx,
        ).await?;
        
        // Start blockchain
        blockchain.start(self.config.block_time_target).await?;
        
        // Start RPC server
        self.start_rpc_server(blockchain.clone(), mempool.clone())?;
        
        // Process network messages
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Process incoming network messages
                    Some((peer_id, message)) = network_rx.recv() => {
                        match message {
                            Message::Transaction(tx) => {
                                if let Err(e) = blockchain.add_transaction(tx.clone()) {
                                    warn!("Invalid transaction from {}: {}", peer_id, e);
                                } else {
                                    // Successfully added to mempool
                                }
                            },
                            Message::Block(block) => {
                                if let Err(e) = blockchain.process_block(block.clone()).await {
                                    warn!("Invalid block from {}: {}", peer_id, e);
                                } else {
                                    // Successfully processed block
                                }
                            },
                            // Handle other message types
                            _ => {}
                        }
                    },
                    
                    // Process newly produced blocks
                    Some(block) = block_rx.recv() => {
                        // Broadcast block to network
                        gossip.broadcast_block(block).await;
                    }
                }
            }
        });
        
        // Connect to seed nodes
        for seed in &self.config.seed_nodes {
            if let Err(e) = gossip.add_peer(seed).await {
                warn!("Failed to connect to seed node {}: {}", seed, e);
            }
        }
        
        info!("Node started on port {}", self.rpc_port);
        
        // Keep the main thread alive
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    }
    
    /// Load or create node identity
    fn load_identity(&mut self) -> Result<()> {
        let identity_path = self.data_dir.join("identity.json");
        
        if identity_path.exists() {
            // Load existing identity
            self.keypair = Some(Keypair::load_from_file(&identity_path)?);
            info!("Loaded identity: {}", self.keypair.as_ref().unwrap().public());
        } else {
            // Create new identity
            let keypair = Keypair::new();
            keypair.save_to_file(&identity_path)?;
            self.keypair = Some(keypair);
            info!("Created new identity: {}", self.keypair.as_ref().unwrap().public());
        }
        
        Ok(())
    }
    
    /// Start the RPC server
    fn start_rpc_server(
        &self,
        blockchain: Arc<Blockchain>,
        mempool: Arc<RwLock<TransactionPool>>,
    ) -> Result<()> {
        // In a real implementation, this would start a JSON-RPC server
        // For simplicity, we're just logging that it would start
        info!("Starting RPC server on port {}", self.rpc_port);
        
        Ok(())
    }
} 