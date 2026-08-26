use serde::{Deserialize, Serialize};

use crate::model::CharacterAlignment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialAlignmentSourceDef {
    Character {},
    Inherent { alignment: CharacterAlignment },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialNatureDef {
    Human,
    Animal,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialBehaviorDef {
    Adventurer,
    Civilian,
    TownEnforcer,
    AlignmentCreature,
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialOwnerRelationDef {
    None,
    Summoner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialProfileDef {
    pub alignment_source: SocialAlignmentSourceDef,
    pub nature: SocialNatureDef,
    pub behavior: SocialBehaviorDef,
    pub owner_relation: SocialOwnerRelationDef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LawZoneDef {
    None,
    Town,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TownLawClassificationDef {
    Permitted,
    TerrainAlignmentViolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellSocialDef {
    pub hostile_act: bool,
    pub town_law: TownLawClassificationDef,
}
