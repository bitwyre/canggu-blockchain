use crate::blockchain::block::{Block, create_genesis_block};
use crate::blockchain::state::BlockchainState;
use crate::blockchain::storage::BlockchainStorage;
use crate::config::Config;
use crate::consensus::proof_of_history::ProofOfHistory;
use crate::consensus::slot_leader::SlotLeaderSchedule;
use crate::crypto::hash::Hash;
use crate::crypto::keys::{Keypair, PublicKey};
use crate::network::gossip::GossipService;
use crate::network::message::Message;
use crate::network::peer::PeerManager;
use crate::runtime::instruction::InstructionProcessor;
use crate::runtime::program::ProgramRegistry;
use crate::transaction::pool::TransactionPool;
use crate::transaction::tx::Transaction;

use anyhow::{Result, anyhow};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time;

/// Blockchain node
pub struct Node {
    /// Node configuration
    config: Config,

    /// Data directory
    data_dir: PathBuf,

    /// RPC port
    rpc_port: u16,

    /// Node identity
    identity: Keypair,

    /// Blockchain storage
    storage: Option<BlockchainStorage>,

    /// Transaction pool
    tx_pool: Option<Arc<TransactionPool>>,

    /// Proof of History generator
    poh: Option<ProofOfHistory>,

    /// Slot leader schedule
    leader_schedule: Option<SlotLeaderSchedule>,

    /// Program registry
    program_registry: Option<ProgramRegistry>,

    /// Gossip service
    gossip_service: Option<GossipService>,

    /// Instruction processor
    instruction_processor: Option<InstructionProcessor>,

    /// Is this node a validator
    is_validator: bool,

    /// Current slot
    current_slot: u64,

    /// Last slot time
    last_slot_time: SystemTime,
}

impl Node {
    /// Create a new node
    pub fn new(config: Config, data_dir: PathBuf, rpc_port: u16) -> Self {
        // Generate identity if it doesn't exist
        let identity_path = data_dir.join("identity.json");
        let identity = if identity_path.exists() {
            match Keypair::load_from_file(&identity_path) {
                Ok(keypair) => {
                    info!("Loaded identity: {}", keypair.public_key());
                    keypair
                }
                Err(_) => {
                    warn!("Failed to load identity, generating new one");
                    let keypair = Keypair::random();
                    let _ = keypair.save_to_file(&identity_path);
                    keypair
                }
            }
        } else {
            info!("Generating new identity");
            let keypair = Keypair::random();
            let _ = keypair.save_to_file(&identity_path);
            keypair
        };

        Self {
            config,
            data_dir,
            rpc_port,
            identity,
            storage: None,
            tx_pool: None,
            poh: None,
            leader_schedule: None,
            program_registry: None,
            gossip_service: None,
            instruction_processor: None,
            is_validator: false,
            current_slot: 0,
            last_slot_time: SystemTime::now(),
        }
    }

    /// Start the node
    pub async fn start(&mut self) -> Result<()> {
        // Initialize storage
        let storage = BlockchainStorage::new(&self.data_dir)?;
        let state = storage.get_state();

        // Set current slot from state
        {
            let current_state = state.read().unwrap();
            self.current_slot = current_state.current_slot;
        }

        // Initialize transaction pool
        let tx_pool = Arc::new(TransactionPool::new(self.config.max_transactions_per_block));

        // Initialize PoH
        let poh = ProofOfHistory::new(self.config.block_time_target * 1_000_000); // Convert to nanoseconds

        // Initialize leader schedule
        let mut leader_schedule = SlotLeaderSchedule::new(100); // 100 slots per epoch

        // Add self as a validator
        let self_pubkey = self.identity.public_key();
        leader_schedule.add_validator(self_pubkey, 100); // Stake of 100

        // Initialize program registry
        let program_registry = ProgramRegistry::new();

        // Create channels for network communication
        let (message_tx, mut message_rx) = mpsc::channel(100);

        // Initialize gossip service
        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", self.config.p2p_port);
        let gossip_service =
            GossipService::new(&listen_addr, self.config.max_peers, message_tx.clone()).await?;

        // Initialize instruction processor
        let instruction_processor = InstructionProcessor::new(Arc::clone(&state));

        // Store initialized components
        self.storage = Some(storage);
        self.tx_pool = Some(tx_pool.clone());
        self.poh = Some(poh);
        self.leader_schedule = Some(leader_schedule);
        self.program_registry = Some(program_registry);
        self.gossip_service = Some(gossip_service);
        self.instruction_processor = Some(instruction_processor);

        // Connect to seed nodes
        for seed in &self.config.seed_nodes {
            if let Some(gossip) = &self.gossip_service {
                if let Err(e) = gossip.add_peer(seed).await {
                    warn!("Failed to connect to seed node {}: {}", seed, e);
                }
            }
        }

        // Decide if we're a validator
        self.is_validator = true; // For simplicity, all nodes are validators

        // Start RPC server
        self.start_rpc_server()?;

        // Start main loop
        self.main_loop().await?;

        Ok(())
    }

