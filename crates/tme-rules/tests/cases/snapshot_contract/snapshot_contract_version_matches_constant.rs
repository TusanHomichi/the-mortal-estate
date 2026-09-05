use super::*;

#[test]
fn snapshot_contract_version_matches_constant() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    assert_eq!(
        snapshot.contract_version,
        tme_rules::SNAPSHOT_CONTRACT_VERSION
    );
    assert!(matches!(
        snapshot.scope,
        tme_rules::SnapshotScopeV1::OmniscientLocal
    ));
}

#[test]
fn debug_30_ecology_slot_state_is_exact_ordered_and_strict() {
    assert_eq!(SNAPSHOT_CONTRACT_VERSION, 31);
    assert_eq!(OBSERVED_SNAPSHOT_CONTRACT_VERSION, 30);
    let engine = ContentParts::tracked(
        "creature_ecology_gallery",
        "profile/creature_ecology_gallery",
    )
    .engine(7)
    .expect("ecology gallery starts");
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.contract_version, 31);
    assert_eq!(
        snapshot
            .ecology_sites
            .iter()
            .map(|site| site.site_id.as_str())
            .collect::<Vec<_>>(),
        ["gallery_lair", "gallery_pack"]
    );

    let serialized = serde_json::to_value(&snapshot).expect("debug snapshot serializes");
    assert_eq!(
        serialized["ecology_sites"][0],
        serde_json::json!({
            "site_id": "gallery_lair",
            "spawn_group_id": "gallery_lair_group",
            "generation": 0,
            "full_clear_due_at": null,
            "member_slots": [{
                "member_id": "burrower",
                "location": {
                    "realm": "realm_0",
                    "level": "room_0",
                    "position": {"x": 2, "y": 1}
                },
                "actor_id": "ecology:gallery_lair:burrower:0",
                "vacant": false,
                "due_at": null
            }]
        })
    );
    assert_eq!(
        serde_json::from_value::<WorldSnapshotV1>(serialized.clone())
            .expect("debug snapshot decodes"),
        snapshot
    );

    let mut unknown = serialized.clone();
    unknown["ecology_sites"][0]["member_slots"][0]["unknown"] = serde_json::json!("rejected");
    assert!(serde_json::from_value::<WorldSnapshotV1>(unknown).is_err());

    let mut missing = serialized;
    missing["ecology_sites"][0]["member_slots"][0]
        .as_object_mut()
        .expect("member slot object")
        .remove("due_at");
    assert!(serde_json::from_value::<WorldSnapshotV1>(missing).is_err());
}

#[test]
fn snapshot_has_player_actor_id() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.controlled_actor_ids, ["player"]);
    // The player actor entry should exist and match
    let player_view = snapshot
        .actors
        .iter()
        .find(|a| a.id == snapshot.controlled_actor_ids[0])
        .expect("snapshot actors must include the current player");
    assert_eq!(player_view.kind, tme_rules::ActorKind::Player);
}

#[test]
fn debug_25_exposes_stable_character_identity_without_observed_leakage() {
    let engine = ContentParts::tracked("character_sheet", "profile/character_sheet")
        .engine(7)
        .expect("engine should start");
    let debug = engine.snapshot();
    let player = debug
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("debug player");
    assert_eq!(
        player.character_id.as_ref().map(|id| id.as_str()),
        Some("character:character_sheet:primary")
    );
    assert_eq!(
        debug
            .actors
            .iter()
            .find(|actor| actor.id == "mireling")
            .expect("debug inherent actor")
            .character_id,
        None
    );

    let observed = serde_json::to_value(
        engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot"),
    )
    .expect("observed JSON");
    assert!(
        observed["actors"]
            .as_array()
            .expect("observed actors")
            .iter()
            .all(|actor| actor.get("character_id").is_none())
    );
}

#[test]
fn snapshot_is_deterministic() {
    let parts = ContentParts::tracked("first_room", "profile/first_room");
    let engine_a = parts.engine(7).expect("engine should start");
    let engine_b = parts.engine(7).expect("engine should start");
    let snap_a = engine_a.snapshot();
    let snap_b = engine_b.snapshot();
    assert_eq!(snap_a, snap_b);
}

