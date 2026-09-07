use crate::content::CharacterCreationProfileDef;
use crate::model::{ActorKind, CharacterAttributes, CharacterId};

use super::{Engine, GameDefinition, StepError, ValidatedWorldSeed};

impl GameDefinition {
    pub fn creation_profiles(&self) -> &[CharacterCreationProfileDef] {
        &self.catalog.creation_profiles
    }
}

impl Engine {
    /// Prepare a complete disconnected character without changing this world or
    /// consuming its RNG. The server commits the candidate with its directory row.
    pub fn prepare_character_creation(
        &self,
        profile_id: &str,
        character_id: CharacterId,
        display_name: &str,
        attributes: CharacterAttributes,
    ) -> Result<Self, StepError> {
        let profile = self
            .definition
            .creation_profiles()
            .iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| StepError::new("unknown character creation profile"))?;
        if display_name.trim().is_empty()
            || character_id.as_str().is_empty()
            || self
                .world
                .actors
                .iter()
                .any(|actor| actor.character_id.as_ref() == Some(&character_id))
        {
            return Err(StepError::new("invalid or duplicate character identity"));
        }
        let seed = profile
            .materialize(
                &character_id,
                attributes,
                self.definition
                    .world_template
                    .arrivals
                    .get(&profile.arrival_id),
                self.definition
                    .catalog
                    .actor_definitions
                    .get(&profile.actor_definition_id)
                    .is_some_and(|actor| actor.kind == ActorKind::Player),
                |id| {
                    self.definition
                        .catalog
                        .item_catalog
                        .get(id)
                        .and_then(|item| item.capability.as_ref())
                        .is_some_and(|capability| capability.spell_book_for.is_some())
                },
            )
            .map_err(|error| StepError::new(error.to_string()))?;
        let validated = ValidatedWorldSeed::new(self.definition.clone(), seed)
            .map_err(|error| StepError::new(error.to_string()))?;
        let mut created = Engine::new(validated, 0)?;
        let mut actor = created.world.actors.remove(0);
        if self.world.actor(&actor.id).is_some()
            || created
                .world
                .item_instances
                .keys()
                .any(|id| self.world.item_instances.contains_key(id))
        {
            return Err(StepError::new(
                "created actor or item identity already exists",
            ));
        }
        let mut candidate = self.clone();
        let now = candidate.world.timing.now;
        actor.name = display_name.to_string();
        actor.timing.ready_at = now;
        actor.attack_ready_at = now;
        actor.resource_activity.last_recovered_at = now;
        actor.timing.tie_break_order = candidate.world.timing.next_tie_break_order;
        candidate.world.timing.next_tie_break_order =
            actor
                .timing
                .tie_break_order
                .checked_add(1)
                .ok_or_else(|| StepError::new("character scheduling order exhausted"))?;
        candidate.world.actors.push(actor);
        candidate
            .world
            .item_instances
            .extend(created.world.item_instances);
        candidate
            .world
            .communication_preferences
            .extend(created.world.communication_preferences);
        candidate.world.character_presence.insert(
            character_id,
            crate::model::CharacterPresenceState {
                connected: false,
                control_epoch: 0,
                absent_since: Some(now),
            },
        );
        candidate.validate_world_item_locations()?;
        candidate.validate_world_item_burden()?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests;
