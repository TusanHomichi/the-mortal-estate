use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::model::{
    ActorKind, CarriedGoldPosition, CarriedPosition, CreatureTrait, PhysicalDamageKind, Stats,
    WorldPosition,
};

use super::{
    ActorAiDef, ActorDeathDef, ActorMagicResistanceDef, MonsterAbilityList, SocialProfileDef,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScavengingProfileDef {
    pub searches_corpses: bool,
    pub collects_ground_items: bool,
    pub collects_gold: bool,
    pub equips_items: bool,
    pub uses_healing_balm: bool,
    pub search_radius: u8,
    pub balm_below_hp_percent: u8,
    pub balm_chance_numerator: u8,
    pub balm_chance_denominator: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorDefinitionDef {
    pub id: String,
    pub kind: ActorKind,
    pub name: String,
    pub creature_traits: Vec<CreatureTrait>,
    pub stats: Stats,
    pub magic_resistance: ActorMagicResistanceDef,
    pub death: ActorDeathDef,
    pub social: SocialProfileDef,
    #[serde(deserialize_with = "deserialize_required_nullable_ai")]
    pub ai: Option<ActorAiDef>,
    #[serde(deserialize_with = "deserialize_required_nullable_i32")]
    pub xp_value: Option<i32>,
    pub physical_damage_affinity_profile_id: String,
    pub monster_abilities: MonsterAbilityList,
    #[serde(default)]
    pub scavenging_profile_id: Option<String>,
}

fn deserialize_required_nullable_ai<'de, D>(deserializer: D) -> Result<Option<ActorAiDef>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<ActorAiDef>::deserialize(deserializer)
}

fn deserialize_required_nullable_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<i32>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalDamageAffinityResponseDef {
    pub damage_kind: PhysicalDamageKind,
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalDamageAffinityProfileDef {
    pub id: String,
    pub responses: Vec<PhysicalDamageAffinityResponseDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootChoiceMemberDef {
    pub member_id: String,
    pub item_definition_id: String,
    pub quantity: u32,
    pub position: CarriedPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LootEntryDef {
    Item {
        id: String,
        chance_numerator: u32,
        chance_denominator: u32,
        item_definition_id: String,
        quantity: u32,
        position: CarriedPosition,
    },
    ItemChoice {
        id: String,
        chance_numerator: u32,
        chance_denominator: u32,
        members: Vec<LootChoiceMemberDef>,
    },
    Gold {
        id: String,
        chance_numerator: u32,
        chance_denominator: u32,
        minimum_amount: i64,
        maximum_amount: i64,
        position: CarriedGoldPosition,
    },
}

impl LootEntryDef {
    pub fn id(&self) -> &str {
        match self {
            Self::Item { id, .. } | Self::ItemChoice { id, .. } | Self::Gold { id, .. } => id,
        }
    }

    pub const fn chance(&self) -> (u32, u32) {
        match self {
            Self::Item {
                chance_numerator,
                chance_denominator,
                ..
            }
            | Self::ItemChoice {
                chance_numerator,
                chance_denominator,
                ..
            }
            | Self::Gold {
                chance_numerator,
                chance_denominator,
                ..
            } => (*chance_numerator, *chance_denominator),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LootTableFamilyDef {
    Ordinary,
    Signature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootTableDef {
    pub family: LootTableFamilyDef,
    pub id: String,
    #[serde(default)]
    pub maximum_non_gold_drops: Option<u8>,
    pub entries: Vec<LootEntryDef>,
}

impl LootTableDef {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn entries(&self) -> &[LootEntryDef] {
        &self.entries
    }

    pub const fn maximum_non_gold_drops(&self) -> Option<u8> {
        self.maximum_non_gold_drops
    }

    pub const fn is_signature(&self) -> bool {
        matches!(self.family, LootTableFamilyDef::Signature)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcologyKindDef {
    Solitary,
    Pack,
    Lair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnMemberDef {
    pub member_id: String,
    pub actor_definition_id: String,
    #[serde(deserialize_with = "deserialize_required_nullable_string")]
    pub loot_table_id: Option<String>,
}

fn deserialize_required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpawnResetDef {
    FullSite {
        delay_units: u32,
    },
    SlotReplenishment {
        slot_delay_units: u32,
        full_clear_delay_units: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnGroupDef {
    pub id: String,
    pub ecology_kind: EcologyKindDef,
    pub members: Vec<SpawnMemberDef>,
    pub reset: SpawnResetDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LairDefinitionDef {
    pub id: String,
    pub name: String,
    pub spawn_group_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EcologySiteSourceDef {
    SpawnGroup { spawn_group_id: String },
    Lair { lair_definition_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcologySiteDef {
    pub id: String,
    pub source: EcologySiteSourceDef,
    pub member_locations: BTreeMap<String, WorldPosition>,
}
