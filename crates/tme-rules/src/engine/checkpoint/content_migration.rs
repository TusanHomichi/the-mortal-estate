//! Explicit offline content cutover. Normal recovery still requires exact identity.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointContentMigration {
    pub from_definition_sha256: String,
    pub to_definition_sha256: String,
    pub retire_npcs: BTreeSet<ActorId>,
    /// Retire these service instances and transfer every remaining listing to
    /// the named retained provider, keeping capability, price, origin and item ID.
    pub merge_merchants: BTreeMap<String, String>,
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
        let mut engine = Self::hydrate_checkpoint(before, checkpoint)?;
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
        let migrated = engine.export_checkpoint()?;
        // Reuse full recovery validation, including all item ownership, balances,
        // references, deadlines and sequence invariants. No mutable state is reseeded.
        Self::hydrate_checkpoint(after, &migrated)?;
        Ok(migrated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::setup::test_engine;

    #[test]
    fn merchant_retirement_preserves_stock_prices_and_all_item_instances() {
        let mut engine = test_engine("town_adventure_loop_gallery");
        let actor_id = ActorId::new("route_keeper");
        let mut service = engine.world.service_instances[0].clone();
        service.id = "retiring_counter".into();
        service.placement = ServicePlacement::Actor {
            actor_id: actor_id.clone(),
        };
        engine.world.service_instances.push(service);
        let item = engine.world.item_instances["bright_staff_stock"].clone();
        engine
            .world
            .item_instances
            .insert("retiring_stock".into(), item);
        engine.world.merchant_inventories.insert(
            MerchantInventoryId::new("retiring_counter", "trail_wares"),
            MerchantInventoryState {
                listings: vec![MerchantListingState {
                    item_instance_id: "retiring_stock".into(),
                    origin: MerchantListingOrigin::PawnPool,
                    price_gold: 37,
                }],
            },
        );
        let before = engine.definition().clone();
        let mut next = before.as_ref().clone();
        next.content_identity.definition_sha256 = "a".repeat(64);
        let after = Arc::new(next);
        let plan = CheckpointContentMigration {
            from_definition_sha256: before.content_identity().definition_sha256.clone(),
            to_definition_sha256: after.content_identity().definition_sha256.clone(),
            retire_npcs: BTreeSet::from([actor_id.clone()]),
            merge_merchants: BTreeMap::from([(
                "retiring_counter".into(),
                "waystation_counter".into(),
            )]),
        };
        let checkpoint = engine.export_checkpoint().unwrap();
        let migrated =
            Engine::migrate_content_checkpoint(before.clone(), after.clone(), &checkpoint, &plan)
                .unwrap();
        let result = Engine::hydrate_checkpoint(after.clone(), &migrated).unwrap();
        assert_eq!(result.world.item_instances, engine.world.item_instances);
        assert_eq!(result.world.banks, engine.world.banks);
        assert_eq!(result.world.locker_vaults, engine.world.locker_vaults);
        assert_eq!(result.world.timing, engine.world.timing);
        assert_eq!(
            result.world.actors,
            engine
                .world
                .actors
                .iter()
                .filter(|a| a.id != actor_id)
                .cloned()
                .collect::<Vec<_>>()
        );
        let listings = &result.world.merchant_inventories
            [&MerchantInventoryId::new("waystation_counter", "trail_wares")]
            .listings;
        assert!(
            listings
                .iter()
                .any(|l| l.item_instance_id == "retiring_stock"
                    && l.price_gold == 37
                    && l.origin == MerchantListingOrigin::PawnPool)
        );
        engine
            .world
            .actors
            .iter_mut()
            .find(|a| a.id == actor_id)
            .unwrap()
            .carried
            .gold
            .sack = 1;
        let burdened = engine.export_checkpoint().unwrap();
        assert!(Engine::migrate_content_checkpoint(before, after, &burdened, &plan).is_err());
    }

    #[test]
    fn explicit_rebind_preserves_every_mutable_byte_and_refuses_wrong_identity_and_player_retirement()
     {
        let engine = test_engine("first_room");
        let before = engine.definition().clone();
        let mut next = before.as_ref().clone();
        next.content_identity.definition_sha256 = "a".repeat(64);
        let after = Arc::new(next);
        let checkpoint = engine.export_checkpoint().unwrap();
        let mut plan = CheckpointContentMigration {
            from_definition_sha256: before.content_identity().definition_sha256.clone(),
            to_definition_sha256: after.content_identity().definition_sha256.clone(),
            retire_npcs: BTreeSet::new(),
            merge_merchants: BTreeMap::new(),
        };
        let output =
            Engine::migrate_content_checkpoint(before.clone(), after.clone(), &checkpoint, &plan)
                .unwrap();
        let mut expected = checkpoint.decode().unwrap();
        expected.content = after.content_identity().clone();
        assert_eq!(output.as_bytes(), serde_json::to_vec(&expected).unwrap());
        assert!(Engine::hydrate_checkpoint(before.clone(), &output).is_err());
        plan.retire_npcs
            .insert(engine.world.controlled_actors().next().unwrap().id.clone());
        assert!(
            Engine::migrate_content_checkpoint(before.clone(), after.clone(), &checkpoint, &plan)
                .is_err()
        );
        plan.from_definition_sha256 = "b".repeat(64);
        assert!(Engine::migrate_content_checkpoint(before, after, &checkpoint, &plan).is_err());
    }
}
