use crate::accounts::accounts::Account;
use crate::blockchain::state::BlockchainState;
use crate::crypto::hash::Hash;
use crate::crypto::keys::PublicKey;
use crate::runtime::vm::{ExecutionContext, ExecutionResult, VirtualMachine};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Type for program IDs
pub type ProgramId = Hash;

/// Information about a program
#[derive(Debug, Clone)]
pub struct Program {
    /// Program ID (hash of the code)
    pub id: ProgramId,

    /// Program bytecode (eBPF)
    pub code: Vec<u8>,

    /// Program owner
    pub owner: PublicKey,
}

/// Registry for programs
pub struct ProgramRegistry {
    /// Programs by ID
    programs: Arc<RwLock<HashMap<ProgramId, Program>>>,
}

impl ProgramRegistry {
    /// Create a new program registry
    pub fn new() -> Self {
        Self {
            programs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a program
    pub fn register_program(&self, program: Program) -> Result<()> {
        let mut programs = self.programs.write().unwrap();
        programs.insert(program.id, program);
        Ok(())
    }

    /// Get a program by ID
    pub fn get_program(&self, id: &ProgramId) -> Option<Program> {
        let programs = self.programs.read().unwrap();
        programs.get(id).cloned()
    }

    /// Remove a program
    pub fn remove_program(&self, id: &ProgramId) -> Result<()> {
        let mut programs = self.programs.write().unwrap();
        programs.remove(id);
        Ok(())
    }

    /// Check if a program exists
    pub fn program_exists(&self, id: &ProgramId) -> bool {
        let programs = self.programs.read().unwrap();
        programs.contains_key(id)
    }

    /// Get all programs
    pub fn get_all_programs(&self) -> Vec<Program> {
        let programs = self.programs.read().unwrap();
        programs.values().cloned().collect()
    }
}

/// Program manager for handling program execution
pub struct ProgramManager {
    /// Virtual machine for executing programs
    vm: VirtualMachine,
}

impl ProgramManager {
    /// Create a new program manager
    pub fn new(max_instructions: u64, gas_limit: u64) -> Self {
        Self {
            vm: VirtualMachine::new(max_instructions, gas_limit),
        }
    }

    /// Execute a program
    // pub fn execute_program(
    //     &self,
    //     program_id: Hash,
    //     program_code: &[u8],
    //     caller: PublicKey,
    //     data: Vec<u8>,
    //     state: &mut BlockchainState,
    //     account_keys: &[PublicKey],
    // ) -> Result<ExecutionResult> {
    //     // Prepare accounts for the program to access
    //     let mut accounts = HashMap::new();

    //     let mut account_refs: Vec<(&PublicKey, &mut Account)> = account_keys
    //         .iter()
    //         .filter_map(|key| state.accounts.get_mut(key).map(|account| (key, account)))
    //         .collect();
    // TODO: Fix account modification
    // for (key, account) in account_refs {
    //     accounts.insert(key.clone(), account);
    //     // Create execution context
    //     let mut context = ExecutionContext {
    //         program_id,
    //         caller,
    //         data,
    //         accounts,
    //     };

    //     // Execute the program
    //     self.vm.execute(program_code, &mut context)
    // }
    // }

    /// Deploy a new program
    pub fn deploy_program(
        &self,
        program_id: Hash,
        program_code: Vec<u8>,
        owner: PublicKey,
        state: &mut BlockchainState,
    ) -> Result<()> {
        // Validate program code
        if program_code.is_empty() {
            return Err(anyhow!("Empty program code"));
        }

        // Store program in state
        state.programs.insert(
            program_id,
            crate::blockchain::state::ProgramInfo {
                code: program_code,
                owner,
                deployment_slot: state.current_slot + 1,
            },
        );

        Ok(())
    }
}
