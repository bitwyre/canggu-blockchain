use crate::blockchain::block::Block;
use crate::crypto::hash::Hash;
use crate::transaction::tx::Transaction;
use serde::{Deserialize, Serialize};

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Ping message to check if peer is alive
    Ping {
        /// Nonce to identify the ping
        nonce: u64,
    },

    /// Pong response to a ping
    Pong {
        /// Nonce from the ping
        nonce: u64,
    },

    /// Transaction broadcast
    Transaction(Transaction),

    /// Block broadcast
    Block(Block),

    /// Request a specific block by hash
    GetBlock {
        /// Hash of the block to retrieve
        hash: Hash,
    },

    /// Request a specific block by slot
    GetBlockBySlot {
        /// Slot number
        slot: u64,
    },

    /// Request the latest block
    GetLatestBlock,

    /// Request information about a peer
    GetPeerInfo,

    /// Response with peer information
    PeerInfo {
        /// Node version
        version: String,

        /// Current blockchain height
        height: u64,

        /// Genesis block hash
        genesis_hash: Hash,
    },
}

/// Encode a message to bytes
pub fn encode_message(message: &Message) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

/// Decode a message from bytes
pub fn decode_message(bytes: &[u8]) -> Result<Message, bincode::Error> {
    bincode::deserialize(bytes)
}