#[test]
fn snapshot_serializes_to_deterministic_json() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    let snap_a = engine.snapshot();
    let snap_b = engine.snapshot();
    let json_a = serde_json::to_string_pretty(&snap_a).expect("serialize");
    let json_b = serde_json::to_string_pretty(&snap_b).expect("serialize");
    assert_eq!(json_a, json_b);
}

#[test]
fn snapshot_includes_all_levels() {
    let engine = ContentParts::tracked("undercroft_loop", "profile/undercroft_loop")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    assert!(
        snapshot.realms[0].levels.len() >= 2,
        "undercroft_loop has multiple levels"
    );
    // Levels should be sorted by id within their realm.
    for i in 1..snapshot.realms[0].levels.len() {
        assert!(
            snapshot.realms[0].levels[i - 1].id <= snapshot.realms[0].levels[i].id,
            "levels must be sorted by id"
        );
    }
}

#[test]
fn snapshot_actors_sorted_by_id() {
    let engine = ContentParts::tracked("kobold_warren", "profile/kobold_warren")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    for i in 1..snapshot.actors.len() {
        assert!(
            snapshot.actors[i - 1].id <= snapshot.actors[i].id,
            "actors must be sorted by id"
        );
    }
}

#[test]
fn snapshot_tiles_are_row_major() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    for realm in &snapshot.realms {
        for level in &realm.levels {
            for i in 1..level.tiles.len() {
                let prev = &level.tiles[i - 1];
                let curr = &level.tiles[i];
                // Row-major: earlier row comes first, or same row earlier column
                assert!(
                    prev.position.y < curr.position.y
                        || (prev.position.y == curr.position.y
                            && prev.position.x < curr.position.x),
                    "tiles must be row-major"
                );
            }
        }
    }
}

#[test]
fn snapshot_ground_items_sorted_by_world_position_and_id() {
    let engine = ContentParts::tracked("supply_cache", "profile/supply_cache")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    for i in 1..snapshot.ground_items.len() {
        let a = &snapshot.ground_items[i - 1];
        let b = &snapshot.ground_items[i];
        let key_a = (
            &a.location.realm,
            &a.location.level,
            a.location.position.y,
            a.location.position.x,
            &a.item.item_instance_id,
        );
        let key_b = (
            &b.location.realm,
            &b.location.level,
            b.location.position.y,
            b.location.position.x,
            &b.item.item_instance_id,
        );
        assert!(
            key_a <= key_b,
            "ground items must be sorted by (realm, level, y, x, item_instance_id)"
        );
    }
}

