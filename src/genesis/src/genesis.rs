use serde::{Deserialize, Serialize};

/// Genesis configuration for the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Initial accounts and their balances
    pub accounts: Vec<GenesisAccount>,
    
    /// Initial validators
    pub validators: Vec<GenesisValidator>,
    
    /// Genesis timestamp
    pub timestamp: u64,
    
    /// Network ID
    pub network_id: String,
}

/// Initial account in genesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    /// Account public key
    pub pubkey: String,
    
    /// Initial balance
    pub balance: u64,
    
    /// Is this account a program
    pub is_program: bool,
    
    /// Program data (if is_program is true)
    pub program_data: Option<Vec<u8>>,
}

/// Initial validator in genesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisValidator {
    /// Validator public key
    pub pubkey: String,
    
    /// Initial stake
    pub stake: u64,
}

impl GenesisConfig {
    /// Create a new genesis configuration
    pub fn new(
        accounts: Vec<GenesisAccount>,
        validators: Vec<GenesisValidator>,
        timestamp: u64,
        network_id: String,
    ) -> Self {
        Self {
            accounts,
            validators,
            timestamp,
            network_id,
        }
    }
    
    /// Create a minimal genesis configuration for testing
    pub fn minimal() -> Self {
        Self {
            accounts: Vec::new(),
            validators: Vec::new(),
            timestamp: 0,
            network_id: "devnet".to_string(),
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
} 