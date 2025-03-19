use crate::crypto::hash::Hash;
use std::time::{Duration, Instant};

/// Proof of History generator
pub struct ProofOfHistory {
    /// Current state of the PoH sequence
    current_hash: Hash,

    /// Number of hashes produced
    count: u64,

    /// Target time to produce a hash
    hash_time_target: Duration,

    /// Last hash timestamp
    last_hash_time: Instant,
}

impl ProofOfHistory {
    /// Create a new PoH generator
    pub fn new(hash_time_target_ns: u64) -> Self {
        Self {
            current_hash: Hash([0; 32]), // Start with zero hash
            count: 0,
            hash_time_target: Duration::from_nanos(hash_time_target_ns),
            last_hash_time: Instant::now(),
        }
    }

    /// Produce the next hash in the sequence
    pub fn tick(&mut self) -> Hash {
        // In a real implementation, we would loop to achieve the target hash time
        // For simplicity, we're just producing the next hash

        // Create a message with the current hash and count
        let mut message = Vec::with_capacity(40);
        message.extend_from_slice(&self.current_hash.0);
        message.extend_from_slice(&self.count.to_le_bytes());

        // Calculate the next hash
        self.current_hash = Hash::hash(&message);
        self.count += 1;

        // Record the time
        self.last_hash_time = Instant::now();

        self.current_hash
    }

    /// Get the current count
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get the current hash
    pub fn hash(&self) -> Hash {
        self.current_hash
    }

    /// Reset the PoH sequence
    pub fn reset(&mut self) {
        self.current_hash = Hash([0; 32]);
        self.count = 0;
        self.last_hash_time = Instant::now();
    }

    /// Verify a sequence of hashes
    pub fn verify(&self, start_hash: Hash, hashes: &[Hash], count_start: u64) -> bool {
        let mut current = start_hash;
        let mut count = count_start;

        for &expected in hashes {
            // Create message
            let mut message = Vec::with_capacity(40);
            message.extend_from_slice(&current.0);
            message.extend_from_slice(&count.to_le_bytes());

            // Calculate hash
            let next = Hash::hash(&message);

            // Check if it matches expected
            if next != expected {
                return false;
            }

            current = next;
            count += 1;
        }

        true
    }
}

/// An entry in the PoH sequence
#[derive(Debug, Clone)]
pub struct PohEntry {
    /// The hash for this entry
    pub hash: Hash,

    /// The count for this entry
    pub count: u64,

    /// Timestamp of this entry
    pub timestamp: std::time::SystemTime,
}
