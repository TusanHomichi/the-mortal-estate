use super::*;

#[test]
fn snapshot_includes_active_effects_and_resistance_boosts() {
    let engine = ContentParts::tracked("status_effects", "profile/status_effects")
        .engine(7)
        .expect("engine should start");
    let snapshot = engine.snapshot();
    assert_eq!(
        snapshot.contract_version,
        tme_rules::SNAPSHOT_CONTRACT_VERSION
    );
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player in snapshot");
    assert_eq!(player.active_effects.len(), 1);
    assert_eq!(player.active_effects[0].instance_id, "rooted_1");
    assert_eq!(player.magic_resistance.boosts.len(), 1);
    assert_eq!(player.magic_resistance.boosts[0].tag, "stun");
}

#[test]
fn snapshot_includes_tile_effects_and_effective_tile_fields() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");

    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:web:1".to_string(),
        effect_id: "web_field".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "web_field".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 2 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["web".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: Some("hindered".to_string()),
        sight: Some("obscured".to_string()),
        hazard: None,
        move_cost: Some(2),
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });

    let snapshot = engine.snapshot();

    assert!(
        snapshot
            .tile_effects
            .iter()
            .any(|effect| effect.instance_id == "tile:web:1")
    );
    let tile = snapshot
        .realms
        .iter()
        .flat_map(|realm| realm.levels.iter())
        .flat_map(|level| level.tiles.iter())
        .find(|tile| tile.position == Coord { x: 1, y: 2 })
        .expect("tile exists");
    assert!(tile.passable);
    assert_eq!(tile.move_cost, Some(2));
}

#[test]
fn later_passable_tile_effect_restores_base_move_cost_after_hindered() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    let position = Coord { x: 1, y: 2 };

    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:web:1".to_string(),
        effect_id: "web_field".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "web_field".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", position),
        kind: "terrain_overlay".to_string(),
        tags: vec!["web".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: Some("hindered".to_string()),
        sight: None,
        hazard: None,
        move_cost: Some(2),
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:clear:1".to_string(),
        effect_id: "clear_path".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "clear_path".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", position),
        kind: "terrain_overlay".to_string(),
        tags: vec!["clear".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: Some("passable".to_string()),
        sight: None,
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });

    let snapshot = engine.snapshot();
    let tile = snapshot
        .realms
        .iter()
        .flat_map(|realm| realm.levels.iter())
        .flat_map(|level| level.tiles.iter())
        .find(|tile| tile.position == position)
        .expect("tile exists");

    assert!(tile.passable);
    assert_eq!(tile.move_cost, Some(1));
}

#[test]
fn summon_snapshot_includes_true_social_identity_and_lifecycle_metadata() {
    let mut engine = bw_summon_snapshot_engine("lawful");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    let actor = engine
        .snapshot()
        .actors
        .into_iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor visible");

    assert_eq!(actor.social.alignment, CharacterAlignment::Lawful);
    assert_eq!(actor.social.nature, SocialNatureViewV1::Other);
    assert_eq!(
        actor.social.behavior,
        SocialBehaviorViewV1::AlignmentCreature
    );
    assert_eq!(
        actor.social.owner_relation,
        SocialOwnerRelationViewV1::Summoner
    );
    assert_eq!(actor.owner_id.as_deref(), Some("player"));
    let summoned = actor.summoned.expect("summoned metadata");
    assert_eq!(summoned.instance_id, "summon:call_echo:1:echo_guardian");
    assert_eq!(summoned.source_spell_id, "call_echo");
    assert_eq!(summoned.template_id, "echo_guardian");
    assert_eq!(summoned.remaining_rounds, Some(1));
}

#[test]
fn summon_observed_snapshot_includes_perceived_social_identity_and_lifecycle_metadata() {
    let mut engine = bw_summon_snapshot_engine("lawful");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    let actor = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot")
        .actors
        .into_iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor visible");

    assert_eq!(actor.social.attack_safety, AttackSafety::Invalid);
    assert_eq!(
        actor.social.apparent_behavior,
        SocialBehaviorViewV1::AlignmentCreature
    );
    assert_eq!(actor.owner_id.as_deref(), Some("player"));
    let summoned = actor.summoned.expect("summoned metadata");
    assert_eq!(summoned.instance_id, "summon:call_echo:1:echo_guardian");
    assert_eq!(summoned.source_spell_id, "call_echo");
    assert_eq!(summoned.template_id, "echo_guardian");
    assert_eq!(summoned.remaining_rounds, Some(1));
}

#[test]
fn debug_25_and_observed_25_expose_safe_corpse_and_claim_summaries() {
    assert_eq!(SNAPSHOT_CONTRACT_VERSION, 31);
    let mut engine = ContentParts::tracked("death_corpse", "profile/death_corpse")
        .engine(7)
        .expect("death gallery starts");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "scavenger".into(),
            },
        )
        .expect("monster defeat");

    let debug = engine.snapshot();
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot");
    assert_eq!(debug.contract_version, 31);
    assert_eq!(observed.contract_version, 30);
    assert_eq!(debug.corpses.len(), 1);
    assert_eq!(observed.corpses, debug.corpses);
    assert_eq!(debug.corpses[0].corpse_id.as_str(), "corpse:1");
    assert!(!debug.corpses[0].searched);
    assert_eq!(
        debug.corpses[0]
            .loot_claim
            .as_ref()
            .expect("corpse claim")
            .basis,
        tme_rules::LootClaimBasis::KillingBlow
    );
    let debug_json = serde_json::to_value(&debug).unwrap();
    assert!(debug_json["corpses"][0].get("contents").is_none());
    assert!(debug_json["corpses"][0].get("sack_gold").is_none());
    let dropped = debug
        .ground_items
        .iter()
        .find(|item| item.item.item_instance_id == "rusted_knife")
        .expect("hand item projection");
    assert!(dropped.loot_claim.is_some());

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(CorpseId::parse("corpse:1").unwrap()),
        )
        .expect("corpse search");
    let after = engine.snapshot();
    assert!(after.corpses[0].searched);
    assert_eq!(after.ground_gold.len(), 1);
    assert_eq!(after.ground_gold[0].gold_pile_id.as_str(), "gold:1");
    assert_eq!(after.ground_gold[0].amount, 3);
    assert_eq!(
        after.ground_gold[0]
            .loot_claim
            .as_ref()
            .expect("ground gold claim")
            .basis,
        tme_rules::LootClaimBasis::KillingBlow
    );
}

#[test]
fn debug_21_automatic_actor_rows_reject_unknown_fields() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("first room starts");
    let row = serde_json::to_value(&engine.snapshot().automatic_actors[0])
        .expect("automatic actor row serializes");
    serde_json::from_value::<tme_rules::AutomaticActorViewV1>(row.clone())
        .expect("current row parses");

    let mut extra = row;
    extra["legacy"] = serde_json::json!(true);
    assert!(serde_json::from_value::<tme_rules::AutomaticActorViewV1>(extra).is_err());
}
