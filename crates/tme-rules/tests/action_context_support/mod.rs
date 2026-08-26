#![allow(dead_code)]

use crate::support::content_parts::ContentParts;
use tme_rules::model::{ActiveEffectSource, TileEffectState};
use tme_rules::{
    ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, Coord, Engine, PlayerCommandV1, PlayerIntent,
    PlayerIntentPayloadV1, WorldPosition,
};

pub(crate) mod common;
pub(crate) mod items;
pub(crate) mod professions;
pub(crate) mod projection;
#[path = "spells/learning.rs"]
pub(crate) mod spell_learning;
#[path = "spells/readiness.rs"]
pub(crate) mod spell_readiness;
#[path = "spells/warmed.rs"]
pub(crate) mod spell_warmed;
#[path = "spells/wizard.rs"]
pub(crate) mod spell_wizard;
