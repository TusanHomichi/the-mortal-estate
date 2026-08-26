use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::{ActorId, ActorKind, CarriedPosition, CharacterId, LogicalTime, WorldPosition};

fn parse_sequence_id(value: &str, prefix: &str) -> Result<u64, String> {
    let Some(sequence) = value.strip_prefix(prefix) else {
        return Err(format!("must start with {prefix}"));
    };
    if sequence.is_empty()
        || sequence.starts_with('0')
        || !sequence.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(format!("must be {prefix}<positive decimal>"));
    }
    sequence
        .parse::<u64>()
        .map_err(|_| format!("must be {prefix}<positive decimal>"))
        .and_then(|parsed| {
            (parsed > 0)
                .then_some(parsed)
                .ok_or_else(|| format!("must be {prefix}<positive decimal>"))
        })
}

macro_rules! sequence_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                parse_sequence_id(&value, $prefix)?;
                Ok(Self(value))
            }

            pub(crate) fn from_sequence(sequence: u64) -> Self {
                assert!(sequence > 0, "sequence IDs start at one");
                Self(format!("{}{}", $prefix, sequence))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(de::Error::custom)
            }
        }
    };
}

sequence_id!(CorpseId, "corpse:");
sequence_id!(GoldPileId, "gold:");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpseDisposition {
    SearchableCorpse,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCause {
    Physical,
    Poison,
    Fire,
    OtherMagic,
    Hazard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActorLifeState {
    Alive,
    Ghost {
        corpse_id: CorpseId,
        defeated_at: LogicalTime,
    },
    AwaitingResurrection {
        cause: DeathCause,
        defeated_at: LogicalTime,
    },
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum LootOwnerId {
    Character(CharacterId),
    TransientActor(ActorId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LootClaimBasis {
    KillingBlow,
    CharacterDeathPile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootClaim {
    pub owner: LootOwnerId,
    pub basis: LootClaimBasis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpseState {
    pub id: CorpseId,
    pub origin_actor_id: ActorId,
    pub origin_character_id: Option<CharacterId>,
    pub origin_kind: ActorKind,
    pub origin_name: String,
    pub location: WorldPosition,
    pub created_at: LogicalTime,
    pub sequence: u64,
    pub searched: bool,
    pub loot_claim: Option<LootClaim>,
    pub contents: BTreeMap<CarriedPosition, String>,
    pub gold: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundGoldPile {
    pub id: GoldPileId,
    pub amount: i64,
    pub location: WorldPosition,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResurrectionMethod {
    Gods,
    Priest,
    Thaumaturge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResurrectionRequest {
    pub actor_id: ActorId,
    pub corpse_id: Option<CorpseId>,
    pub method: ResurrectionMethod,
    pub destination: WorldPosition,
    pub current_hp: i32,
    pub current_stamina: i32,
}
