//! Per-track skill positions, permanent learning rates, and practice progression.
//!
//! Level/title facts are separate from the ten critique ranks inside each
//! level. Catalog-authored rules provide every practice threshold and rate.

use crate::content::SkillTrackKind;
use crate::events::Event;
use crate::model::SkillEntry;

use super::{Engine, StepError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillTrainingCategory {
    Combat,
    Thievery,
    Magic,
}

fn training_category(kind: SkillTrackKind) -> SkillTrainingCategory {
    match kind {
        SkillTrackKind::Weapon | SkillTrackKind::MartialArts => SkillTrainingCategory::Combat,
        SkillTrackKind::Thievery => SkillTrainingCategory::Thievery,
        SkillTrackKind::Magic => SkillTrainingCategory::Magic,
    }
}

/// The only classes with an intrinsic magic-skill track.
///
/// Knight spell access remains a separate ring-powered spell lane and
/// intentionally returns no skill track.
pub fn magic_skill_track_for_class(class_id: &str) -> Option<&'static str> {
    match class_id {
        "wizard" => Some("wizard_magic"),
        "thaumaturge" => Some("thaumaturge_magic"),
        "thief" => Some("thief_magic"),
        _ => None,
    }
}

impl Engine {
    pub(super) fn skill_track_is_allowed_for_actor(
        &self,
        actor_index: usize,
        track_id: &str,
    ) -> bool {
        let Some(character) = self.world.actors[actor_index].character.as_ref() else {
            return false;
        };
        let class_id = character.identity.current_class_id.as_str();
        if matches!(
            track_id,
            "wizard_magic" | "thaumaturge_magic" | "thief_magic" | "knight_magic"
        ) && magic_skill_track_for_class(class_id) != Some(track_id)
        {
            return false;
        }
        !self
            .definition
            .catalog
            .skill_catalog
            .as_ref()
            .is_some_and(|catalog| {
                catalog.track(track_id).is_none()
                    || !catalog.track_is_eligible_for_class(track_id, class_id)
            })
    }

    pub(super) fn skill_track_display(&self, track_id: &str) -> Option<String> {
        self.definition
            .catalog
            .skill_catalog
            .as_ref()
            .and_then(|catalog| catalog.track_display(track_id))
            .map(str::to_string)
    }

    pub(super) fn skill_level_title(&self, track_id: &str, level: u8) -> Option<String> {
        self.definition
            .catalog
            .skill_catalog
            .as_ref()
            .and_then(|catalog| catalog.level_title(track_id, level))
            .map(str::to_string)
    }

    pub(super) fn skill_level_for_actor(&self, actor_index: usize, track_id: &str) -> u8 {
        self.skill_entry_for_actor(actor_index, track_id)
            .map(|entry| entry.level)
            .unwrap_or(0)
    }

    pub(super) fn skill_entry_for_actor(
        &self,
        actor_index: usize,
        track_id: &str,
    ) -> Option<&SkillEntry> {
        self.world
            .actors
            .get(actor_index)?
            .character
            .as_ref()
            .and_then(|character| {
                character
                    .skill_ledger
                    .iter()
                    .find(|entry| entry.track_id == track_id)
            })
    }

    pub(super) fn highest_skill_level_in_category(
        &self,
        actor_index: usize,
        requested_track_id: &str,
    ) -> Option<u8> {
        let character = self.world.actors.get(actor_index)?.character.as_ref()?;
        let catalog = self.definition.catalog.skill_catalog.as_ref()?;
        let requested_category = training_category(catalog.track(requested_track_id)?.kind);
        let class_id = character.identity.current_class_id.as_str();

        Some(
            character
                .skill_ledger
                .iter()
                .filter(|entry| catalog.track_is_eligible_for_class(&entry.track_id, class_id))
                .filter_map(|entry| {
                    let track = catalog.track(&entry.track_id)?;
                    (training_category(track.kind) == requested_category).then_some(entry.level)
                })
                .max()
                .unwrap_or(0),
        )
    }

    pub(super) fn set_skill_learning_rate(
        &mut self,
        actor_index: usize,
        track_id: &str,
        new_learning_rate: u64,
    ) -> Result<(), StepError> {
        if !self.skill_track_is_allowed_for_actor(actor_index, track_id) {
            return Err(StepError::new(format!(
                "skill {track_id:?} is not available to this actor"
            )));
        }
        let base_learning_rate = self.definition.catalog.rules.skills.base_learning_rate;
        if new_learning_rate < base_learning_rate {
            return Err(StepError::new(format!(
                "learning rate for skill {track_id:?} must be at least the configured base"
            )));
        }
        let character = self
            .world
            .actors
            .get_mut(actor_index)
            .and_then(|actor| actor.character.as_mut())
            .ok_or_else(|| StepError::new("actor has no character sheet"))?;
        let current_learning_rate = character
            .skill_ledger
            .iter()
            .find(|entry| entry.track_id == track_id)
            .map_or(base_learning_rate, |entry| entry.learning_rate);
        if new_learning_rate <= current_learning_rate {
            return Err(StepError::new(format!(
                "learning rate for skill {track_id:?} must increase"
            )));
        }
        let entry_index = character
            .skill_ledger
            .iter()
            .position(|entry| entry.track_id == track_id)
            .unwrap_or_else(|| {
                character.skill_ledger.push(SkillEntry::untrained(
                    track_id.to_string(),
                    base_learning_rate,
                ));
                character.skill_ledger.len() - 1
            });
        let entry = &mut character.skill_ledger[entry_index];
        entry.learning_rate = new_learning_rate;
        Ok(())
    }

