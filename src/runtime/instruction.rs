use crate::blockchain::state::BlockchainState;
use crate::crypto::keys::PublicKey;
use crate::runtime::program::ProgramId;
use crate::runtime::vm::VirtualMachine;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
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
        let vm = VirtualMachine::new(Arc::clone(&state), 1_000_000); // Default gas limit

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
        let result = self
            .vm
            .execute(&instruction.program_id, &instruction.data)?;

        // Return program's return value
        Ok(result.return_value)
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
    let system_program_id = ProgramId([1; 32]);

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
