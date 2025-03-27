use crate::crypto::hash::{Hash, Hashable};
use crate::crypto::keys::{Keypair, PublicKey, Signature};
use anyhow::{anyhow, Result};
use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};

/// Transaction instruction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Transfer tokens from sender to recipient
    Transfer {
        /// Recipient address
        to: PublicKey,

        /// Amount to transfer
        amount: u64,
    },

    /// Deploy a program (smart contract)
    DeployProgram {
        /// Program ID (hash of the program)
        program_id: Hash,

        /// Program bytecode
        code: Vec<u8>,
    },

    /// Call a deployed program
    CallProgram {
        /// Program ID to call
        program_id: Hash,

        /// Call data
        data: Vec<u8>,
    },
}

/// A transaction on the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction sender
    pub sender: PublicKey,

    /// Transaction nonce (to prevent replay attacks)
    pub nonce: u64,

    /// Transaction instruction
    pub instruction: Instruction,

    /// Transaction signature
    pub signature: Option<Signature>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(sender: PublicKey, nonce: u64, instruction: Instruction) -> Self {
        Self {
            sender,
            nonce,
            instruction,
            signature: None,
        }
    }

    /// Create a new transfer transaction
    pub fn new_transfer(from: &Keypair, to: &PublicKey, amount: u64) -> Self {
        let mut tx = Self::new(
            from.public(),
            0, // In a real implementation, we would get the current nonce
            Instruction::Transfer {
                to: to.clone(),
                amount,
            },
        );

        // Sign the transaction
        tx.sign(from);

        tx
    }

    /// Create a new program deployment transaction
    pub fn new_deploy_program(from: &Keypair, code: Vec<u8>) -> Self {
        // Calculate program ID (hash of code)
        let program_id = Hash::hash(&code);

        let mut tx = Self::new(
            from.public(),
            0, // In a real implementation, we would get the current nonce
            Instruction::DeployProgram { program_id, code },
        );

        // Sign the transaction
        tx.sign(from);

        tx
    }

    /// Create a new program call transaction
    pub fn new_call_program(from: &Keypair, program_id: Hash, data: Vec<u8>) -> Self {
        let mut tx = Self::new(
            from.public(),
            0, // In a real implementation, we would get the current nonce
            Instruction::CallProgram { program_id, data },
        );

        // Sign the transaction
        tx.sign(from);

        tx
    }

    /// Sign the transaction
    pub fn sign(&mut self, keypair: &Keypair) {
        // Create message to sign
        let message = self.message_for_signing();

        // Sign message
        let signature = keypair.sign(&message);

        // Set signature
        self.signature = Some(signature);
    }

    /// Verify the transaction signature
    pub fn verify_signature(&self) -> bool {
        if let Some(signature) = &self.signature {
            let message = self.message_for_signing();
            self.sender.verify(&message, signature)
        } else {
            false
        }
    }

    /// Get the message that should be signed
    fn message_for_signing(&self) -> Vec<u8> {
        // Create a copy without the signature
        let unsigned = Self {
            sender: self.sender.clone(),
            nonce: self.nonce,
            instruction: self.instruction.clone(),
            signature: None,
        };

        // Serialize the unsigned transaction
        bincode::serialize(&unsigned).unwrap_or_default()
    }

    /// Submit a transaction to the blockchain
    pub async fn submit_transaction(url: &str, tx: &Transaction) -> Result<TransactionResponse> {
        let client = reqwest::Client::new();

        // Serialize transaction
        let tx_data = bincode::serialize(tx)?;
        let tx_base64 = base64::encode(&tx_data);

        // Create request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [tx_base64]
        });

        // Send request
        let response = client.post(url).json(&request).send().await?;

        // Parse response
        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            return Err(anyhow!("RPC error: {}", error));
        }

        if let Some(result) = json.get("result") {
            let signature = result
                .as_str()
                .ok_or_else(|| anyhow!("Invalid signature in response"))?
                .to_string();

            Ok(TransactionResponse { signature })
        } else {
            Err(anyhow!("Invalid RPC response"))
        }
    }

    /// Add this method
    pub fn hash(&self) -> Hash {
        // Create a hash of the transaction
        let mut data = Vec::new();
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.sender.to_bytes());
        match &self.instruction {
            Instruction::Transfer { to, amount } => {
                data.extend_from_slice(&to.to_bytes());
                data.extend_from_slice(&amount.to_le_bytes());

                println!("Transfer to: {:?}, amount: {}", to, amount);
            }
            _ => {
                // Handle other variants (DeployProgram, CallProgram)
                println!("Not a transfer instruction");
            }
        }

        Hash::hash(&data)
    }

    /// Add verify method
    pub fn verify(&self) -> bool {
        if let Some(signature) = &self.signature {
            // Verify the signature
            // Implementation depends on your crypto library
            true // Placeholder
        } else {
            false
        }
    }
}

/// Response from submitting a transaction
pub struct TransactionResponse {
    /// Transaction signature (ID)
    pub signature: String,
}

impl Hashable for Transaction {
    fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).unwrap_or_default();
        Hash::hash(&encoded)
    }
}
