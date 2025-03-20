use crate::crypto::hash::{Hash, Hashable};
use crate::transaction::tx::Transaction;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// A pool of pending transactions
pub struct TransactionPool {
    /// Transactions indexed by their hash
    transactions: Arc<Mutex<HashMap<Hash, Transaction>>>,

    /// Transaction hashes by sender (for quickly finding a sender's transactions)
    sender_txs: Arc<Mutex<HashMap<Vec<u8>, HashSet<Hash>>>>,

    /// Maximum number of transactions the pool can hold
    max_size: usize,
}

impl TransactionPool {
    /// Create a new transaction pool
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: Arc::new(Mutex::new(HashMap::new())),
            sender_txs: Arc::new(Mutex::new(HashMap::new())),
            max_size,
        }
    }

    /// Add a transaction to the pool
    pub fn add_transaction(&self, tx: Transaction) -> Result<Hash> {
        // Verify the transaction
        if !tx.verify() {
            return Err(anyhow::anyhow!("Invalid transaction signature"));
        }

        let tx_hash = tx.hash();
        let sender_bytes = tx.sender.to_bytes().to_vec();

        // Add to pool
        {
            let mut txs = self.transactions.lock().unwrap();

            // Check if pool is full
            if txs.len() >= self.max_size {
                return Err(anyhow::anyhow!("Transaction pool is full"));
            }

            // Add transaction
            txs.insert(tx_hash, tx);
        }

        // Add to sender index
        {
            let mut sender_txs = self.sender_txs.lock().unwrap();
            sender_txs
                .entry(sender_bytes)
                .or_insert_with(HashSet::new)
                .insert(tx_hash);
        }

        Ok(tx_hash)
    }

    /// Get a transaction by its hash
    pub fn get_transaction(&self, tx_hash: &Hash) -> Option<Transaction> {
        let txs = self.transactions.lock().unwrap();
        txs.get(tx_hash).cloned()
    }

    /// Remove a transaction from the pool
    pub fn remove_transaction(&self, tx_hash: &Hash) -> Option<Transaction> {
        // Remove from transactions
        let tx = {
            let mut txs = self.transactions.lock().unwrap();
            txs.remove(tx_hash)
        };

        // Remove from sender index
        if let Some(tx) = &tx {
            let sender_bytes = tx.sender.to_bytes().to_vec();
            let mut sender_txs = self.sender_txs.lock().unwrap();

            if let Some(hashes) = sender_txs.get_mut(&sender_bytes) {
                hashes.remove(tx_hash);

                // If this was the last transaction for this sender, remove the entry
                if hashes.is_empty() {
                    sender_txs.remove(&sender_bytes);
                }
            }
        }

        tx
    }

    /// Get all transactions in the pool
    pub fn get_all_transactions(&self) -> Vec<Transaction> {
        let txs = self.transactions.lock().unwrap();
        txs.values().cloned().collect()
    }

    /// Get all transactions from a specific sender
    pub fn get_sender_transactions(&self, sender_bytes: &[u8]) -> Vec<Transaction> {
        let sender_txs = self.sender_txs.lock().unwrap();
        let txs = self.transactions.lock().unwrap();

        if let Some(hashes) = sender_txs.get(sender_bytes) {
            hashes
                .iter()
                .filter_map(|hash| txs.get(hash).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get the number of transactions in the pool
    pub fn size(&self) -> usize {
        let txs = self.transactions.lock().unwrap();
        txs.len()
    }

    /// Clear all transactions from the pool
    pub fn clear(&self) {
        let mut txs = self.transactions.lock().unwrap();
        let mut sender_txs = self.sender_txs.lock().unwrap();

        txs.clear();
        sender_txs.clear();
    }

    /// Select transactions for inclusion in a block
    pub fn select_transactions(&self, max_count: usize) -> Vec<Transaction> {
        let txs = self.transactions.lock().unwrap();

        // In a real implementation, you would prioritize transactions by fee
        txs.values().take(max_count).cloned().collect()
    }
}
