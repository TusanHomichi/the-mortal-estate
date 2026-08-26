use serde::{Deserialize, Serialize};

use crate::model::{
    Coord, CreatureTrait, ResurrectionMethod, SpellCastClass, SpellCastingMethod,
    SpellCatalogState, SpellDurationPolicy, SpellEffectFamily, SpellItemLocation,
    SpellResistanceBoost, SpellResistanceMitigation, SpellResistanceMitigationMode,
    SpellTargetKind, WorldSite,
};

use super::{SpellSocialDef, TopologyTargetDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDef {
    pub id: String,
    pub name: String,
    pub status: String,
    pub social: SpellSocialDef,
    pub effect_model: Option<String>,
    #[serde(default)]
    pub lane: Option<String>,
    #[serde(default)]
    pub skill_requirement: Option<i32>,
    #[serde(default)]
    pub mp_cost: Option<i32>,
    #[serde(default)]
    pub stamina_cost: Option<i32>,
    #[serde(default)]
    pub effect: Option<SpellEffectDef>,
    #[serde(default)]
    pub target: Option<SpellTargetDef>,
    #[serde(default)]
    pub acquisition: Option<SpellAcquisitionDef>,
    #[serde(default)]
    pub casting: Option<SpellCastingDef>,
    #[serde(default)]
    pub catalog_entry: Option<SpellCatalogEntryDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellCatalogEntryDef {
    pub row_id: String,
    pub topic_id: String,
    #[serde(default)]
    pub acquisition_row_id: Option<String>,
    pub variant_id: String,
    pub effect_family: SpellEffectFamily,
    #[serde(default)]
    pub target_kind: Option<SpellTargetKind>,
    pub state: SpellCatalogState,
    #[serde(default)]
    pub open_question_ids: Vec<String>,
    #[serde(default)]
    pub resistance_tags: Vec<String>,
    #[serde(default)]
    pub resistance_mitigation_mode: Option<SpellResistanceMitigationMode>,
    #[serde(default)]
    pub client_row_id: Option<String>,
    #[serde(default)]
    pub client_spell_id: Option<u32>,
    #[serde(default)]
    pub client_verb_type: Option<u32>,
    #[serde(default)]
    pub client_powerable: Option<bool>,
    #[serde(default)]
    pub client_spell_poem_id: Option<u32>,
    #[serde(default)]
    pub client_offensive: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellEffectDef {
    pub family: SpellEffectFamily,
    #[serde(default)]
    pub status_kind: Option<String>,
    #[serde(default)]
    pub damage_kind: Option<String>,
    #[serde(default)]
    pub potency: Option<i32>,
    #[serde(default)]
    pub start_delay_rounds: Option<i32>,
    #[serde(default)]
    pub suppresses_action: Option<bool>,
    #[serde(default)]
    pub stacking: Option<String>,
    #[serde(default)]
    pub duration: Option<SpellDurationDef>,
    #[serde(default)]
    pub resistance: Option<SpellResistanceDef>,
    #[serde(default)]
    pub banish: Option<SpellBanishDef>,
    #[serde(default)]
    pub instant_death: Option<SpellInstantDeathDef>,
    #[serde(default)]
    pub raise_dead: Option<SpellRaiseDeadDef>,
    #[serde(default)]
    pub turn_undead: Option<SpellTurnUndeadDef>,
    #[serde(default)]
    pub terrain_overlay: Option<SpellTerrainOverlayDef>,
    #[serde(default)]
    pub door_control: Option<SpellDoorControlDef>,
    #[serde(default)]
    pub item_utility: Option<SpellItemUtilityDef>,
    #[serde(default)]
    pub locate: Option<SpellLocateDef>,
    #[serde(default)]
    pub scry: Option<SpellScryDef>,
    #[serde(default)]
    pub portal: Option<SpellPortalDef>,
    #[serde(default)]
    pub summon_actor_id: Option<String>,
    #[serde(default)]
    pub item_interaction: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellBanishDef {
    pub eligible_traits: Vec<CreatureTrait>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellInstantDeathDef {
    pub damage_per_magic_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellRaiseDeadDef {
    pub method: ResurrectionMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellTurnUndeadDef {
    pub eligible_trait: CreatureTrait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellResistanceDef {
    Incoming {
        tag: String,
        mitigation: SpellResistanceMitigation,
    },
    Boost {
        boosts: Vec<SpellResistanceBoost>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDoorControlDef {
    pub action: String,
    #[serde(default)]
    pub range: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellItemUtilityDef {
    pub action: String,
    #[serde(default)]
    pub combat_add_rating_bonus: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub output_item_definition_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellLocateDef {
    pub subject: String,
    pub id: String,
    #[serde(default = "default_observed_only")]
    pub observed_only: bool,
}

fn default_observed_only() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellScryDef {
    pub scope: String,
    pub site: WorldSite,
    #[serde(default)]
    pub position: Option<Coord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellPortalDef {
    pub target: TopologyTargetDef,
    #[serde(default)]
    pub two_way: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellTargetDef {
    pub kind: SpellTargetKind,
    #[serde(default)]
    pub range: Option<i32>,
    #[serde(default)]
    pub requires_visible: Option<bool>,
    #[serde(default)]
    pub area: Option<SpellAreaDef>,
    #[serde(default)]
    pub item_location: Option<SpellItemLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDurationDef {
    pub policy: SpellDurationPolicy,
    #[serde(default)]
    pub rounds: Option<i32>,
    #[serde(default)]
    pub tick_interval_rounds: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellAreaDef {
    pub shape: String,
    #[serde(default)]
    pub radius: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellTerrainOverlayDef {
    #[serde(default)]
    pub passability: Option<String>,
    #[serde(default)]
    pub sight: Option<String>,
    #[serde(default)]
    pub hazard: Option<String>,
    #[serde(default)]
    pub move_cost: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellAcquisitionDef {
    pub gold_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellCastingDef {
    pub method: SpellCastingMethod,
    pub cast_class: SpellCastClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellTeachingDef {
    pub spell_id: String,
}
