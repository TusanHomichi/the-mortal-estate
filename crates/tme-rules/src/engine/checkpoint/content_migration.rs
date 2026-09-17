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
    /// Actor definition IDs whose authored **combat ratings** are re-derived
    /// from the destination definition for every retained actor that uses them.
    /// Only `attack` and `defense` move: identity, kind, health, resources, life
    /// state, progression, inventory, balances, timing and every other mutable
    /// fact are untouched, and an actor whose retained ratings already match the
    /// destination is left byte-identical. A definition whose authored health
    /// also moved must additionally be declared in `rebuild_actor_health`.
    pub rederive_actor_stats: BTreeSet<String>,
    /// Actor definition IDs whose authored **health pool** is re-derived from
    /// the destination definition for every retained actor that uses them.
    ///
    /// This is a separate declaration because a health pool is not a rating: it
    /// is both an immutable authored maximum and the ceiling of a mutable
    /// current value, and the cutover has to say what happens to the second one.
    /// The applied policy is [`ActorHealthPolicy::PreserveCurrent`], recorded
    /// here so the plan, the receipt and the tests all name the same rule.
    pub rebuild_actor_health: BTreeSet<String>,
}

/// What a content cutover does to a retained actor's current health when the
/// authored health pool moves.
///
/// `PreserveCurrent` keeps every retained actor's current health exactly as the
/// checkpoint holds it — a wounded actor stays exactly as wounded, in absolute
/// points — and raises the ceiling to the destination's authored maximum. It
/// never heals a living actor to the new maximum, never lowers a living actor's
/// current health, and never revives a dead one. Current health is clamped down
/// only when the destination authors a smaller pool than the actor currently
/// holds, which no other field can absorb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorHealthPolicy {
    PreserveCurrent,
}

impl ActorHealthPolicy {
    pub const APPLIED: Self = Self::PreserveCurrent;
}

/// How one retained actor's health was affected by a declared rebuild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorHealthRebuild {
    pub definition_id: String,
    pub maximum_before: i32,
    pub maximum_after: i32,
    pub current_before: i32,
    pub current_after: i32,
}

impl CheckpointContentMigration {
    /// Actor definition IDs present in **both** catalogs whose authored combat
    /// ratings differ. Destination-only definitions are additions and no
    /// retained actor can reference them, so they need no declaration; a
    /// source-only definition is a removal, and any actor still referencing it
    /// is refused separately.
    fn moved_actor_stat_definitions(
        before: &GameDefinition,
        after: &GameDefinition,
    ) -> BTreeSet<String> {
        after
            .catalog
            .actor_definitions
            .iter()
            .filter_map(|(definition_id, destination)| {
                let source = before.catalog.actor_definitions.get(definition_id)?;
                let moved = source.stats.attack != destination.stats.attack
                    || source.stats.defense != destination.stats.defense;
                moved.then(|| definition_id.clone())
            })
            .collect()
    }

    /// Actor definition IDs present in both catalogs whose authored health pool
    /// differs. Kept apart from the rating movement so a cutover that only
    /// retunes attack or defense never has to declare a health policy.
    fn moved_actor_health_definitions(
        before: &GameDefinition,
        after: &GameDefinition,
    ) -> BTreeSet<String> {
        after
            .catalog
            .actor_definitions
            .iter()
            .filter_map(|(definition_id, destination)| {
                let source = before.catalog.actor_definitions.get(definition_id)?;
                (source.stats.hp != destination.stats.hp).then(|| definition_id.clone())
            })
            .collect()
    }

    /// Reject a declaration that names something which did not move, and a
    /// movement nobody declared. Retained values cannot be rescued later:
    /// ordinary recovery still requires exact content identity, so an undeclared
    /// movement is a silent rules split.
    fn declared_movements(
        declared: &BTreeSet<String>,
        moved: &BTreeSet<String>,
        what: &str,
    ) -> Result<(), CheckpointError> {
        for definition_id in declared {
            if !moved.contains(definition_id) {
                return Err(CheckpointError::new(format!(
                    "declared actor definition {definition_id:?} did not change its {what}"
                )));
            }
        }
        if let Some(definition_id) = moved.iter().find(|id| !declared.contains(*id)) {
            return Err(CheckpointError::new(format!(
                "content migration leaves {definition_id:?} on a superseded {what}"
            )));
        }
        Ok(())
    }

