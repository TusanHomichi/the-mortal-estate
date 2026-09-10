//! Explicit offline content cutover. Normal recovery still requires exact identity.
use super::*;

#[path = "content_relocation.rs"]
mod content_relocation;
pub use content_relocation::ContentRelocation;
use content_relocation::apply_content_relocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointContentMigration {
    pub from_definition_sha256: String,
    pub to_definition_sha256: String,
    pub retire_npcs: BTreeSet<ActorId>,
    /// Retire these service instances and transfer every remaining listing to
    /// the named retained provider, keeping capability, price, origin and item ID.
    pub merge_merchants: BTreeMap<String, String>,
    /// Authored members that keep their identity while their coordinates move.
    /// Every typed live spatial field in these members is translated by the
    /// declared offset.
    pub relocations: Vec<ContentRelocation>,
    /// Whether authored topology introduced by the destination definition may
    /// seed absent door and hidden-transition state. Existing mutated state is
    /// always preserved; new keys are inserted only when this is true.
    pub initialize_new_topology: bool,
}

impl Engine {
    pub fn migrate_content_checkpoint(
        before: Arc<GameDefinition>,
        after: Arc<GameDefinition>,
        checkpoint: &FacetCheckpointV5,
        plan: &CheckpointContentMigration,
    ) -> Result<FacetCheckpointV5, CheckpointError> {
        if before.content_identity().definition_sha256 != plan.from_definition_sha256
            || after.content_identity().definition_sha256 != plan.to_definition_sha256
            || plan.from_definition_sha256 == plan.to_definition_sha256
        {
            return Err(CheckpointError::new("content migration identity differs"));
        }
        let mut engine = Self::hydrate_checkpoint(before.clone(), checkpoint)?;
        for id in &plan.retire_npcs {
            let actor = engine
                .world
                .actors
                .iter()
                .find(|a| &a.id == id)
                .ok_or_else(|| CheckpointError::new("retired NPC is absent"))?;
            if actor.life_state != ActorLifeState::Alive
                || actor.kind != ActorKind::Npc
                || actor.character_id.is_some()
                || actor.character.is_some()
                || !actor.carried.items.is_empty()
                || actor.carried.gold != CarriedGold::default()
                || actor.ecology_origin.is_some()
                || actor.summoned.is_some()
                || !actor.active_effects.is_empty()
                || actor.warmed_spell.is_some()
                || actor.balm_effect.is_some()
            {
                return Err(CheckpointError::new(
                    "NPC retirement would discard owned state",
                ));
            }
        }
        for (source, target) in &plan.merge_merchants {
            if source == target
                || plan.merge_merchants.contains_key(target)
                || !engine
                    .world
                    .service_instances
                    .iter()
                    .any(|s| &s.id == target)
            {
                return Err(CheckpointError::new(
                    "merchant migration target is not retained",
                ));
            }
            let service = engine
                .world
                .service_instances
                .iter()
                .find(|s| &s.id == source)
                .ok_or_else(|| CheckpointError::new("retired service is absent"))?;
            let retiring_provider = match &service.placement {
                ServicePlacement::Actor { actor_id } => plan.retire_npcs.contains(actor_id),
                ServicePlacement::Fixed { location } => engine
                    .world
                    .actors
                    .iter()
                    .any(|a| plan.retire_npcs.contains(&a.id) && &a.location == location),
            };
            if !retiring_provider {
                return Err(CheckpointError::new(
                    "retired service is not owned by a retired NPC",
                ));
            }
            let keys: Vec<_> = engine
                .world
                .merchant_inventories
                .keys()
                .filter(|k| &k.service_id == source)
                .cloned()
                .collect();
            if keys.is_empty() {
                return Err(CheckpointError::new("retired merchant has no inventory"));
            }
            for key in keys {
                let inventory = engine.world.merchant_inventories.remove(&key).unwrap();
                let target_key = MerchantInventoryId::new(target, &key.capability_id);
                let destination = engine
                    .world
                    .merchant_inventories
                    .get_mut(&target_key)
                    .ok_or_else(|| {
                        CheckpointError::new("retained merchant capability is absent")
                    })?;
                destination.listings.extend(inventory.listings);
            }
        }
        engine
            .world
            .actors
            .retain(|a| !plan.retire_npcs.contains(&a.id));
        engine
            .world
            .service_instances
            .retain(|s| !plan.merge_merchants.contains_key(&s.id));
        // Translate every typed live spatial field for the authored members
        // that moved, then reconcile keyed topology state against the
        // destination definition before the actor passability refusal runs.
        apply_content_relocation(before.as_ref(), after.as_ref(), plan, &mut engine.world)?;
        // Retained actors and their home positions must remain usable. A layout
        // requiring relocation needs its own explicit migration plan and proof.
        for actor in &engine.world.actors {
            for position in [&actor.location, &actor.home_location] {
                if after.world_position_status(position) != Some(SeedWorldPositionStatus::Passable)
                {
                    return Err(CheckpointError::new(
                        "content migration requires actor relocation",
                    ));
                }
            }
        }
        engine.definition = after.clone();
        // `initial_events` are immutable bootstrap history: they are carried
        // through migration byte-identical and are never rewritten or reseeded.
        // The binding from a checkpoint to the source definition that produced
        // that history is recorded outside the checkpoint by the deployment
        // process.
        let migrated = engine.export_checkpoint()?;
        // Reuse full recovery validation, including all item ownership, balances,
        // references, deadlines and sequence invariants. No mutable state is reseeded.
        Self::hydrate_checkpoint(after, &migrated)?;
        Ok(migrated)
    }
}

#[cfg(test)]
#[path = "content_relocation_tests.rs"]
mod content_relocation_tests;
