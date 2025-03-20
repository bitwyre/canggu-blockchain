use crate::blockchain::block::Block;
use crate::crypto::hash::{Hash, Hashable};
use crate::transaction::tx::Transaction;
use serde::{Deserialize, Serialize};
use std::fmt;

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

/// Custom error type for message encoding/decoding
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

/// Encode a message to bytes
pub fn encode_message(message: &Message) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

/// Decode a message from bytes
pub fn decode_message(bytes: &[u8]) -> Result<Message, bincode::Error> {
    bincode::deserialize(bytes)
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Ping { nonce } => write!(f, "Ping({})", nonce),
            Message::Pong { nonce } => write!(f, "Pong({})", nonce),
            Message::Transaction(tx) => write!(f, "Transaction(hash={})", tx.hash()),
            Message::Block(block) => write!(
                f,
                "Block(slot={}, hash={})",
                block.header.slot,
                block.hash()
            ),
            Message::GetBlock { hash } => write!(f, "GetBlock({})", hash),
            Message::GetBlockBySlot { slot } => write!(f, "GetBlockBySlot({})", slot),
            Message::GetLatestBlock => write!(f, "GetLatestBlock"),
            Message::GetPeerInfo => write!(f, "GetPeerInfo"),
            Message::PeerInfo {
                version,
                height,
                genesis_hash,
            } => {
                write!(
                    f,
                    "PeerInfo(version={}, height={}, genesis={})",
                    version, height, genesis_hash
                )
            }
        }
    }
}
