use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature as Ed25519Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;

/// A public key (Ed25519)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub VerifyingKey);

/// A signature (Ed25519)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub Ed25519Signature);

/// A keypair (Ed25519)
pub struct Keypair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl Keypair {
    // Generate a random keypair
    pub fn random() -> Self {
        let mut csprng = OsRng {};

        // Generate random bytes for keypair
        let mut keypair_bytes = [0u8; 64]; // 32 bytes for public + 32 for private
        csprng.try_fill_bytes(&mut keypair_bytes);

        // Create the SigningKey from keypair bytes
        let signing_key =
            SigningKey::from_keypair_bytes(&keypair_bytes).expect("Failed to generate keypair");
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }
    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.verifying_key)
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        let signature = self.signing_key.sign(message);
        Signature(signature)
    }

    /// Save the keypair to a file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let keypair_bytes = bincode::serialize(&KeypairSerialized {
            public: self.verifying_key.to_bytes(),
            secret: self.signing_key.to_bytes(),
        })?;

        fs::write(path, &keypair_bytes)?;
        Ok(())
    }

    /// Load a keypair from a file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let keypair_bytes = fs::read(path)?;
        let keypair_data: KeypairSerialized = bincode::deserialize(&keypair_bytes)?;

        let signing_key = SigningKey::from_bytes(&keypair_data.secret.into());
        let verifying_key = VerifyingKey::from_bytes(&keypair_data.public)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;

        Ok(Self {
            signing_key,
            verifying_key,
        })
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
