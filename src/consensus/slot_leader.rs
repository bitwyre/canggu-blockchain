use crate::crypto::hash::Hash;
use crate::crypto::keys::PublicKey;
use std::collections::HashMap;

/// Type for validator stake
pub type Stake = u64;

/// Slot leader selector
pub struct SlotLeaderSchedule {
    /// Current validators and their stakes
    validators: HashMap<PublicKey, Stake>,

    /// Total stake in the system
    total_stake: Stake,

    /// Number of slots per epoch
    slots_per_epoch: u64,
}

impl SlotLeaderSchedule {
    /// Create a new slot leader schedule
    pub fn new(slots_per_epoch: u64) -> Self {
        Self {
            validators: HashMap::new(),
            total_stake: 0,
            slots_per_epoch,
        }
    }

    /// Add a validator with stake
    pub fn add_validator(&mut self, validator: PublicKey, stake: Stake) {
        // If validator already exists, update stake
        if let Some(current_stake) = self.validators.get_mut(&validator) {
            self.total_stake -= *current_stake;
            *current_stake = stake;
        } else {
            self.validators.insert(validator, stake);
        }

        self.total_stake += stake;
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, validator: &PublicKey) {
        if let Some(stake) = self.validators.remove(validator) {
            self.total_stake -= stake;
        }
    }

    /// Select a leader for a specific slot
    pub fn get_slot_leader(&self, slot: u64, random_seed: Hash) -> Option<PublicKey> {
        if self.validators.is_empty() || self.total_stake == 0 {
            return None;
        }

        // In a real implementation, this would use VRF (Verifiable Random Function)
        // For simplicity, we're using a simpler approach

        // Combine slot and random seed
        let mut seed = Vec::with_capacity(40);
        seed.extend_from_slice(&random_seed.0);
        seed.extend_from_slice(&slot.to_le_bytes());

        // Generate deterministic random value for this slot
        let random_hash = Hash::hash(&seed);

        // Convert to an integer and use modulo arithmetic to select a validator
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&random_hash.0[0..8]);
        let random_value = u64::from_le_bytes(bytes);

        // Select validator weighted by stake
        let mut remaining = random_value % self.total_stake;

        for (validator, stake) in &self.validators {
            if remaining < *stake {
                return Some(validator.clone());
            }
            remaining -= stake;
        }

        // Fallback to first validator (should not happen)
        self.validators.keys().next().cloned()
    }

    /// Generate a schedule for the next epoch
    pub fn generate_schedule(&self, epoch: u64, random_seed: Hash) -> Vec<PublicKey> {
        let mut schedule = Vec::with_capacity(self.slots_per_epoch as usize);

        for i in 0..self.slots_per_epoch {
            let slot = epoch * self.slots_per_epoch + i;
            if let Some(leader) = self.get_slot_leader(slot, random_seed) {
                schedule.push(leader);
            }
        }

        schedule
    }
}
