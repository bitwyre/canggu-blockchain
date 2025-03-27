use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use anyhow::{Result, anyhow};

/// A public key (wrapper around a byte array)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    /// Create a new public key from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Create a public key from a slice
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(anyhow!("Invalid public key length"));
        }
        
        let mut result = [0u8; 32];
        result.copy_from_slice(bytes);
        
        Ok(Self(result))
    }
    
    /// Get the bytes of the public key
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
    
    /// Convert to a base58 string
    pub fn to_string(&self) -> String {
        bs58::encode(&self.0).into_string()
    }
    
    /// Create from a base58 string
    pub fn from_string(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| anyhow!("Invalid base58 string: {}", e))?;
            
        Self::from_bytes(&bytes)
    }
}

impl FromStr for Pubkey {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s)
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Default for Pubkey {
    fn default() -> Self {
        Self([0; 32])
    }
}