use anyhow::{Result, anyhow};
use ed25519_dalek::{Signer, SigningKey};
use ed25519_dalek::{ Signature as Ed25519Signature, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::CryptoRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{fmt, fs};
use std::path::Path;

/// A public key (Ed25519)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub VerifyingKey);

/// A signature (Ed25519)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub Ed25519Signature);

/// A keypair (Ed25519)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keypair(ed25519_dalek::SigningKey);

impl Keypair {
    /// Can be used for generating a Keypair without a dependency on `rand` types
    pub const SECRET_KEY_LENGTH: usize = 32;

    /// Constructs a new, random `Keypair` using a caller-provided RNG
    pub fn generate<R>(csprng: &mut R) -> Self
    where
        R: CryptoRng + RngCore,
    {  
        Self(ed25519_dalek::SigningKey::generate(csprng))
    }

    /// Constructs a new, random `Keypair` using `OsRng`
    pub fn new() -> Self {
        let mut rng = OsRng;
        Self::generate(&mut rng)
    }

    /// Recovers a `Keypair` from a byte array
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ed25519_dalek::SignatureError> {
        if bytes.len() < ed25519_dalek::KEYPAIR_LENGTH {
            return Err(ed25519_dalek::SignatureError::from_source(String::from(
                "candidate keypair byte array is too short",
            )));
        }
        let secret_bytes: [u8; 32] = bytes[..ed25519_dalek::SECRET_KEY_LENGTH]
            .try_into()
            .map_err(|_| ed25519_dalek::SignatureError::from_source(String::from("invalid secret key length")))?;
        let secret = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let public: [u8; 32] = bytes[32..ed25519_dalek::PUBLIC_KEY_LENGTH].try_into().unwrap();
        let public = VerifyingKey::from_bytes(&public).unwrap();

        let expected_public = secret.verifying_key();

        (public == expected_public)
            .then_some(Self(secret))
            .ok_or(ed25519_dalek::SignatureError::from_source(String::from(
                "keypair bytes do not specify same pubkey as derived from their secret key",
            )))
    }

    /// Recovers a `Keypair` from a base58-encoded string
    pub fn from_base58_string(s: &str) -> Self {
        let mut buf = [0u8; ed25519_dalek::KEYPAIR_LENGTH];
        bs58::decode(s).onto(&mut buf).unwrap();
        Self::from_bytes(&buf).unwrap()
    }

    /// Returns this `Keypair` as a base58-encoded string
    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.0.to_bytes()).into_string()
    }

    /// Gets this `Keypair`'s SecretKey
    pub fn secret(&self) -> &ed25519_dalek::SecretKey {
        &self.0.as_bytes()
    }

    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }

    /// Save the keypair to a file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let keypair_bytes = bincode::serialize(&KeypairSerialized {
            public: self.public().to_bytes(),
            secret: *self.0.as_bytes()
        })?;

        let json_bytes = serde_json::to_string(&keypair_bytes)?;
        fs::write(path, &json_bytes)?;
        Ok(())
    }
    
    /// Load a keypair from a file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let keypair_bytes = fs::read(path)?;

        let deserialized_bytes: Vec<u8> = serde_json::from_slice(&keypair_bytes)?;

        let keypair_data: KeypairSerialized = bincode::deserialize(&deserialized_bytes)?;

        let signing_key = SigningKey::from_bytes(&keypair_data.secret.into());
        let verifying_key = VerifyingKey::from_bytes(&keypair_data.public)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;

        let keypair = Self(signing_key);
        Ok(keypair)
    }
}

impl Signer<Signature> for Keypair {
    
    fn try_sign(&self, msg: &[u8]) -> Result<Signature, ed25519_dalek::ed25519::Error> {
        let signature = self.0.try_sign(msg)?;
        Ok(Signature(signature))
    }
    
    fn sign(&self, msg: &[u8]) -> Signature {
        self.try_sign(msg).unwrap()
    }
}

/// Serializable keypair struct (for storage)
#[derive(Serialize, Deserialize)]
struct KeypairSerialized {
    public: [u8; 32],
    secret: [u8; 32],
}

impl PublicKey {
    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let pubkey = VerifyingKey::from_bytes(
            bytes
                .try_into()
                .map_err(|_| anyhow!("Invalid public key length"))?,
        )
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;
        Ok(Self(pubkey))
    }

    /// Convert to a base58 string
    pub fn to_string(&self) -> String {
        bs58::encode(self.0.to_bytes()).into_string()
    }

    /// Create from a base58 string
    pub fn from_string(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| anyhow!("Invalid base58 string: {}", e))?;

        Self::from_bytes(&bytes)
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.0.verify(message, &signature.0).is_ok()
    }

    /// Returns this `verifyingKey` as a base58-encoded string
    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.0.to_bytes()).into_string()
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0.to_bytes()).into_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;                                                  

    #[test]
    fn test_keypair_roundtrip() {
        let mut rng = OsRng;
        let keypair = Keypair::generate(&mut rng);
        let serialized = serde_json::to_string(&keypair).unwrap();

        let deserialized: Keypair = serde_json::from_str(&serialized).unwrap();

        assert_eq!(keypair, deserialized);
        assert_eq!(keypair.secret(), deserialized.secret());
    }
}