    /// Re-derive the declared combat ratings and return the declared health
    /// rebuilds. Every retained actor must reference a definition the
    /// destination still authors: a source-only definition cannot be reconciled
    /// by any policy here and leaves the actor without authored rules.
    fn apply_actor_stat_reconciliation(
        &self,
        before: &GameDefinition,
        after: &GameDefinition,
        engine: &mut Engine,
    ) -> Result<Vec<ActorHealthRebuild>, CheckpointError> {
        let ratings = Self::moved_actor_stat_definitions(before, after);
        Self::declared_movements(&self.rederive_actor_stats, &ratings, "combat rating")?;
        let health = Self::moved_actor_health_definitions(before, after);
        Self::declared_movements(&self.rebuild_actor_health, &health, "health pool")?;
        let mut rebuilds = Vec::new();
        for actor in &mut engine.world.actors {
            let destination = after
                .catalog
                .actor_definitions
                .get(&actor.definition_id)
                .ok_or_else(|| {
                    CheckpointError::new(format!(
                        "content migration removes the definition of retained actor {:?}",
                        actor.id
                    ))
                })?;
            if ratings.contains(&actor.definition_id) {
                actor.stats.attack = destination.stats.attack;
                actor.stats.defense = destination.stats.defense;
            }
            if !health.contains(&actor.definition_id) {
                continue;
            }
            if let Some(rebuild) = Self::apply_actor_health_policy(
                actor,
                destination.stats.hp,
                ActorHealthPolicy::APPLIED,
            ) {
                rebuilds.push(rebuild);
            }
        }
        Ok(rebuilds)
    }

    /// Apply the authored health ceiling to one retained actor under
    /// [`ActorHealthPolicy::PreserveCurrent`].
    fn apply_actor_health_policy(
        actor: &mut ActorState,
        authored_maximum_after: i32,
        policy: ActorHealthPolicy,
    ) -> Option<ActorHealthRebuild> {
        let ActorHealthPolicy::PreserveCurrent = policy;
        let maximum_before = actor.max_hp();
        let current_before = actor.hp;
        // A dead actor keeps its life state and its zero: a health-pool change
        // is not a resurrection, and defeat cleanup owns everything after it.
        let current_after = if current_before <= 0 {
            0
        } else {
            // The authored ceiling moved, so the actor keeps the health it has.
            // A larger pool leaves the wound exactly as large in absolute terms;
            // a smaller pool is the only case that can lower the current value.
            current_before.min(authored_maximum_after).max(1)
        };
        actor.stats.hp = authored_maximum_after;
        actor.hp = current_after;
        if let Some(character) = actor.character.as_mut() {
            character.resources.max_hp = authored_maximum_after;
            // `peak_hp` is the highest authored ceiling the character has held
            // and validation requires `max_hp <= peak_hp`; a larger pool raises
            // it, a smaller one keeps the record of what was reached.
            character.resources.peak_hp = character.resources.peak_hp.max(authored_maximum_after);
            character.resources.hp = current_after;
        }
        (maximum_before != authored_maximum_after || current_before != current_after).then(|| {
            ActorHealthRebuild {
                definition_id: actor.definition_id.clone(),
                maximum_before,
                maximum_after: authored_maximum_after,
                current_before,
                current_after,
            }
        })
    }
}

impl Engine {
    pub fn migrate_content_checkpoint(
        before: Arc<GameDefinition>,
        after: Arc<GameDefinition>,
        checkpoint: &FacetCheckpointV5,
        plan: &CheckpointContentMigration,
    ) -> Result<FacetCheckpointV5, CheckpointError> {
        Self::migrate_content_checkpoint_reported(before, after, checkpoint, plan)
            .map(|(migrated, _)| migrated)
    }

    /// The same cutover, also returning what the declared health policy did to
    /// each retained actor. Deployment uses the report-free form; proofs and
    /// operators read this one when they need to show the policy's effect.
    pub fn migrate_content_checkpoint_reported(
        before: Arc<GameDefinition>,
        after: Arc<GameDefinition>,
        checkpoint: &FacetCheckpointV5,
        plan: &CheckpointContentMigration,
    ) -> Result<(FacetCheckpointV5, Vec<ActorHealthRebuild>), CheckpointError> {
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
        // Authored combat ratings are immutable definition facts, so no
        // retained actor may keep a rating the destination definition no longer
        // authors. Every movement is declared before any state is written, and a
        // declared health-pool change names its own policy rather than borrowing
        // the rating operation.
        let health_rebuilds =
            plan.apply_actor_stat_reconciliation(before.as_ref(), after.as_ref(), &mut engine)?;
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
        Ok((migrated, health_rebuilds))
    }
}

#[cfg(test)]
#[path = "content_relocation_tests.rs"]
mod content_relocation_tests;
