use crate::crypto::hash::Hash;
use crate::crypto::keys::PublicKey;
use anyhow::Result;
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