    pub(super) fn award_skill_practice(
        &mut self,
        actor_index: usize,
        track_id: &str,
        raw_amount: u64,
    ) -> Result<Vec<Event>, StepError> {
        if raw_amount == 0 {
            return Ok(Vec::new());
        }
        if !self.skill_track_is_allowed_for_actor(actor_index, track_id) {
            return Ok(Vec::new());
        }

        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let track_display = self.skill_track_display(track_id);
        let catalog = self.definition.catalog.skill_catalog.clone();
        let base_learning_rate = self.definition.catalog.rules.skills.base_learning_rate;
        let practice_thresholds = self
            .definition
            .catalog
            .rules
            .skills
            .practice_thresholds
            .clone();
        let current = self.skill_entry_for_actor(actor_index, track_id);
        if current.is_some_and(SkillEntry::is_maximum) {
            return Ok(Vec::new());
        }
        let learning_rate = current.map_or(base_learning_rate, |entry| entry.learning_rate);
        if learning_rate == 0 {
            return Err(StepError::new(format!(
                "skill {track_id:?} has an invalid zero learning rate"
            )));
        }
        let credited_amount = raw_amount.checked_mul(learning_rate).ok_or_else(|| {
            StepError::new(format!(
                "skill {track_id:?} practice credit must not overflow"
            ))
        })?;
        let character = self.world.actors[actor_index]
            .character
            .as_mut()
            .expect("character was checked above");
        let entry_index = character
            .skill_ledger
            .iter()
            .position(|entry| entry.track_id == track_id)
            .unwrap_or_else(|| {
                character.skill_ledger.push(SkillEntry::untrained(
                    track_id.to_string(),
                    base_learning_rate,
                ));
                character.skill_ledger.len() - 1
            });
        let entry = &mut character.skill_ledger[entry_index];
        entry.practice_points = entry
            .practice_points
            .checked_add(credited_amount)
            .ok_or_else(|| {
                StepError::new(format!(
                    "skill {track_id:?} practice pool must not overflow"
                ))
            })?;
        let awarded_total = entry.practice_points;
        let awarded_level = entry.level;
        let awarded_critique_rank = entry.critique_rank;
        let mut events = vec![Event::SkillPracticeAwarded {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            track_id: track_id.to_string(),
            track_display: track_display.clone(),
            raw_amount,
            learning_rate,
            credited_amount,
            practice_points: awarded_total,
            level: awarded_level,
            critique_rank: awarded_critique_rank,
        }];

        loop {
            if entry.is_maximum() {
                entry.practice_points = 0;
                break;
            }
            let needed = practice_thresholds
                .get(usize::from(entry.level))
                .copied()
                .ok_or_else(|| {
                    StepError::new(format!(
                        "skill {track_id:?} has no practice threshold for level {}",
                        entry.level
                    ))
                })?;
            if entry.practice_points < needed {
                break;
            }
            entry.practice_points -= needed;
            if !entry.advance_position() {
                return Err(StepError::new(format!(
                    "skill {track_id:?} could not advance from its current position"
                )));
            }
            let level_title = catalog
                .as_ref()
                .and_then(|catalog| catalog.level_title(track_id, entry.level))
                .map(str::to_string);
            events.push(Event::SkillPositionChanged {
                actor_id: actor_id.clone(),
                actor: actor_name.clone(),
                track_id: track_id.to_string(),
                track_display: track_display.clone(),
                new_level: entry.level,
                new_critique_rank: entry.critique_rank,
                level_title,
            });
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn skill_position_can_advance_far_beyond_character_level() {
        let mut engine = crate::engine::setup::test_engine("character_sheet");
        let actor_index = engine
            .world
            .actors
            .iter()
            .position(|actor| actor.id == "player")
            .expect("player fixture");
        engine.world.actors[actor_index]
            .character
            .as_mut()
            .expect("character fixture")
            .progression
            .level = 1;

        while engine.skill_level_for_actor(actor_index, "hand") < 8 {
            let skill_level = engine.skill_level_for_actor(actor_index, "hand");
            let threshold = engine.definition.catalog.rules.skills.practice_thresholds
                [usize::from(skill_level)];
            engine
                .award_skill_practice(actor_index, "hand", threshold)
                .expect("practice remains independent of character level");
        }

        let character = engine.world.actors[actor_index]
            .character
            .as_ref()
            .expect("character fixture");
        assert_eq!(character.progression.level, 1);
        assert!(
            character
                .skill_ledger
                .iter()
                .any(|entry| entry.track_id == "hand" && entry.level >= 8)
        );
    }
}
