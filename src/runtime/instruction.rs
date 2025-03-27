use crate::blockchain::state::BlockchainState;
use crate::crypto::hash::Hash;
use crate::crypto::keys::PublicKey;
use crate::runtime::program::ProgramId;
use crate::runtime::vm::{ExecutionContext, VirtualMachine};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Instruction data that can be passed to programs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionData {
    /// Program to call
    pub program_id: ProgramId,

    /// Accounts to pass to the program
    pub accounts: Vec<AccountMeta>,

    /// Raw instruction data
    pub data: Vec<u8>,
}

/// Account metadata for instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    /// Account public key
    pub pubkey: PublicKey,

    /// Whether the account is a signer
    pub is_signer: bool,

    /// Whether the account can be written to
    pub is_writable: bool,
}

/// Instruction processor
pub struct InstructionProcessor {
    /// Current blockchain state
    state: Arc<RwLock<BlockchainState>>,

    /// Virtual machine for executing programs
    vm: VirtualMachine,
}

impl InstructionProcessor {
    /// Create a new instruction processor
    pub fn new(state: Arc<RwLock<BlockchainState>>) -> Self {
        let vm = VirtualMachine::new(1_000_000, 1_000_000); // max_instructions, gas_limit

        Self { state, vm }
    }

    /// Process an instruction
    pub fn process_instruction(
        &self,
        instruction: &InstructionData,
        signers: &[PublicKey],
    ) -> Result<u64> {
        // Verify signers
        for account in &instruction.accounts {
            if account.is_signer && !signers.contains(&account.pubkey) {
                return Err(anyhow!(
                    "Missing required signature for account: {}",
                    account.pubkey
                ));
            }
        }

        // Check program exists
        {
            let state = self.state.read().unwrap();
            if !state.programs.contains_key(&instruction.program_id) {
                return Err(anyhow!("Program not found: {}", instruction.program_id));
            }
        }

        // Execute program
        let result = self.vm.execute(
            &instruction.data,
            &mut ExecutionContext {
                program_id: instruction.program_id,
                caller: signers[0].clone(), // Assuming first signer is caller
                data: instruction.data.clone(),
                accounts: HashMap::new(), // You'll need to populate this
            },
        )?;

        // Return data instead of return_value
        Ok(result.data[0] as u64) // Or however you want to interpret the result
    }

    /// Process multiple instructions in a transaction
    pub fn process_instructions(
        &self,
        instructions: &[InstructionData],
        signers: &[PublicKey],
    ) -> Result<()> {
        for instruction in instructions {
            self.process_instruction(instruction, signers)?;
        }

        Ok(())
    }
}

/// Create a simple transfer instruction
pub fn create_transfer_instruction(
    from: &PublicKey,
    to: &PublicKey,
    amount: u64,
) -> InstructionData {
    // In a real implementation, this would use the system program
    // For simplicity, we're using a placeholder program ID
    let system_program_id = ProgramId::new([1; 32]);

    // Create account metadata
    let accounts = vec![
        AccountMeta {
            pubkey: from.clone(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: to.clone(),
            is_signer: false,
            is_writable: true,
        },
    ];

    // Create instruction data (amount as bytes)
    let mut data = Vec::with_capacity(9);
    data.push(0); // Instruction type (0 = transfer)
    data.extend_from_slice(&amount.to_le_bytes());

    InstructionData {
        program_id: system_program_id,
        accounts,
        data,
    }
}

/// Program instruction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Transfer tokens from one account to another
    Transfer {
        /// Source account
        from: PublicKey,

        /// Destination account
        to: PublicKey,

        /// Amount to transfer
        amount: u64,
    },

    /// Create a new account
    CreateAccount {
        /// Owner of the new account
        owner: PublicKey,

        /// Initial balance
        balance: u64,

        /// Space to allocate for account data
        space: u64,
    },

    /// Call a program
    CallProgram {
        /// Program ID to call
        program_id: Hash,

        /// Accounts to pass to the program
        accounts: Vec<PublicKey>,

        /// Data to pass to the program
        data: Vec<u8>,
    },

    /// Deploy a program
    DeployProgram {
        /// Program ID (hash of the code)
        program_id: Hash,

        /// Program code
        code: Vec<u8>,
    },
}

impl Instruction {
    /// Get the program ID for this instruction
    pub fn program_id(&self) -> Hash {
        match self {
            Self::Transfer { .. } => Hash::default(), // System program
            Self::CreateAccount { .. } => Hash::default(), // System program
            Self::CallProgram { program_id, .. } => *program_id,
            Self::DeployProgram { .. } => Hash::default(), // Loader program
        }
    }

    /// Get the accounts required for this instruction
    pub fn accounts(&self) -> Vec<PublicKey> {
        match self {
            Self::Transfer { from, to, .. } => vec![from.clone(), to.clone()],
            Self::CreateAccount { owner, .. } => vec![owner.clone()],
            Self::CallProgram { accounts, .. } => accounts.clone(),
            Self::DeployProgram { .. } => Vec::new(),
        }
    }

    /// Serialize the instruction to bytes
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
