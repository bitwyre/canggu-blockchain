use crate::blockchain::block::{Block, BlockHeader};
use crate::blockchain::state::BlockchainState;
use crate::blockchain::storage::BlockchainStorage;
use crate::consensus::proof_of_history::ProofOfHistory;
use crate::crypto::hash::Hash;
use crate::crypto::keys::PublicKey;
use crate::transaction::mempool::TransactionPool;
use crate::transaction::tx::Transaction;
use anyhow::{Result, anyhow};
use log::{error, info, warn};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Main blockchain engine
pub struct Blockchain {
    /// Storage for blocks and state
    storage: Arc<BlockchainStorage>,

    /// Current blockchain state
    state: Arc<RwLock<BlockchainState>>,

    /// Transaction pool
    mempool: Arc<RwLock<TransactionPool>>,

    /// Proof of History generator
    poh: Arc<RwLock<ProofOfHistory>>,

    /// Current validator identity
    validator_key: Option<PublicKey>,

    /// Channel for newly produced blocks
    block_tx: mpsc::Sender<Block>,
}

impl Blockchain {
    /// Create a new blockchain instance
    pub fn new(
        storage: Arc<BlockchainStorage>,
        mempool: Arc<RwLock<TransactionPool>>,
        poh: Arc<RwLock<ProofOfHistory>>,
        validator_key: Option<PublicKey>,
    ) -> Result<(Self, mpsc::Receiver<Block>)> {
        let state = storage.get_state();
        let (block_tx, block_rx) = mpsc::channel(100);

        let blockchain = Self {
            storage,
            state,
            mempool,
            poh,
            validator_key,
            block_tx,
        };

        Ok((blockchain, block_rx))
    }

    /// Start the blockchain engine
    pub async fn start(&self, block_time_ms: u64) -> Result<()> {
        // Start block production if we're a validator
        if self.validator_key.is_some() {
            let block_tx = self.block_tx.clone();
            let mempool = self.mempool.clone();
            let state = self.state.clone();
            let poh = self.poh.clone();
            let storage = self.storage.clone();
            let validator_key = self.validator_key.clone().unwrap();

            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_millis(block_time_ms));

                loop {
                    interval.tick().await;

                    // Try to produce a block
                    match Self::produce_block(&mempool, &state, &poh, &storage, &validator_key)
                        .await
                    {
                        Ok(block) => {
                            if let Err(e) = block_tx.send(block.clone()).await {
                                error!("Failed to send produced block: {}", e);
                            }

                            // Apply block to our own state
                            if let Err(e) = storage.store_block(&block) {
                                error!("Failed to store produced block: {}", e);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to produce block: {}", e);
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Process a new block received from the network
    pub async fn process_block(&self, block: Block) -> Result<()> {
        // Verify block
        self.verify_block(&block)?;

        // Store block
        self.storage.store_block(&block)?;

        // Remove included transactions from mempool
        {
            let mut mempool = self.mempool.write().unwrap();
            for tx in &block.transactions {
                mempool.remove_transaction(&tx.hash());
            }
        }

        Ok(())
    }

    /// Verify a block
    fn verify_block(&self, block: &Block) -> Result<()> {
        // Verify block structure
        if block.transactions.is_empty() {
            return Err(anyhow!("Block has no transactions"));
        }

        // Verify block header
        let state = self.state.read().unwrap();

        // Check slot number
        if block.header.slot != state.current_slot + 1 {
            return Err(anyhow!(
                "Invalid block slot: expected {}, got {}",
                state.current_slot + 1,
                block.header.slot
            ));
        }

        // Check parent hash
        if block.header.parent_hash != state.last_block_hash {
            return Err(anyhow!("Invalid parent hash"));
        }

        // Verify transactions
        for tx in &block.transactions {
            self.verify_transaction(tx)?;
        }

        Ok(())
    }

    /// Verify a transaction
    fn verify_transaction(&self, tx: &Transaction) -> Result<()> {
        // Verify signature
        if !tx.verify_signature() {
            return Err(anyhow!("Invalid transaction signature"));
        }

        // Verify sender has enough balance (for transfers)
        if let crate::transaction::tx::Instruction::Transfer { amount, .. } = &tx.instruction {
            let state = self.state.read().unwrap();
            if let Some(account) = state.accounts.get(&tx.sender) {
                if account.balance < *amount {
                    return Err(anyhow!("Insufficient balance"));
                }
            } else {
                return Err(anyhow!("Sender account not found"));
            }
        }

        Ok(())
    }

    /// Produce a new block
    async fn produce_block(
        mempool: &Arc<RwLock<TransactionPool>>,
        state: &Arc<RwLock<BlockchainState>>,
        poh: &Arc<RwLock<ProofOfHistory>>,
        storage: &Arc<BlockchainStorage>,
        validator_key: &PublicKey,
    ) -> Result<Block> {
        // Get current state
        let current_state = state.read().unwrap();
        let next_slot = current_state.current_slot + 1;
        let parent_hash = current_state.last_block_hash;

        // Get transactions from mempool
        let transactions = {
            let mempool = mempool.read().unwrap();
            mempool.get_transactions(1000) // Get up to 1000 transactions
        };

        // Generate PoH hash
        let poh_hash = {
            let mut poh = poh.write().unwrap();
            poh.tick()
        };

        // Calculate state root after applying transactions
        let mut temp_state = current_state.clone();
        for tx in &transactions {
            if let Err(e) = temp_state.apply_transaction(tx) {
                warn!("Failed to apply transaction: {}", e);
                // In a real implementation, we would exclude this transaction
            }
        }
        let state_root = temp_state.state_root();

        // Create block
        let block = Block::new(
            next_slot,
            parent_hash,
            transactions,
            validator_key.to_bytes().to_vec(),
            poh_hash,
            state_root,
        );

        Ok(block)
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&self, tx: Transaction) -> Result<Hash> {
        // Verify transaction
        self.verify_transaction(&tx)?;

        // Add to mempool
        let mut mempool = self.mempool.write().unwrap();
        mempool.add_transaction(tx)
    }

    /// Get the current blockchain height
    pub fn height(&self) -> u64 {
        let state = self.state.read().unwrap();
        state.current_slot
    }

    /// Get the latest block hash
    pub fn latest_block_hash(&self) -> Hash {
        let state = self.state.read().unwrap();
        state.last_block_hash
    }
}
