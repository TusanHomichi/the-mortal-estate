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
        relocations: vec![],
        initialize_new_topology: false,
        retire_npcs: BTreeSet::from([actor_id.clone()]),
        merge_merchants: BTreeMap::from([("retiring_counter".into(), "waystation_counter".into())]),
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
fn explicit_rebind_preserves_every_mutable_byte_and_refuses_wrong_identity_and_player_retirement() {
    let engine = test_engine("first_room");
    let before = engine.definition().clone();
    let mut next = before.as_ref().clone();
    next.content_identity.definition_sha256 = "a".repeat(64);
    let after = Arc::new(next);
    let checkpoint = engine.export_checkpoint().unwrap();
    let mut plan = CheckpointContentMigration {
        from_definition_sha256: before.content_identity().definition_sha256.clone(),
        to_definition_sha256: after.content_identity().definition_sha256.clone(),
        relocations: vec![],
        initialize_new_topology: false,
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

fn expanded(engine: &Engine, realm: &str, level: &str, offset: Coord) -> Arc<GameDefinition> {
    let mut next = engine.definition().as_ref().clone();
    next.content_identity.definition_sha256 = "c".repeat(64);
    let room = next
        .world_template
        .realms
        .get_mut(realm)
        .unwrap()
        .levels
        .get_mut(level)
        .unwrap();
    let old = room.cells.clone();
    let width = room.width + offset.x + 1;
    let height = room.height + offset.y + 1;
    let mut cells = vec![vec![old[0][0].clone(); width as usize]; height as usize];
    for (y, row) in old.into_iter().enumerate() {
        for (x, cell) in row.into_iter().enumerate() {
            cells[y + offset.y as usize][x + offset.x as usize] = cell;
        }
    }
    room.width = width;
    room.height = height;
    room.cells = cells;
    let shift = |p: &mut WorldPosition| {
        if p.realm == realm && p.level == level {
            p.position.x += offset.x;
            p.position.y += offset.y;
        }
    };
    next.world_template.navigation = next
        .world_template
        .navigation
        .into_iter()
        .map(|(mut at, mut edges)| {
            shift(&mut at);
            for edge in &mut edges {
                shift(&mut edge.target);
            }
            (at, edges)
        })
        .collect();
    for p in next.world_template.arrivals.values_mut() {
        shift(p);
    }
    Arc::new(next)
}

fn relocation_plan(
    engine: &Engine,
    after: &GameDefinition,
    realm: &str,
    level: &str,
    offset: Coord,
) -> CheckpointContentMigration {
    CheckpointContentMigration {
        from_definition_sha256: engine
            .definition()
            .content_identity()
            .definition_sha256
            .clone(),
        to_definition_sha256: after.content_identity().definition_sha256.clone(),
        retire_npcs: BTreeSet::new(),
        merge_merchants: BTreeMap::new(),
        relocations: vec![ContentRelocation {
            realm: realm.into(),
            level: level.into(),
            offset,
        }],
        initialize_new_topology: false,
    }
}

#[test]
fn expanded_map_translates_live_positions_and_preserves_every_other_checkpoint_fact() {
    let mut engine = test_engine("first_room");
    let member = engine.world.actors[0].location.clone();
    let player = engine.world.controlled_actors().next().unwrap().clone();
    if let Some(ai) = engine
        .world
        .actors
        .iter_mut()
        .find_map(|actor| actor.ai.as_mut())
    {
        ai.awareness.remembered = Some(RememberedHostile {
            actor_id: player.id,
            last_seen: player.location,
            remaining_opportunities: 2,
        });
    }
    let offset = Coord { x: 19, y: 3 };
    let after = expanded(&engine, &member.realm, &member.level, offset);
    let plan = relocation_plan(&engine, &after, &member.realm, &member.level, offset);
    let before = engine.export_checkpoint().unwrap();
    let output = Engine::migrate_content_checkpoint(
        engine.definition().clone(),
        after.clone(),
        &before,
        &plan,
    )
    .unwrap();
    let hydrated = Engine::hydrate_checkpoint(after, &output).unwrap();
    assert_eq!(hydrated.export_checkpoint().unwrap(), output);
    // Independently transform the serialized expected world by the address
    // shape, rather than calling the typed migration's field visitor. A missed
    // typed location (including AI memory) makes the entire-world comparison fail.
    let mut expected: serde_json::Value = serde_json::from_slice(before.as_bytes()).unwrap();
    fn shift(value: &mut serde_json::Value, member: &WorldPosition, offset: Coord) {
        match value {
            serde_json::Value::Object(map)
                if map.get("realm") == Some(&serde_json::json!(member.realm))
                    && map.get("level") == Some(&serde_json::json!(member.level))
                    && map.contains_key("position") =>
            {
                let p = map.get_mut("position").unwrap();
                p["x"] = serde_json::json!(p["x"].as_i64().unwrap() + i64::from(offset.x));
                p["y"] = serde_json::json!(p["y"].as_i64().unwrap() + i64::from(offset.y));
            }
            serde_json::Value::Object(map) => {
                for value in map.values_mut() {
                    shift(value, member, offset);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    shift(value, member, offset);
                }
            }
            _ => {}
        }
    }
    shift(&mut expected["world"], &member, offset);
    let actual: serde_json::Value = serde_json::from_slice(output.as_bytes()).unwrap();
    assert_eq!(actual["world"], expected["world"]);
    expected["content"] = actual["content"].clone();
    assert_eq!(actual, expected);
    assert_eq!(hydrated.initial_events, engine.initial_events);
    assert!(Engine::hydrate_checkpoint(engine.definition().clone(), &output).is_err());
}

#[test]
fn migration_refuses_duplicate_zero_overflow_and_out_of_bounds_relocations() {
    let engine = test_engine("first_room");
    let member = engine.world.actors[0].location.clone();
    let offset = Coord { x: 2, y: 3 };
    let after = expanded(&engine, &member.realm, &member.level, offset);
    let plan = relocation_plan(&engine, &after, &member.realm, &member.level, offset);
    let checkpoint = engine.export_checkpoint().unwrap();
    for kind in 0..5 {
        let mut bad = plan.clone();
        match kind {
            0 => bad.relocations.push(bad.relocations[0].clone()),
            1 => bad.relocations[0].offset = Coord { x: 0, y: 0 },
            2 => bad.relocations[0].offset.x = i32::MAX,
            3 => bad.relocations[0].offset.x = -1,
            _ => bad.relocations[0].level = "absent".into(),
        }
        assert!(
            Engine::migrate_content_checkpoint(
                engine.definition().clone(),
                after.clone(),
                &checkpoint,
                &bad
            )
            .is_err()
        );
        assert_eq!(engine.export_checkpoint().unwrap(), checkpoint);
    }
    let old_shape = serde_json::json!({"from_definition_sha256": plan.from_definition_sha256, "to_definition_sha256": plan.to_definition_sha256, "retire_npcs": [], "merge_merchants": {}});
    assert!(serde_json::from_value::<CheckpointContentMigration>(old_shape).is_err());
}

#[test]
fn new_topology_requires_explicit_initialization_and_existing_open_state_survives() {
    let mut engine = test_engine("world_topology_gallery");
    let old = engine.world.door_states.keys().next().unwrap().clone();
    engine.world.door_states.insert(old.clone(), true);
    let mut next = engine.definition().as_ref().clone();
    next.content_identity.definition_sha256 = "d".repeat(64);
    let new_at = WorldPosition::new("realm_0", "hidden_room", Coord { x: 1, y: 1 });
    assert!(!next.world_template.navigation.contains_key(&new_at));
    next.world_template.navigation.insert(
        new_at.clone(),
        vec![NavigationDef {
            kind: NavigationKind::Door,
            target: new_at.clone(),
            initial_state: Some(DoorState::Closed),
            hidden: true,
        }],
    );
    let after = Arc::new(next);
    let mut plan = CheckpointContentMigration {
        from_definition_sha256: engine
            .definition()
            .content_identity()
            .definition_sha256
            .clone(),
        to_definition_sha256: after.content_identity().definition_sha256.clone(),
        retire_npcs: BTreeSet::new(),
        merge_merchants: BTreeMap::new(),
        relocations: vec![],
        initialize_new_topology: false,
    };
    let checkpoint = engine.export_checkpoint().unwrap();
    assert!(
        Engine::migrate_content_checkpoint(
            engine.definition().clone(),
            after.clone(),
            &checkpoint,
            &plan
        )
        .is_err()
    );
    plan.initialize_new_topology = true;
    let result = Engine::migrate_content_checkpoint(
        engine.definition().clone(),
        after.clone(),
        &checkpoint,
        &plan,
    )
    .unwrap();
    let hydrated = Engine::hydrate_checkpoint(after, &result).unwrap();
    assert!(hydrated.world.door_states[&old]);
    assert!(!hydrated.world.door_states[&new_at]);
    assert!(!hydrated.world.hidden_transition_revealed[&new_at]);
    assert_eq!(hydrated.world.actors, engine.world.actors);
    assert_eq!(hydrated.world.item_instances, engine.world.item_instances);
}

#[test]
fn npc_patrols_follow_their_original_home_member_and_keep_their_progress() {
    let mut engine = test_engine("town_adventure_loop_gallery");
    let patrol = engine
        .world
        .actors
        .iter_mut()
        .find(|a| a.npc.is_some())
        .unwrap()
        .npc
        .as_mut()
        .unwrap();
    patrol.patrol = vec![Coord { x: 2, y: 1 }, Coord { x: 3, y: 1 }];
    patrol.patrol_next = 1;
    let npc = engine
        .world
        .actors
        .iter()
        .find(|a| a.npc.as_ref().is_some_and(|n| !n.patrol.is_empty()))
        .unwrap();
    let member = npc.home_location.clone();
    let offset = Coord { x: 2, y: 3 };
    let after = expanded(&engine, &member.realm, &member.level, offset);
    let plan = relocation_plan(&engine, &after, &member.realm, &member.level, offset);
    let checkpoint = engine.export_checkpoint().unwrap();
    let output = Engine::migrate_content_checkpoint(
        engine.definition().clone(),
        after.clone(),
        &checkpoint,
        &plan,
    )
    .unwrap();
    let hydrated = Engine::hydrate_checkpoint(after, &output).unwrap();
    let retained = hydrated.world.actor(&npc.id).unwrap().npc.as_ref().unwrap();
    let previous = npc.npc.as_ref().unwrap();
    assert_eq!(retained.patrol_next, previous.patrol_next);
    assert_eq!(
        retained.following_character_id,
        previous.following_character_id
    );
    assert_eq!(
        retained.patrol,
        previous
            .patrol
            .iter()
            .map(|p| Coord {
                x: p.x + 2,
                y: p.y + 3
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(hydrated.world.item_instances, engine.world.item_instances);
    assert_eq!(hydrated.world.timing, engine.world.timing);
    assert_eq!(hydrated.initial_events, engine.initial_events);
}
