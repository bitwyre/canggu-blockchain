use crate::crypto::hash::Hash;
use crate::transaction::tx::Transaction;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Transaction pool (mempool) for storing pending transactions
pub struct TransactionPool {
    /// Pending transactions by hash
    transactions: HashMap<Hash, Transaction>,

    /// Set of transaction hashes ordered by fee/priority
    priority_queue: Vec<Hash>,

    /// Maximum number of transactions in the pool
    max_size: usize,
}

impl TransactionPool {
    /// Create a new transaction pool
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: HashMap::new(),
            priority_queue: Vec::new(),
            max_size,
        }
    }

    /// Add a transaction to the pool
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<Hash> {
        let tx_hash = tx.hash();

        // Check if transaction already exists
        if self.transactions.contains_key(&tx_hash) {
            return Ok(tx_hash);
        }

        // Check if pool is full
        if self.transactions.len() >= self.max_size {
            self.remove_lowest_priority_tx()?;
        }

        // Add transaction
        self.transactions.insert(tx_hash, tx);

        // Add to priority queue (simple implementation - would be more complex in production)
        self.priority_queue.push(tx_hash);

        Ok(tx_hash)
    }

    /// Get transactions for inclusion in a block
    pub fn get_transactions(&self, max_count: usize) -> Vec<Transaction> {
        let mut result = Vec::new();

        for hash in &self.priority_queue {
            if result.len() >= max_count {
                break;
            }

            if let Some(tx) = self.transactions.get(hash) {
                result.push(tx.clone());
            }
        }

        result
    }

    /// Remove a transaction from the pool
    pub fn remove_transaction(&mut self, hash: &Hash) {
        self.transactions.remove(hash);
        self.priority_queue.retain(|h| h != hash);
    }

    /// Remove lowest priority transaction
    fn remove_lowest_priority_tx(&mut self) -> Result<()> {
        if let Some(hash) = self.priority_queue.pop() {
            self.transactions.remove(&hash);
            Ok(())
        } else {
            Ok(()) // Pool is empty
        }
    }

    /// Get the number of transactions in the pool
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
}
