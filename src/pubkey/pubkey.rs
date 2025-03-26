use std::{error::Error, fmt};

use bs58;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pubkey(pub(crate) [u8; 32]);

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pubkey({})", bs58::encode(self.0).into_string())
    }
}
impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl From<&str> for Pubkey {
    fn from(s: &str) -> Self {
        let mut bytes = [0u8; 32];
        bs58::decode(s).onto(&mut bytes).unwrap();
        Self(bytes)
    }
}

impl From<&[u8]> for Pubkey {
    fn from(bytes: &[u8]) -> Self {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Self(arr)
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl Pubkey {
    /// Create a new `Pubkey` from a byte array
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        if bytes.len() != 32 {
            return Err(Box::new(ed25519_dalek::SignatureError::from_source(String::from(
                "invalid public key length",
            ))));
        }
        Ok(Self::from(bytes))
    }
}