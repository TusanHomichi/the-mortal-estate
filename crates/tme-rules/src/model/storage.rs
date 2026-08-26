use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{CarriedPosition, CharacterId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BankId(String);

impl BankId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LockerVaultId(String);

impl LockerVaultId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankDefinition {
    pub transaction_cap_gold: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankState {
    pub balances: BTreeMap<CharacterId, i64>,
}

impl BankState {
    pub fn balance(&self, character_id: &CharacterId) -> i64 {
        self.balances.get(character_id).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockerVaultDefinition {
    pub capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockerVaultState {
    pub lockers: BTreeMap<CharacterId, Vec<String>>,
}

impl LockerVaultState {
    pub fn contents(&self, character_id: &CharacterId) -> &[String] {
        self.lockers
            .get(character_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemOfferState {
    pub sender_character_id: CharacterId,
    pub recipient_character_id: CharacterId,
    pub source_position: CarriedPosition,
}