#[test]
fn snapshot_includes_movement_and_magic_rules() {
    let engine = ContentParts::tracked("terrain_movement", "profile/terrain_movement")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    assert!(snapshot.rules.movement.controlled_path_points > 0);
    assert!(snapshot.rules.movement.automatic_step_points > 0);
    assert_eq!(snapshot.rules.magic.warmup.units, 1);
    assert_eq!(
        snapshot.rules.magic.warmup.evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(
        snapshot.rules.magic.damage_interruption.comparison,
        tme_rules::DamageInterruptionComparisonViewV1::StrictlyGreater
    );
    assert_eq!(snapshot.rules.magic.damage_interruption.numerator, 1);
    assert_eq!(snapshot.rules.magic.damage_interruption.denominator, 5);
    assert_eq!(
        snapshot.rules.magic.damage_interruption.evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(snapshot.rules.magic.casting_practice.minimum_raw_points, 1);
    assert_eq!(snapshot.rules.magic.casting_practice.raw_points_per_mp, 1);
    assert_eq!(
        snapshot
            .rules
            .magic
            .casting_practice
            .primary_attribute_points_per_bonus,
        10
    );
    assert_eq!(
        snapshot.rules.magic.casting_practice.evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(snapshot.rules.magic.thaum_above_skill.roll_denominator, 20);
    assert_eq!(
        snapshot
            .rules
            .magic
            .thaum_above_skill
            .penalty_per_missing_level,
        1
    );
    assert_eq!(
        snapshot
            .rules
            .magic
            .thaum_above_skill
            .minimum_success_threshold,
        1
    );
    assert_eq!(
        snapshot.rules.magic.thaum_above_skill.evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(snapshot.rules.magic.kill_experience.directed.numerator, 1);
    assert_eq!(snapshot.rules.magic.kill_experience.directed.denominator, 1);
    assert_eq!(
        snapshot
            .rules
            .magic
            .kill_experience
            .area_or_illusion
            .numerator,
        2
    );
    assert_eq!(
        snapshot
            .rules
            .magic
            .kill_experience
            .area_or_illusion
            .denominator,
        5
    );
    assert_eq!(
        snapshot.rules.magic.kill_experience.fraction_evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(
        snapshot.rules.magic.kill_experience.rounding,
        tme_rules::view::MagicArithmeticRoundingViewV1::Down
    );
    assert_eq!(
        snapshot.rules.magic.kill_experience.rounding_evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(
        snapshot.rules.magic.mp_recovery.active_item_policy,
        tme_rules::view::ActiveMpRecoveryItemPolicyViewV1::HighestMultiplier
    );
    assert_eq!(
        snapshot.rules.magic.mp_recovery.rounding,
        tme_rules::view::MagicArithmeticRoundingViewV1::Down
    );
    assert_eq!(
        snapshot.rules.magic.mp_recovery.evidence_state,
        tme_rules::MagicRuleEvidenceStateViewV1::OriginalProvisional
    );
    assert_eq!(
        engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot")
            .rules
            .magic,
        snapshot.rules.magic
    );
}

#[test]
fn snapshot_reflects_door_state_after_open() {
    let mut engine = ContentParts::tracked("undercroft_loop", "profile/undercroft_loop")
        .engine(7)
        .expect("engine should start");

    // Find a door direction from the initial snapshot
    let initial = engine.snapshot();
    let player_location = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player must exist")
        .location
        .clone();
    let door_tile = initial
        .realms
        .iter()
        .find(|realm| realm.id == player_location.realm)
        .into_iter()
        .flat_map(|realm| realm.levels.iter())
        .find(|level| level.id == player_location.level)
        .into_iter()
        .flat_map(|level| level.tiles.iter())
        .find(|t| {
            matches!(
                t.transition.as_ref().map(|tr| tr.kind),
                Some(tme_rules::TransitionKindViewV1::Door)
            )
        })
        .expect("undercroft_loop must have a door tile");

    // Find which direction leads from the player to this door
    let door_pos = door_tile.position;
    let dx = (door_pos.x - player_location.position.x).signum();
    let dy = (door_pos.y - player_location.position.y).signum();
    let direction = tme_rules::Direction::all()
        .iter()
        .find(|d| d.delta() == (dx, dy))
        .copied()
        .expect("door must be adjacent or reachable");

    // Open the door
    let _events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::Open(direction),
        )
        .expect("open must succeed");

    let after = engine.snapshot();
    let same_tile_after = after
        .realms
        .iter()
        .find(|realm| realm.id == player_location.realm)
        .into_iter()
        .flat_map(|realm| realm.levels.iter())
        .find(|level| level.id == player_location.level)
        .into_iter()
        .flat_map(|level| level.tiles.iter())
        .find(|t| t.position == door_pos && t.terrain_id == door_tile.terrain_id)
        .expect("same door tile must exist");

    let door_after = same_tile_after
        .transition
        .as_ref()
        .expect("must still have transition");
    assert_eq!(
        door_after.door_state,
        Some(tme_rules::DoorStateViewV1::Open),
        "door must be open after open command"
    );
}

#[test]
fn hidden_transition_is_absent_from_snapshots_until_revealed() {
    let mut engine = hidden_open_door_engine();
    let door_position = Coord { x: 4, y: 1 };

    let initial = engine.snapshot();
    assert_eq!(
        tile_transition_in_snapshot(&initial, "workroom", door_position),
        None,
        "hidden transitions must not appear in omniscient local snapshots before reveal"
    );
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should build");
    assert_eq!(
        tile_transition_in_observed_snapshot(&observed, "workroom", door_position),
        None,
        "hidden transitions must not appear in observed snapshots before reveal"
    );

    engine
        .set_navigation_revealed(
            &WorldPosition::new("realm_0", "workroom", door_position),
            true,
        )
        .expect("reveal should succeed");
    assert_eq!(
        tile_transition_in_snapshot(&engine.snapshot(), "workroom", door_position)
            .map(|transition| transition.kind),
        Some(tme_rules::TransitionKindViewV1::Door),
        "revealed hidden transitions should appear in snapshots"
    );

    engine
        .set_navigation_revealed(
            &WorldPosition::new("realm_0", "workroom", door_position),
            false,
        )
        .expect("hide should succeed");
    assert_eq!(
        tile_transition_in_snapshot(&engine.snapshot(), "workroom", door_position),
        None,
        "hiding a revealed transition should remove it from snapshots again"
    );
}

#[test]
fn portal_transition_appears_in_snapshot_and_observed_snapshot_while_active() {
    let mut engine = portal_snapshot_engine();
    let anchor = WorldPosition::new("realm_0", "workroom", Coord { x: 2, y: 1 });
    engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blue_gate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: anchor.clone(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("portal spell should cast");

    let snapshot = engine.snapshot();
    let transition = tile_transition_in_snapshot(&snapshot, "workroom", anchor.position)
        .expect("active portal should appear in snapshot");
    assert_eq!(
        serde_json::to_value(transition.kind).expect("transition kind serializes"),
        "portal"
    );

    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should build");
    let observed_transition =
        tile_transition_in_observed_snapshot(&observed, "workroom", anchor.position)
            .expect("active portal should appear in observed snapshot");
    assert_eq!(
        serde_json::to_value(observed_transition.kind).expect("transition kind serializes"),
        "portal"
    );
}

#[test]
fn snapshot_includes_one_exact_carried_layout_without_duplicate_weapon_field() {
    // Use kobold_warren because the player starts with a weapon (yew_shortbow)
    let engine = ContentParts::tracked("kobold_warren", "profile/kobold_warren")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    let player_view = snapshot
        .actors
        .iter()
        .find(|a| a.id == snapshot.controlled_actor_ids[0])
        .expect("player must exist in snapshot");
    // Player must start with a weapon in kobold_warren
    assert!(
        player_view
            .carried
            .items
            .iter()
            .any(|item| item.position == tme_rules::CarriedPosition::RightHand),
        "player should start with a right-hand weapon in kobold_warren"
    );
    // Verify carried items are sorted by exact position.
    for i in 1..player_view.carried.items.len() {
        assert!(
            player_view.carried.items[i - 1].position <= player_view.carried.items[i].position,
            "carried items must be sorted by exact position"
        );
    }
}

#[test]
fn snapshot_25_and_observed_25_agree_on_exact_positioned_player_gold() {
    let engine = ContentParts::tracked("gold_training", "profile/gold_training")
        .engine(7)
        .expect("gold fixture should start");
    let debug = serde_json::to_value(engine.snapshot()).expect("debug snapshot");
    let observed = serde_json::to_value(
        engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot"),
    )
    .expect("observed snapshot JSON");
    let debug_player = debug["actors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|actor| actor["id"] == "player")
        .unwrap();
    let observed_player = observed["actors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|actor| actor["id"] == "player")
        .unwrap();
    let expected = serde_json::json!({"left_hand": 0, "right_hand": 0, "sack": 500});
    assert_eq!(debug_player["carried"]["gold"], expected);
    assert_eq!(observed_player["carried"]["gold"], expected);
    assert!(debug_player["carried"].get("sack_gold").is_none());
    assert!(observed_player["carried"].get("sack_gold").is_none());
}

#[test]
fn snapshot_item_surfaces_share_the_explicit_instance_projection() {
    let mut engine =
        ContentParts::tracked("item_instance_contract", "profile/item_instance_contract")
            .engine(7)
            .expect("engine should start");

    let initial = serde_json::to_value(engine.snapshot()).expect("snapshot should serialize");
    let ground_tonic = initial["ground_items"]
        .as_array()
        .expect("ground items should be an array")
        .iter()
        .find(|item| item["item_instance_id"] == "tonic_a")
        .expect("tonic_a should be projected on the ground");
    assert_eq!(ground_tonic["item_definition_id"], "restorative_tonic");
    assert_eq!(ground_tonic["quantity"], 2);
    assert_eq!(ground_tonic["identified"], false);
    assert_eq!(ground_tonic["appraised"], true);
    assert_eq!(ground_tonic["known_unit_value_gold"], 12);
    assert_eq!(ground_tonic["known_stack_value_gold"], 24);
    assert_eq!(ground_tonic["unit_burden"], 3);
    assert_eq!(ground_tonic["stack_burden"], 6);
    assert!(
        ground_tonic.get("item_id").is_none(),
        "shared item projections must not expose ambiguous item_id"
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("taking tonic_a should succeed");
    let after = serde_json::to_value(engine.snapshot()).expect("snapshot should serialize");
    let player = after["actors"]
        .as_array()
        .expect("actors should be an array")
        .iter()
        .find(|actor| actor["id"] == "player")
        .expect("player should be projected");
    let carried = &player["carried"]["items"][0];
    assert_eq!(carried["item_instance_id"], "tonic_a");
    assert_eq!(carried["item_definition_id"], "restorative_tonic");
    assert_eq!(carried["quantity"], 2);
    assert_eq!(carried["known_stack_value_gold"], 24);
    assert_eq!(carried["stack_burden"], 6);
    assert_eq!(
        player["burden"],
        serde_json::json!({
            "item_burden": 6,
            "coin_burden": 5,
            "total_burden": 11,
            "lightly_loaded_limit": 100000,
            "moderately_loaded_limit": 200000,
            "heavily_loaded_limit": 300000,
            "tier": "lightly_loaded"
        })
    );
}

#[test]
fn item_capability_view_rejects_raw_economy_value() {
    let raw_value = serde_json::json!({"value_gold": 12});
    assert!(
        serde_json::from_value::<tme_rules::view::ItemCapabilityViewV1>(raw_value).is_err(),
        "capability projections must not deserialize raw economy value"
    );
}

#[test]
fn snapshot_projects_dx_spell_book_and_mp_recovery_capabilities_exactly() {
    let mut engine = ContentParts::tracked(
        "spell_learning_purchase_casting_xp",
        "profile/spell_learning_purchase_casting_xp",
    )
    .engine(7)
    .expect("engine should start");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("bound Spell Book should move to its owner's right hand");

    let snapshot = engine.snapshot();
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player");
    let book = player
        .carried
        .items
        .iter()
        .find(|item| item.item.item_instance_id == "spell_book")
        .expect("Spell Book");
    assert_eq!(book.position, tme_rules::CarriedPosition::RightHand);
    assert_eq!(book.item.binding, tme_rules::view::ItemBindingViewV1::Bound);
    assert_eq!(
        book.capability
            .as_ref()
            .expect("Spell Book capability")
            .spell_book_for
            .as_deref(),
        Some(["wizard_magic".to_string()].as_slice())
    );

    let capability = tme_rules::model::ItemCapability {
        mp_recovery_multiplier: Some(tme_rules::model::MpRecoveryMultiplier {
            numerator: 3,
            denominator: 2,
            evidence_state: tme_rules::MagicRuleEvidenceState::OriginalProvisional,
        }),
        ..tme_rules::model::ItemCapability::default()
    };
    let projected = tme_rules::view::ItemCapabilityViewV1::from(&capability);
    assert_eq!(
        serde_json::to_value(projected).expect("capability serializes"),
        serde_json::json!({
            "mp_recovery_multiplier": {
                "numerator": 3,
                "denominator": 2,
                "evidence_state": "original_provisional"
            }
        })
    );
}

#[test]
fn snapshot_projects_delayed_spell_damage_credit_exactly() {
    let mut engine = ContentParts::tracked(
        "control_poison_protection",
        "profile/control_poison_protection",
    )
    .engine(7)
    .expect("engine should start");
    let target = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target");
    engine.world_mut().actors[target]
        .magic_resistance
        .natural_save_twentieths = 0;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "venom".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("venom should apply");

    let snapshot = engine.snapshot();
    let target = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target");
    let credit = target
        .active_effects
        .iter()
        .find(|effect| effect.effect_id == "venom")
        .and_then(|effect| effect.spell_damage_credit.as_ref())
        .expect("delayed spell credit");
    assert_eq!(credit.caster_actor_id, "player");
    assert_eq!(credit.spell_id, "venom");
    assert_eq!(
        credit.reward_class,
        tme_rules::SpellDamageRewardClass::Directed
    );
}
