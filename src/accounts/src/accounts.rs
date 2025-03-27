use crate::pubkey::pubkey::Pubkey;
use serde::{
    Deserialize,
    ser::{Serialize, SerializeStruct, Serializer},
};

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
}

impl Account {
    pub fn new(balance: u64, nonce: u64, data: Vec<u8>, owner: Pubkey, executable: bool) -> Self {
        Self {
            balance,
            nonce,
            data,
            owner,
            executable,
        }
    }
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Account", 5)?;
        state.serialize_field("balance", &self.balance)?;
        state.serialize_field("nonce", &self.nonce)?;
        state.serialize_field("data", &self.data)?;
        state.serialize_field("owner", &self.owner)?;
        state.serialize_field("executable", &self.executable)?;
        state.end()
    }
}
