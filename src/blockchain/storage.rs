use crate::blockchain::block::Block;
use crate::blockchain::state::BlockchainState;
use crate::crypto::hash::Hash;
use anyhow::{Result, anyhow};
use sled::Db;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Storage for the blockchain data
pub struct BlockchainStorage {
    /// Database for storing blocks
    db: Db,

    /// Current blockchain state
    state: Arc<RwLock<BlockchainState>>,
}

impl BlockchainStorage {
    /// Create a new blockchain storage
    pub fn new(data_dir: &Path) -> Result<Self> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(data_dir)?;

        // Open database
        let db_path = data_dir.join("blockchain.db");
        let db = sled::open(db_path)?;

        // Initialize state
        let state = if let Some(state_bytes) = db.get("state")? {
            bincode::deserialize(&state_bytes)?
        } else {
            BlockchainState::new()
        };

        let state = Arc::new(RwLock::new(state));

        Ok(Self { db, state })
    }
    /// Get a block by its hash
    pub fn get_block(&self, hash: &Hash) -> Result<Option<Block>> {
        let key = format!("block:{}", hex::encode(hash.0));

        if let Some(block_bytes) = self.db.get(key)? {
            let block: Block = bincode::deserialize(&block_bytes)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    /// Get a block by its slot number
    pub fn get_block_by_slot(&self, slot: u64) -> Result<Option<Block>> {
        let key = format!("slot:{}", slot);

        if let Some(hash_bytes) = self.db.get(key)? {
            let hash: Hash = bincode::deserialize(&hash_bytes)?;
            match self.get_block(&hash)? {
                Some(block) => Ok(Some(block)),
                None => Err(anyhow!("Block hash exists but block not found")),
            }
        } else {
            Ok(None)
        }
    }

    /// Store a block
    pub fn store_block(&self, block: &Block) -> Result<()> {
        let hash = block.hash();
        let block_key = format!("block:{}", hex::encode(hash.0));
        let slot_key = format!("slot:{}", block.header.slot);

        // Serialize block
        let block_bytes = bincode::serialize(block)?;

        // Store mappings
        self.db.insert(block_key, block_bytes)?;
        self.db.insert(slot_key, bincode::serialize(&hash)?)?;

        // Update last block hash
        self.db
            .insert("last_block_hash", bincode::serialize(&hash)?)?;

        // Update state by applying the block
        {
            let mut state = self.state.write().unwrap();
            state.apply_block(block)?;

            // Store updated state
            let state_bytes = bincode::serialize(&*state)?;
            self.db.insert("state", state_bytes)?;
        }

        self.db.flush()?;
        Ok(())
    }

    /// Get the current blockchain state
    pub fn get_state(&self) -> Arc<RwLock<BlockchainState>> {
        Arc::clone(&self.state)
    }

    /// Get the last block hash
    pub fn get_last_block_hash(&self) -> Result<Hash> {
        if let Some(hash_bytes) = self.db.get("last_block_hash")? {
            let hash: Hash = bincode::deserialize(&hash_bytes)?;
            Ok(hash)
        } else {
            // Default hash for empty blockchain
            Ok(Hash([0; 32]))
        }
    }
}
