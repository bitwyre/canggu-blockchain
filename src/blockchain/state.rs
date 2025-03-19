use crate::blockchain::block::Block;
use crate::crypto::hash::{Hash, Hashable};
use crate::crypto::keys::PublicKey;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The state of the blockchain
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockchainState {
    /// Current account balances
    pub accounts: HashMap<PublicKey, AccountInfo>,

    /// Deployed programs (smart contracts)
    pub programs: HashMap<Hash, ProgramInfo>,

    /// Current slot/height of the blockchain
    pub current_slot: u64,

    /// Hash of the last block
    pub last_block_hash: Hash,
}

/// Information about an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account balance
    pub balance: u64,

    /// Account nonce (for preventing replay attacks)
    pub nonce: u64,

    /// Account data (optional)
    pub data: Vec<u8>,

    /// Owner program (if this is a program-controlled account)
    pub owner: Option<Hash>,
}

/// Information about a deployed program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    /// Program code (eBPF bytecode)
    pub code: Vec<u8>,

    /// Program owner
    pub owner: PublicKey,

    /// Deployment slot
    pub deployment_slot: u64,
}

impl BlockchainState {
    /// Create a new blockchain state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            programs: HashMap::new(),
            current_slot: 0,
            last_block_hash: Hash([0; 32]),
        }
    }

    /// Apply a block to the current state
    pub fn apply_block(&mut self, block: &Block) -> Result<()> {
        // Ensure block is for the next slot
        if block.header.slot != self.current_slot + 1 {
            return Err(anyhow!(
                "Invalid block slot: expected {}, got {}",
                self.current_slot + 1,
                block.header.slot
            ));
        }

        // Ensure parent hash matches our last block hash
        if block.header.parent_hash != self.last_block_hash {
            return Err(anyhow!("Invalid parent hash"));
        }

        // Process each transaction in the block
        for tx in &block.transactions {
            self.apply_transaction(tx)?;
        }

        // Update state
        self.current_slot = block.header.slot;
        self.last_block_hash = block.hash();

        Ok(())
    }

    /// Apply a single transaction to the state
    fn apply_transaction(&mut self, tx: &crate::transaction::tx::Transaction) -> Result<()> {
        // This is simplified. In a real implementation, you would:
        // 1. Verify transaction signature
        // 2. Check nonce to prevent replay
        // 3. Check balance for transfers
        // 4. Execute program instructions

        match &tx.instruction {
            crate::transaction::tx::Instruction::Transfer { to, amount } => {
                let sender = tx.sender.clone();

                // Get sender account or create a new one (for simplicity)
                let sender_account =
                    self.accounts
                        .entry(sender.clone())
                        .or_insert_with(|| AccountInfo {
                            balance: 0,
                            nonce: 0,
                            data: Vec::new(),
                            owner: None,
                        });

                // Check balance
                if sender_account.balance < *amount {
                    return Err(anyhow!("Insufficient balance"));
                }

                // Update sender balance
                sender_account.balance -= amount;
                sender_account.nonce += 1;

                // Update recipient balance
                let recipient_account =
                    self.accounts
                        .entry(to.clone())
                        .or_insert_with(|| AccountInfo {
                            balance: 0,
                            nonce: 0,
                            data: Vec::new(),
                            owner: None,
                        });

                recipient_account.balance += amount;
            }

            crate::transaction::tx::Instruction::DeployProgram { program_id, code } => {
                let owner = tx.sender.clone();

                // Store program
                self.programs.insert(
                    *program_id,
                    ProgramInfo {
                        code: code.clone(),
                        owner,
                        deployment_slot: self.current_slot + 1,
                    },
                );
            }

            crate::transaction::tx::Instruction::CallProgram { program_id, data } => {
                // In a real implementation, this would execute the eBPF program
                // For simplicity, we're not implementing the VM execution here
                if !self.programs.contains_key(program_id) {
                    return Err(anyhow!("Program not found"));
                }

                // TODO: Execute program using eBPF VM
            }
        }

        Ok(())
    }

    /// Calculate the state root hash
    pub fn state_root(&self) -> Hash {
        let encoded = bincode::serialize(self).unwrap_or_default();
        Hash::hash(&encoded)
    }
}
