use std::collections::BTreeMap;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::model::{
    ActiveEffectSource, ActorId, CarriedGold, CarriedPosition, CharacterAttributes, CharacterId,
    CharacterSheetV1, CorpseDisposition, MagicRuleEvidenceState, SpellResistanceBoost,
    WorldPosition,
};

use super::{ItemInstanceSeedDef, NpcDef, StarterCharacterDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionedItemDef {
    pub item_instance_id: String,
    pub position: CarriedPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedLayoutDef {
    pub items: Vec<PositionedItemDef>,
    pub gold: CarriedGold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorDeathDef {
    pub remains: CorpseDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorMagicResistanceDef {
    pub natural_save_twentieths: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummonTemplateDef {
    pub id: String,
    pub actor_definition_id: String,
    #[serde(default)]
    pub item_instances: BTreeMap<String, ItemInstanceSeedDef>,
    pub carried: CarriedLayoutDef,
    #[serde(default)]
    pub active_effects: Vec<ActiveEffectDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSeedDef {
    pub id: ActorId,
    pub actor_definition_id: String,
    pub location: WorldPosition,
    #[serde(deserialize_with = "deserialize_required_nullable_npc")]
    pub npc: Option<NpcDef>,
    #[serde(default)]
    pub character_id: Option<CharacterId>,
    #[serde(default)]
    pub character: Option<CharacterSheetV1>,
    #[serde(default)]
    pub starter_character: Option<StarterCharacterDef>,
    pub carried: CarriedLayoutDef,
    #[serde(default)]
    pub active_effects: Vec<ActiveEffectDef>,
}

fn deserialize_required_nullable_npc<'de, D>(deserializer: D) -> Result<Option<NpcDef>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<NpcDef>::deserialize(deserializer)
}

impl ActorSeedDef {
    pub(crate) fn effective_attributes(&self) -> Option<&CharacterAttributes> {
        self.character
            .as_ref()
            .map(|character| &character.attributes)
            .or_else(|| {
                self.starter_character
                    .as_ref()
                    .map(StarterCharacterDef::attributes)
            })
    }

    pub(crate) fn effective_current_class_id(&self) -> Option<&str> {
        self.character
            .as_ref()
            .map(|character| character.identity.current_class_id.as_str())
            .or_else(|| {
                self.starter_character
                    .as_ref()
                    .map(StarterCharacterDef::current_class_id)
            })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonsterAbilityList {
    entries: Vec<MonsterAbilityDef>,
    pub(super) present: bool,
}

impl<'de> Deserialize<'de> for MonsterAbilityList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<MonsterAbilityDef>::deserialize(deserializer).map(|entries| Self {
            entries,
            present: true,
        })
    }
}

impl Serialize for MonsterAbilityList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.entries.serialize(serializer)
    }
}

impl Deref for MonsterAbilityList {
    type Target = [MonsterAbilityDef];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl<'a> IntoIterator for &'a MonsterAbilityList {
    type Item = &'a MonsterAbilityDef;
    type IntoIter = std::slice::Iter<'a, MonsterAbilityDef>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonsterAbilityDef {
    pub id: String,
    pub kind: String,
    pub spell_id: String,
    pub cooldown_rounds: u32,
    #[serde(default)]
    pub target_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveEffectDef {
    pub instance_id: String,
    pub effect_id: String,
    pub source: ActiveEffectSource,
    pub kind: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub potency: i32,
    pub remaining_rounds: Option<i32>,
    pub until_condition: Option<String>,
    pub stacking: String,
    #[serde(default)]
    pub start_delay_rounds: i32,
    #[serde(default = "default_tick_interval_rounds_i32")]
    pub tick_interval_rounds: i32,
    #[serde(default)]
    pub suppresses_action: bool,
    #[serde(default)]
    pub resistance_boosts: Vec<SpellResistanceBoost>,
}

fn default_tick_interval_rounds_i32() -> i32 {
    1
}
