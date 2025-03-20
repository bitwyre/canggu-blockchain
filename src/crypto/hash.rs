use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A 32-byte SHA-256 hash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create a new hash from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Hash the given data
    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);

        Self(bytes)
    }

    /// Convert the hash to a hexadecimal string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create a hash from a hexadecimal string
    pub fn from_hex(hex_str: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }

        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes);

        Ok(Self(result))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Trait for objects that can be hashed
pub trait Hashable {
    /// Compute the hash of this object
    fn hash(&self) -> Hash;
}
