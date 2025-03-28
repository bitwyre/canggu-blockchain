use serde::{Deserialize, Serialize};
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake256,
};
use std::fmt;

/// A hash in the Merkle tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Create a new hash from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Hash the given data
    /// Hash the given data
    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Shake256::default();
        hasher.update(data);

        let mut reader = hasher.finalize_xof();

        let mut bytes = [0u8; 32];
        reader.read(&mut bytes);

        Self(bytes)
    }

    /// Get the bytes of the hash
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Convert to a hex string
    pub fn to_hex(&self) -> String {
        let mut result = String::with_capacity(64);
        for byte in &self.0 {
            result.push_str(&format!("{:02x}", byte));
        }
        result
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// A Merkle tree for efficient verification of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Nodes in the tree
    nodes: Vec<Hash>,

    /// Number of leaf nodes
    leaf_count: usize,
}

impl MerkleTree {
    /// Create a new Merkle tree from a list of items
    pub fn new<T: AsRef<[u8]>>(items: &[T]) -> Self {
        if items.is_empty() {
            return Self {
                nodes: vec![Hash([0; 32])],
                leaf_count: 0,
            };
        }

        // Create leaf nodes
        let mut leaf_nodes: Vec<Hash> =
            items.iter().map(|item| Hash::hash(item.as_ref())).collect();

        let leaf_count = leaf_nodes.len();

        // Ensure power of 2 by padding with zeros
        let mut size = leaf_count.next_power_of_two();
        while leaf_nodes.len() < size {
            leaf_nodes.push(Hash([0; 32]));
        }

        // Build the tree
        let mut nodes = leaf_nodes;
        let mut level_size = size;

        while level_size > 1 {
            level_size /= 2;
            let mut next_level = Vec::with_capacity(level_size);

            for i in 0..level_size {
                let left = nodes[i * 2];
                let right = nodes[i * 2 + 1];

                // Combine left and right nodes
                let mut combined = Vec::with_capacity(64);
                combined.extend_from_slice(&left.to_bytes());
                combined.extend_from_slice(&right.to_bytes());

                next_level.push(Hash::hash(&combined));
            }

            nodes = next_level;
        }

        Self { nodes, leaf_count }
    }

    /// Get the root hash of the tree
    pub fn root(&self) -> Hash {
        if self.nodes.is_empty() {
            Hash([0; 32])
        } else {
            self.nodes[0]
        }
    }

    /// Generate a proof for the item at the given index
    pub fn generate_proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaf_count {
            return None;
        }

        // TODO: Implement proof generation

        Some(MerkleProof {
            proof: Vec::new(),
            root: self.root(),
        })
    }
}

/// A proof that an item is in the Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Proof nodes
    proof: Vec<Hash>,

    /// Root hash
    root: Hash,
}

impl MerkleProof {
    /// Verify the proof for the given item
    pub fn verify<T: AsRef<[u8]>>(&self, item: T) -> bool {
        // TODO: Implement proof verification

        false
    }
}
