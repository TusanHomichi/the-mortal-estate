//! Content-driven XP receipt, derived pending levels, and ordered level application.

use crate::events::Event;
use crate::model::{
    CharacterProgression, GrowthAttribute, GrowthRule, ProgressionGrowthProfile, ProgressionRules,
    WeightedGrowthOutcome,
};

use super::{Engine, StepError};

pub(super) fn award_character_experience(
    engine: &mut Engine,
    actor_index: usize,
    xp_gained: i32,
) -> Result<Vec<Event>, StepError> {
    if xp_gained <= 0 {
        return Ok(Vec::new());
    }
    let actor = engine
        .world
        .actors
        .get_mut(actor_index)
        .ok_or_else(|| StepError::new("unknown XP recipient"))?;
    let Some(character) = &mut actor.character else {
        return Ok(Vec::new());
    };
    character.progression.experience = character
        .progression
        .experience
        .checked_add(i64::from(xp_gained))
        .ok_or_else(|| StepError::new("character experience overflow"))?;
    Ok(vec![Event::ExperienceAwarded {
        actor_id: actor.id.clone(),
        actor: actor.name.clone(),
        amount: xp_gained,
        total_xp: character.progression.experience,
    }])
}

fn earned_level(experience: i64, rules: &ProgressionRules) -> Option<i32> {
    rules
        .level_thresholds
        .iter()
        .rev()
        .find(|row| experience >= row.cumulative_experience)
        .map(|row| row.level)
}

pub(super) fn pending_target_level(
    progression: &CharacterProgression,
    rules: &ProgressionRules,
) -> Option<i32> {
    earned_level(progression.experience, rules).filter(|earned| *earned > progression.level)
}

fn selected_outcomes(
    rule: &GrowthRule,
    strength: i32,
    constitution: i32,
) -> Result<&[WeightedGrowthOutcome], StepError> {
    match rule {
        GrowthRule::Fixed { outcomes } => Ok(outcomes),
        GrowthRule::AttributeBands { attribute, bands } => {
            let value = match attribute {
                GrowthAttribute::Strength => strength,
                GrowthAttribute::Constitution => constitution,
            };
            bands
                .iter()
                .rev()
                .find(|band| value >= band.minimum_attribute)
                .map(|band| band.outcomes.as_slice())
                .ok_or_else(|| StepError::new("growth rule has no matching attribute band"))
        }
    }
}

fn roll_growth(
    engine: &mut Engine,
    rule: &GrowthRule,
    strength: i32,
    constitution: i32,
) -> Result<i32, StepError> {
    let outcomes = selected_outcomes(rule, strength, constitution)?;
    let weights = outcomes
        .iter()
        .map(|outcome| outcome.weight)
        .collect::<Vec<_>>();
    let index = engine
        .rng
        .weighted_index(&weights)
        .map_err(StepError::new)?;
    outcomes
        .get(index)
        .map(|outcome| outcome.amount)
        .ok_or_else(|| StepError::new("growth outcome index is invalid"))
}

fn profile_for_current_class(
    engine: &Engine,
    actor_index: usize,
) -> Result<ProgressionGrowthProfile, StepError> {
    let actor = engine
        .world
        .actors
        .get(actor_index)
        .ok_or_else(|| StepError::new("unknown level recipient"))?;
    let character = actor
        .character
        .as_ref()
        .ok_or_else(|| StepError::new("level recipient has no character"))?;
    engine
        .definition
        .catalog
        .rules
        .progression
        .growth_profiles
        .get(&character.identity.current_class_id)
        .cloned()
        .ok_or_else(|| {
            StepError::new(format!(
                "no progression growth profile for current class {:?}",
                character.identity.current_class_id
            ))
        })
}

