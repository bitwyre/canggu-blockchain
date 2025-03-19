use crate::crypto::hash::{Hash, Hashable};
use crate::crypto::keys::{Keypair, PublicKey, Signature};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Instructions that can be executed in a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Transfer tokens from sender to recipient
    Transfer {
        /// Recipient public key
        to: PublicKey,
        /// Amount to transfer
        amount: u64,
    },

    /// Deploy a program (smart contract)
    DeployProgram {
        /// Program ID (hash of the code)
        program_id: Hash,
        /// Program bytecode (eBPF)
        code: Vec<u8>,
    },

    /// Call a program with data
    CallProgram {
        /// Program ID to call
        program_id: Hash,
        /// Input data for the program
        data: Vec<u8>,
    },
}

/// Transaction response after submission
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Transaction signature
    pub signature: String,
    /// Status of the transaction
    pub status: String,
}

/// A transaction on the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction nonce (to prevent replay attacks)
    pub nonce: u64,

    /// Sender's public key
    pub sender: PublicKey,

    /// Instruction to execute
    pub instruction: Instruction,

    /// Transaction fee
    pub fee: u64,

    /// Timestamp when transaction was created
    pub timestamp: u64,

    /// Transaction signature
    pub signature: Option<Signature>,
}

impl Transaction {
    /// Create a new unsigned transaction
    fn new(sender: PublicKey, instruction: Instruction, nonce: u64, fee: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            nonce,
            sender,
            instruction,
            fee,
            timestamp,
            signature: None,
        }
    }

    /// Create a new token transfer transaction
    pub fn new_transfer(keypair: &Keypair, to: &PublicKey, amount: u64) -> Self {
        let sender = keypair.public_key();
        let instruction = Instruction::Transfer {
            to: to.clone(),
            amount,
        };

        // In a real implementation, the nonce would be fetched from the account state
        let nonce = 0;
        let fee = 1; // Fixed fee for simplicity

        let mut tx = Self::new(sender, instruction, nonce, fee);
        tx.sign(keypair);
        tx
    }

    /// Create a new program deployment transaction
    pub fn new_deploy_program(keypair: &Keypair, code: Vec<u8>) -> Self {
        let sender = keypair.public_key();
        let program_id = Hash::hash(&code);

        let instruction = Instruction::DeployProgram { program_id, code };

        // In a real implementation, the nonce would be fetched from the account state
        let nonce = 0;
        let fee = 10; // Higher fee for program deployment

        let mut tx = Self::new(sender, instruction, nonce, fee);
        tx.sign(keypair);
        tx
    }

    /// Create a new program call transaction
    pub fn new_call_program(keypair: &Keypair, program_id: Hash, data: Vec<u8>) -> Self {
        let sender = keypair.public_key();
        let instruction = Instruction::CallProgram { program_id, data };

        // In a real implementation, the nonce would be fetched from the account state
        let nonce = 0;
        let fee = 2; // Medium fee for program calls

        let mut tx = Self::new(sender, instruction, nonce, fee);
        tx.sign(keypair);
        tx
    }

    /// Sign the transaction
    pub fn sign(&mut self, keypair: &Keypair) {
        // Create a copy without the signature
        let mut tx_copy = self.clone();
        tx_copy.signature = None;

        // Serialize and sign
        let message = bincode::serialize(&tx_copy).unwrap_or_default();
        let signature = keypair.sign(&message);

        self.signature = Some(signature);
    }

    /// Verify the transaction signature
    pub fn verify(&self) -> bool {
        if let Some(signature) = &self.signature {
            // Create a copy without the signature
            let mut tx_copy = self.clone();
            tx_copy.signature = None;

            // Serialize and verify
            let message = bincode::serialize(&tx_copy).unwrap_or_default();
            self.sender.verify(&message, signature)
        } else {
            false
        }
    }
}

impl Hashable for Transaction {
    fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).unwrap_or_default();
        Hash::hash(&encoded)
    }
}

/// Submit a transaction to the network
pub async fn submit_transaction(url: &str, tx: &Transaction) -> Result<TransactionResponse> {
    // Serialize transaction
    let tx_json = serde_json::to_string(tx)?;

    // In a real implementation, this would make an HTTP request to the node
    // For simplicity, we're just returning a success response
    Ok(TransactionResponse {
        signature: tx.hash().to_hex(),
        status: "pending".to_string(),
    })
}
