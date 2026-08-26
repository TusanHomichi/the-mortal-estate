use serde::{Deserialize, Serialize};

use crate::model::{ActorAiBehavior, ActorAwarenessPolicy, PhysicalAttackMode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorAwarenessDef {
    Unrestricted {},
    LineOfSightMemory { memory_opportunities: u32 },
}

impl ActorAwarenessDef {
    pub(crate) const fn policy(&self) -> ActorAwarenessPolicy {
        match self {
            Self::Unrestricted {} => ActorAwarenessPolicy::Unrestricted,
            Self::LineOfSightMemory {
                memory_opportunities,
            } => ActorAwarenessPolicy::LineOfSightMemory {
                memory_opportunities: *memory_opportunities,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorAiDef {
    pub behavior: ActorAiBehavior,
    pub cadence_units: u32,
    pub aggro_radius: u32,
    pub leash_range: u32,
    pub awareness: ActorAwarenessDef,
    pub physical_attack_modes: Vec<PhysicalAttackMode>,
}

pub(super) fn validate_actor_ai_definition(
    definition: &ActorAiDef,
    label: &str,
    errors: &mut Vec<String>,
) {
    if definition.cadence_units == 0 {
        errors.push(format!("{label}.cadence_units must be positive"));
    }
    if definition.aggro_radius == 0 {
        errors.push(format!("{label}.aggro_radius must be positive"));
    }
    if definition.leash_range == 0 {
        errors.push(format!("{label}.leash_range must be positive"));
    }
    if let ActorAwarenessDef::LineOfSightMemory {
        memory_opportunities,
    } = definition.awareness
        && memory_opportunities == 0
    {
        errors.push(format!(
            "{label}.awareness.memory_opportunities must be positive"
        ));
    }
    if definition.physical_attack_modes.is_empty() {
        errors.push(format!("{label}.physical_attack_modes must be non-empty"));
    }
    let mut seen = std::collections::BTreeMap::new();
    for (index, mode) in definition.physical_attack_modes.iter().enumerate() {
        if let Some(previous) = seen.insert(*mode, index) {
            errors.push(format!(
                "{label}.physical_attack_modes[{index}] duplicates {label}.physical_attack_modes[{previous}]"
            ));
        }
    }
}