pub(super) fn apply_ready_level_advances(
    engine: &mut Engine,
    actor_index: usize,
    events: &mut Vec<Event>,
) -> Result<(), StepError> {
    let target_level = {
        let actor = engine
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown level recipient"))?;
        if !actor.is_alive() {
            return Ok(());
        }
        let Some(character) = actor.character.as_ref() else {
            return Ok(());
        };
        if character.resources.hp != character.resources.max_hp
            || character.resources.stamina != character.resources.max_stamina
        {
            return Ok(());
        }
        let Some(target) = pending_target_level(
            &character.progression,
            &engine.definition.catalog.rules.progression,
        ) else {
            return Ok(());
        };
        target
    };

    loop {
        let (current_level, total_xp, strength, constitution) = {
            let character = engine.world.actors[actor_index]
                .character
                .as_ref()
                .expect("character checked before level application");
            (
                character.progression.level,
                character.progression.experience,
                character.attributes.strength,
                character.attributes.constitution,
            )
        };
        if current_level >= target_level {
            return Ok(());
        }
        let next_level = current_level
            .checked_add(1)
            .ok_or_else(|| StepError::new("character level overflow"))?;
        if !engine
            .definition
            .catalog
            .rules
            .progression
            .level_thresholds
            .iter()
            .any(|row| row.level == next_level)
        {
            return Err(StepError::new(
                "pending level is absent from authored thresholds",
            ));
        }

        let profile = profile_for_current_class(engine, actor_index)?;
        let hp_growth = roll_growth(engine, &profile.hit_points, strength, constitution)?;
        let mp_growth = profile
            .magic_points
            .as_ref()
            .map(|rule| roll_growth(engine, rule, strength, constitution))
            .transpose()?
            .unwrap_or(0);
        let stamina_growth = roll_growth(engine, &profile.stamina_points, strength, constitution)?;

        let combat_growth = profile
            .physical_attribute_adds_by_level
            .iter()
            .find(|row| row.level == next_level)
            .cloned();
        let next_physical_attribute_adds = combat_growth
            .as_ref()
            .map(|growth| {
                let current = &engine.world.actors[actor_index]
                    .character
                    .as_ref()
                    .expect("character checked before combat-add growth")
                    .physical_attribute_adds;
                Ok::<_, StepError>((
                    current
                        .strength_adds
                        .checked_add(growth.strength_adds)
                        .ok_or_else(|| StepError::new("strength combat-add overflow"))?,
                    current
                        .dexterity_adds
                        .checked_add(growth.dexterity_adds)
                        .ok_or_else(|| StepError::new("dexterity combat-add overflow"))?,
                ))
            })
            .transpose()?;

        let receipt =
            engine.apply_level_growth(actor_index, hp_growth, mp_growth, stamina_growth)?;
        let (actor_id, actor_name, current_class_id) = {
            let actor = &mut engine.world.actors[actor_index];
            let character = actor
                .character
                .as_mut()
                .expect("character checked before level commit");
            character.progression.level = next_level;
            if let Some((strength_adds, dexterity_adds)) = next_physical_attribute_adds {
                character.physical_attribute_adds.strength_adds = strength_adds;
                character.physical_attribute_adds.dexterity_adds = dexterity_adds;
            }
            (
                actor.id.clone(),
                actor.name.clone(),
                character.identity.current_class_id.clone(),
            )
        };

        events.push(Event::LevelGained {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            current_class_id,
            new_level: next_level,
            total_xp,
            hp_growth,
            hp: receipt.hp,
            max_hp: receipt.max_hp,
            peak_hp: receipt.peak_hp,
            mp_growth,
            mp: receipt.mp,
            max_mp: receipt.max_mp,
            stamina_growth,
            stamina: receipt.stamina,
            max_stamina: receipt.max_stamina,
        });
        if let Some((strength_adds, dexterity_adds)) = next_physical_attribute_adds {
            events.push(Event::PhysicalAttributeAddsChanged {
                actor_id,
                actor: actor_name,
                strength_adds,
                dexterity_adds,
            });
        }
    }
}
