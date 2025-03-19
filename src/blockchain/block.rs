use crate::crypto::hash::{Hash, Hashable};
use crate::transaction::tx::Transaction;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A block in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header containing metadata
    pub header: BlockHeader,

    /// List of transactions included in the block
    pub transactions: Vec<Transaction>,
}

/// Header of a block containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block height/slot number
    pub slot: u64,

    /// Hash of the parent block
    pub parent_hash: Hash,

    /// Timestamp when the block was created
    pub timestamp: u64,

    /// Merkle root of transactions
    pub transactions_root: Hash,

    /// Public key of the validator that produced this block
    pub validator: Vec<u8>,

    /// Proof of History (PoH) hash
    pub poh_hash: Hash,

    /// Hash of the current blockchain state after applying this block
    pub state_root: Hash,
}

impl Block {
    /// Create a new block
    pub fn new(
        slot: u64,
        parent_hash: Hash,
        transactions: Vec<Transaction>,
        validator: Vec<u8>,
        poh_hash: Hash,
        state_root: Hash,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate transactions merkle root
        let transactions_root = Self::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            slot,
            parent_hash,
            timestamp,
            transactions_root,
            validator,
            poh_hash,
            state_root,
        };

        Self {
            header,
            transactions,
        }
    }

    /// Calculate merkle root of transactions
    fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        // This is a simplified implementation
        // In a real blockchain, you'd implement a proper Merkle tree
        let mut combined = Vec::new();
        for tx in transactions {
            let tx_hash = tx.hash();
            combined.extend_from_slice(&tx_hash.0);
        }

        // If there are no transactions, use a default hash
        if combined.is_empty() {
            return Hash([0; 32]);
        }

        Hash::hash(&combined)
    }

    /// Get the block's hash
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }
}

impl Hashable for BlockHeader {
    fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).unwrap_or_default();
        Hash::hash(&encoded)
    }
}

/// Genesis block for initializing the blockchain
pub fn create_genesis_block() -> Block {
    let transactions = Vec::new();
    let validator = Vec::new(); // No validator for genesis
    let poh_hash = Hash([0; 32]);
    let state_root = Hash([0; 32]);

    Block::new(
        0,             // slot 0
        Hash([0; 32]), // parent hash (all zeros for genesis)
        transactions,
        validator,
        poh_hash,
        state_root,
    )
}