    /// Start the RPC server
    fn start_rpc_server(&self) -> Result<()> {
        // This would be a full implementation in a real system
        // For simplicity, we're just logging
        info!("Started RPC server on port {}", self.rpc_port);

        Ok(())
    }

    /// Main node loop
    async fn main_loop(&mut self) -> Result<()> {
        info!("Node started");

        // Get components
        let tx_pool = self
            .tx_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Transaction pool not initialized"))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| anyhow!("Storage not initialized"))?;
        let state = storage.get_state();

        // Initialize block generation ticker
        let mut slot_interval =
            time::interval(Duration::from_millis(self.config.block_time_target));

        // Main event loop
        loop {
            tokio::select! {
                // New slot tick
                _ = slot_interval.tick() => {
                    if self.is_validator {
                        self.try_produce_block().await?;
                    }
                }

                // Process other events
                // For example, handling network messages, RPC requests, etc.
            }
        }
    }

    /// Try to produce a block if we're the leader for this slot
    async fn try_produce_block(&mut self) -> Result<()> {
        // Increment slot
        self.current_slot += 1;
        self.last_slot_time = SystemTime::now();

        // Check if we're the leader for this slot
        let leader_schedule = self
            .leader_schedule
            .as_ref()
            .ok_or_else(|| anyhow!("Leader schedule not initialized"))?;
        let random_seed = Hash([0; 32]); // In a real implementation, this would be from previous blocks

        if let Some(leader) = leader_schedule.get_slot_leader(self.current_slot, random_seed) {
            let our_pubkey = self.identity.public_key();

            if leader == our_pubkey {
                info!("We are the leader for slot {}", self.current_slot);

                // Create a new block
                self.produce_block().await?;
            }
        }

        Ok(())
    }

    /// Produce a new block
    async fn produce_block(&mut self) -> Result<()> {
        // Get components
        let tx_pool = self
            .tx_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Transaction pool not initialized"))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| anyhow!("Storage not initialized"))?;
        let poh = self
            .poh
            .as_mut()
            .ok_or_else(|| anyhow!("PoH not initialized"))?;
        let gossip = self
            .gossip_service
            .as_ref()
            .ok_or_else(|| anyhow!("Gossip service not initialized"))?;

        // Generate PoH hash
        let poh_hash = poh.tick();

        // Get last block hash
        let parent_hash = storage.get_last_block_hash()?;

        // Get transactions from pool
        let transactions = tx_pool.select_transactions(self.config.max_transactions_per_block);

        // Get current state
        let state = storage.get_state();
        let state_root = {
            let state = state.read().unwrap();
            state.state_root()
        };

        // Create block
        let block = Block::new(
            self.current_slot,
            parent_hash,
            transactions,
            self.identity.public_key().to_bytes(),
            poh_hash,
            state_root,
        );

        // Store block
        storage.store_block(&block)?;

        // Broadcast block
        gossip.broadcast_block(block.clone()).await;

        info!(
            "Produced block at slot {}: {}",
            self.current_slot,
            block.hash()
        );

        Ok(())
    }
}
