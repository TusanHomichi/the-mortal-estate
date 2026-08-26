use crate::model::{ActorKind, CharacterAlignment, SpellEffectFamily, SpellTargetKind};

use super::super::{
    SocialAlignmentSourceDef, SocialBehaviorDef, SocialNatureDef, SocialOwnerRelationDef,
    SocialProfileDef, SpellDef, TownLawClassificationDef,
};

pub(super) fn validate_actor_social_profile(
    profile: &SocialProfileDef,
    kind: ActorKind,
    has_character_role: bool,
    has_ai: bool,
    summon_template: bool,
    label: &str,
    errors: &mut Vec<String>,
) {
    let inherent_alignment = match &profile.alignment_source {
        SocialAlignmentSourceDef::Character {} => {
            if !has_character_role {
                errors.push(format!(
                    "{label}.social.alignment_source character requires a character-backed actor"
                ));
            }
            None
        }
        SocialAlignmentSourceDef::Inherent { alignment } => {
            if has_character_role {
                errors.push(format!(
                    "{label}.social.alignment_source must be character for a character-backed actor"
                ));
            }
            Some(*alignment)
        }
    };

    if summon_template {
        if !matches!(
            profile.alignment_source,
            SocialAlignmentSourceDef::Inherent { .. }
        ) {
            errors.push(format!(
                "{label}.social.alignment_source must be inherent for a summon template"
            ));
        }
        if profile.owner_relation != SocialOwnerRelationDef::Summoner {
            errors.push(format!(
                "{label}.social.owner_relation must be summoner for a summon template"
            ));
        }
    } else if profile.owner_relation != SocialOwnerRelationDef::None {
        errors.push(format!(
            "{label}.social.owner_relation summoner is valid only for summon templates"
        ));
    }

    if profile.behavior == SocialBehaviorDef::TownEnforcer
        && !(kind == ActorKind::Npc
            && profile.nature == SocialNatureDef::Human
            && inherent_alignment == Some(CharacterAlignment::Lawful))
    {
        errors.push(format!(
            "{label}.social.behavior town_enforcer requires an inherent-lawful human NPC"
        ));
    }

    match kind {
        ActorKind::Player => {
            if profile.nature != SocialNatureDef::Human {
                errors.push(format!("{label}.social.nature must be human for a player"));
            }
            if profile.behavior != SocialBehaviorDef::Adventurer {
                errors.push(format!(
                    "{label}.social.behavior must be adventurer for a player"
                ));
            }
        }
        ActorKind::Npc => {
            if has_character_role {
                errors.push(format!(
                    "{label}.social does not permit a character-backed NPC"
                ));
            }
            if profile.nature != SocialNatureDef::Human {
                errors.push(format!("{label}.social.nature must be human for an NPC"));
            }
            let eligible_lawful_human = inherent_alignment == Some(CharacterAlignment::Lawful)
                && profile.nature == SocialNatureDef::Human
                && matches!(
                    profile.behavior,
                    SocialBehaviorDef::Civilian | SocialBehaviorDef::TownEnforcer
                );
            if inherent_alignment == Some(CharacterAlignment::Lawful)
                && profile.nature == SocialNatureDef::Human
                && !matches!(
                    profile.behavior,
                    SocialBehaviorDef::Civilian | SocialBehaviorDef::TownEnforcer
                )
            {
                errors.push(format!(
                    "{label}.social.behavior must be civilian or town_enforcer for an inherent-lawful human NPC"
                ));
            }
            if eligible_lawful_human && !has_ai {
                errors.push(format!(
                    "{label}.ai is required for an inherent-lawful human NPC"
                ));
            }
            if has_ai && !eligible_lawful_human {
                errors.push(format!(
                    "{label}.ai is valid on an NPC only for an inherent-lawful human civilian or town_enforcer"
                ));
            }
        }
        ActorKind::Monster => {
            if matches!(
                profile.behavior,
                SocialBehaviorDef::Adventurer
                    | SocialBehaviorDef::Civilian
                    | SocialBehaviorDef::TownEnforcer
            ) {
                errors.push(format!(
                    "{label}.social.behavior is not valid for a monster"
                ));
            }
        }
    }

    if profile.behavior == SocialBehaviorDef::Civilian && kind != ActorKind::Npc {
        errors.push(format!(
            "{label}.social.behavior civilian is valid only for an NPC"
        ));
    }
    if profile.behavior == SocialBehaviorDef::AlignmentCreature
        && !matches!(
            profile.alignment_source,
            SocialAlignmentSourceDef::Inherent { .. }
        )
    {
        errors.push(format!(
            "{label}.social.behavior alignment_creature requires inherent alignment"
        ));
    }
}

pub(super) fn validate_spell_social_definition(
    spell: &SpellDef,
    index: usize,
    errors: &mut Vec<String>,
) {
    let family = spell
        .effect
        .as_ref()
        .map(|effect| effect.family)
        .or_else(|| {
            spell
                .catalog_entry
                .as_ref()
                .map(|entry| entry.effect_family)
        });
    let target = spell.target.as_ref().map(|target| target.kind).or_else(|| {
        spell
            .catalog_entry
            .as_ref()
            .and_then(|entry| entry.target_kind)
    });
    let hostile_family = matches!(
        family,
        Some(
            SpellEffectFamily::Banish
                | SpellEffectFamily::Curse
                | SpellEffectFamily::DirectDamage
                | SpellEffectFamily::InstantDeath
                | SpellEffectFamily::Poison
                | SpellEffectFamily::TurnUndead
        )
    ) || (family == Some(SpellEffectFamily::ControlStatus)
        && target == Some(SpellTargetKind::Actor));

    if spell.social.hostile_act != hostile_family {
        errors.push(format!(
            "spells[{index}].social.hostile_act must be {hostile_family} for the current effect family and target"
        ));
    }

    if spell.social.town_law == TownLawClassificationDef::TerrainAlignmentViolation
        && !matches!(
            family,
            Some(
                SpellEffectFamily::TerrainOverlay
                    | SpellEffectFamily::Darkness
                    | SpellEffectFamily::Light
            )
        )
    {
        errors.push(format!(
            "spells[{index}].social.town_law terrain_alignment_violation requires a terrain, darkness, or light effect family"
        ));
    }
}
