use crate::support::content_parts::ContentParts;
use tme_rules::{ActorKind, Direction, Engine, Event, PlayerIntent};

fn fixture_engine(case_id: &str, rng_seed: u64) -> Engine {
    ContentParts::tracked(case_id, &format!("profile/{case_id}"))
        .engine(rng_seed)
        .unwrap_or_else(|error| panic!("{case_id} engine should start: {error}"))
}

#[test]
fn character_hp_matches_actor_hp_after_damage() {
    // characte_sheet fixture has a character with resources.hp = max_hp = 12
    let mut engine = fixture_engine("character_sheet", 7);

    // Move east twice to reach and engage the mireling at (3,1)
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    // Attack — this should not change player HP, but verifies sync after combat
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");

    let player = &engine.world().actors[0];
    assert_eq!(player.kind, ActorKind::Player);
    let cs = player
        .character
        .as_ref()
        .expect("player should have character sheet");
    assert_eq!(
        player.hp, cs.resources.hp,
        "actor.hp and character.resources.hp must match after combat"
    );
    assert!(
        player.hp <= cs.resources.max_hp,
        "hp should not exceed max_hp"
    );
}

#[test]
fn max_hp_uses_character_resources_when_present() {
    let engine = fixture_engine("character_sheet", 7);

    let player = &engine.world().actors[0];
    let cs = player
        .character
        .as_ref()
        .expect("player should have character sheet");
    assert_eq!(
        player.max_hp(),
        cs.resources.max_hp,
        "max_hp() should return character.resources.max_hp when character sheet present"
    );
    // max_hp is authoritative from character resources, not stats.hp
    assert!(
        player.hp <= player.max_hp(),
        "current hp must not exceed max_hp"
    );
}

#[test]
fn non_character_fixture_unchanged() {
    // first_room fixture has no character sheet
    let mut engine = fixture_engine("first_room", 7);

    // Move and attack to trigger damage
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");

    let player = &engine.world().actors[0];
    assert!(
        player.character.is_none(),
        "non-character fixture should have no character sheet"
    );
    assert!(
        player.hp > 0,
        "player should still be alive after one attack"
    );
    // Non-character HP comes from stats.hp; max_hp() should fall back to stats.hp
    assert_eq!(
        player.max_hp(),
        player.stats.hp,
        "non-character max_hp() should fall back to stats.hp"
    );
}

#[test]
fn character_hp_in_snapshot_matches_actor_hp() {
    let mut engine = fixture_engine("character_sheet", 7);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");

    let snapshot = engine.snapshot();
    let player = &engine.world().actors[0];
    let cs = player.character.as_ref().unwrap();

    // Find the player in the snapshot by actor id
    let player_view = snapshot
        .actors
        .iter()
        .find(|a| a.id == player.id)
        .expect("player should be in snapshot");

    let csv = player_view
        .character
        .as_ref()
        .expect("snapshot should have character");
    assert_eq!(
        csv.resources.hp, cs.resources.hp,
        "snapshot character.hp must match engine character.hp"
    );
    assert_eq!(
        csv.resources.hp, player.hp,
        "snapshot character.hp must match actor.hp"
    );
    assert_eq!(
        csv.resources.max_hp, cs.resources.max_hp,
        "snapshot character.max_hp must match engine character.max_hp"
    );
}

#[test]
fn character_hp_syncs_through_full_script_with_combat() {
    // xp_progression fixture has a character sheet and involves combat
    let mut engine = fixture_engine("xp_progression", 7);

    // Verify initial sync
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(player.hp, cs.resources.hp, "initial hp must be synced");
        assert_eq!(
            cs.resources.hp, cs.resources.max_hp,
            "initial hp should be at max"
        );
    }

    // Step through script: move_path east,east
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");

    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(
            player.hp, cs.resources.hp,
            "hp must be synced after movement"
        );
    }

    // Step: attack mireling
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");

    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(player.hp, cs.resources.hp, "hp must be synced after attack");
        assert!(
            player.hp <= cs.resources.max_hp,
            "hp must not exceed max_hp"
        );
    }
}

#[test]
fn character_hp_initialized_correctly_on_start() {
    // Verify that a character-backed fixture starts with hp == max_hp and properly synced
    let engine = fixture_engine("character_sheet", 7);

    let player = &engine.world().actors[0];
    let cs = player
        .character
        .as_ref()
        .expect("player should have character sheet");

    assert_eq!(
        player.hp, cs.resources.hp,
        "initial actor.hp must match character.resources.hp"
    );
    assert_eq!(
        cs.resources.hp, cs.resources.max_hp,
        "initial character resources should start at max_hp"
    );
    assert_eq!(
        cs.resources.hp, player.stats.hp,
        "initial character resources.hp must match stats.hp"
    );
}

#[test]
fn character_hp_matches_actor_hp_after_player_defeat() {
    let mut engine = fixture_engine("death_corpse", 7);

    // Verify initial sync
    {
        let player = &engine.world().actors[0];
        let cs = player
            .character
            .as_ref()
            .expect("player should have character sheet");
        assert_eq!(player.hp, cs.resources.hp, "initial hp must be synced");
        assert_eq!(
            cs.resources.hp, cs.resources.max_hp,
            "initial hp must be at max"
        );
        assert_eq!(
            cs.resources.hp, player.stats.hp,
            "character hp must match stats.hp"
        );
    }

    let intents = [
        PlayerIntent::PhysicalAttack {
            authorization: tme_rules::HostilityAuthorization::Safe,
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "scavenger".into(),
        },
        PlayerIntent::PhysicalAttack {
            authorization: tme_rules::HostilityAuthorization::Safe,
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "lookout".into(),
        },
        PlayerIntent::SearchCorpse(tme_rules::CorpseId::parse("corpse:2").unwrap()),
        PlayerIntent::SearchCorpse(tme_rules::CorpseId::parse("corpse:1").unwrap()),
        PlayerIntent::Wait,
    ];
    for intent in intents {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
            .expect("gallery step should succeed");
        let player = &engine.world().actors[0];
        let cs = player
            .character
            .as_ref()
            .expect("player should have character sheet");
        assert_eq!(
            player.hp, cs.resources.hp,
            "actor and character HP must remain synchronized"
        );
    }
    let player = &engine.world().actors[0];
    assert!(!player.is_alive());
    assert_eq!(player.hp, 0);
    assert_eq!(player.character.as_ref().unwrap().resources.hp, 0);
}

#[test]
fn character_hp_matches_actor_hp_after_resource_recovery() {
    // Verify HP sync invariant through combat + cadence recovery steps.
    // Exact damage values depend on combat formula (hit/miss/block).
    // The invariant is actor.hp == character.resources.hp after every step.
    let mut engine = fixture_engine("resting_hollow", 7);

    for step in 0..12 {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .expect("wait should succeed");
        let player = &engine.world().actors[0];
        if let Some(cs) = &player.character {
            assert_eq!(player.hp, cs.resources.hp, "hp sync after step {}", step);
            assert!(
                player.hp <= cs.resources.max_hp && player.hp >= 0,
                "hp in valid range"
            );
        }
    }
}

#[test]
fn character_hp_matches_actor_hp_after_balm_healing() {
    // Verify HP sync invariant through combat + balm healing steps.
    // Exact damage/healing values depend on combat formula.
    // The invariant is actor.hp == character.resources.hp after every step.
    let mut engine = fixture_engine("balm_cache", 7);

    for step in 0..12 {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .expect("wait should succeed");
        let player = &engine.world().actors[0];
        if let Some(cs) = &player.character {
            assert_eq!(player.hp, cs.resources.hp, "hp sync after step {}", step);
            assert!(
                player.hp <= cs.resources.max_hp && player.hp >= 0,
                "hp in valid range"
            );
        }
    }
}

#[test]
fn character_hp_stays_in_sync_after_each_mutation() {
    // Step through character_sheet fixture and assert hp sync after each step
    let mut engine = fixture_engine("character_sheet", 7);

    // After engine start
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(player.hp, cs.resources.hp, "hp sync at start");
    }

    // After movement (no damage expected)
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(player.hp, cs.resources.hp, "hp sync after move");
    }

    // After attack
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(player.hp, cs.resources.hp, "hp sync after attack");
        assert!(
            player.hp <= cs.resources.max_hp,
            "post-combat hp must not exceed max_hp"
        );
    }
}

#[test]
fn character_stamina_syncs_after_movement() {
    let mut engine = fixture_engine("character_sheet", 7);

    // Initial stamina
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(player.stamina, cs.resources.stamina, "initial stamina sync");
        assert_eq!(player.stamina, 10);
        assert_eq!(cs.resources.stamina, 10);
    }

    // A lightly loaded Walk has no stamina charge.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move should succeed");

    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(
            player.stamina, cs.resources.stamina,
            "stamina must sync after movement"
        );
    }
}

#[test]
fn lightly_loaded_walk_preserves_stamina_and_mirror() {
    let mut engine = fixture_engine("character_sheet", 7);

    let player = &engine.world().actors[0];
    assert_eq!(player.stamina, 10);

    // Move east as a lightly loaded Walk with no stamina charge.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move should succeed");

    let player = &engine.world().actors[0];
    let cs = player.character.as_ref().unwrap();
    assert_eq!(
        player.stamina, 10,
        "lightly loaded Walk should preserve stamina"
    );
    assert_eq!(
        player.stamina, cs.resources.stamina,
        "stamina must sync after movement"
    );
}

#[test]
fn ordinary_attack_preserves_stamina_and_mirror() {
    let mut engine = fixture_engine("character_sheet", 7);

    // Move to engage mireling
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");

    // Before attack, check stamina
    {
        let player = &engine.world().actors[0];
        assert!(player.stamina > 0, "should have stamina before attack");
    }

    let stamina_before = engine.world().actors[0].stamina;
    // Ordinary physical attacks do not spend stamina.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");

    let player = &engine.world().actors[0];
    let cs = player.character.as_ref().unwrap();
    assert_eq!(
        player.stamina, cs.resources.stamina,
        "stamina must sync after attack"
    );
    assert_eq!(player.stamina, stamina_before);
}

#[test]
fn wait_has_no_direct_stamina_recovery_or_activity_write() {
    let mut engine = fixture_engine("character_sheet", 7);

    // Establish an active movement timestamp before checking that Wait leaves it alone.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");

    {
        let player = &mut engine.world_mut().actors[0];
        player.stamina = 8;
        player
            .character
            .as_mut()
            .expect("character")
            .resources
            .stamina = 8;
    }

    let last_active_before = engine.world().actors[0].resource_activity.last_active_at;
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should succeed");

    assert_eq!(
        engine.world().actors[0].resource_activity.last_active_at,
        last_active_before,
        "wait must leave actor-local activity unchanged"
    );
    assert!(
        !events
            .events
            .iter()
            .any(|event| matches!(event, Event::MovementStaminaSpent { .. }))
    );
}

#[test]
fn stamina_syncs_after_each_mutation() {
    let mut engine = fixture_engine("character_sheet", 7);

    // After engine start
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(
            player.stamina, cs.resources.stamina,
            "stamina sync at start"
        );
    }

    // After movement
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(
            player.stamina, cs.resources.stamina,
            "stamina sync after move"
        );
    }

    // After attack
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");
    {
        let player = &engine.world().actors[0];
        let cs = player.character.as_ref().unwrap();
        assert_eq!(
            player.stamina, cs.resources.stamina,
            "stamina must sync after attack"
        );
    }
}

#[test]
fn non_character_actor_unlimited_stamina() {
    // first_room fixture has no character sheet — stamina should be unchanged
    let mut engine = fixture_engine("first_room", 7);

    let player = &engine.world().actors[0];
    assert!(player.character.is_none());
    assert_eq!(player.stamina, 10, "non-character stamina defaults to 10");

    // Move and attack — non-character actor should not lose stamina
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should succeed");

    let player = &engine.world().actors[0];
    assert_eq!(
        player.stamina, 10,
        "non-character stamina should remain at default after actions"
    );
}

#[test]
fn stamina_capped_at_max_stamina() {
    let mut parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(1);
    let mut engine = parts.engine(7).expect("character engine should start");

    let player = &engine.world().actors[0];
    assert_eq!(player.max_stamina(), 10);

    // Establish activity, then arrange an inactive cadence boundary one below max.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");

    {
        let player = &mut engine.world_mut().actors[0];
        player.stamina = 9;
        player
            .character
            .as_mut()
            .expect("character")
            .resources
            .stamina = 9;
    }
    // Wait — stamina should recover toward max but not exceed it
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should succeed");
    let player = &engine.world().actors[0];
    assert_eq!(player.stamina, 10);
    assert!(
        player.stamina <= player.max_stamina(),
        "stamina must not exceed max_stamina"
    );
}

#[test]
fn level_growth_is_one_atomic_mirrored_resource_mutation() {
    let mut engine = fixture_engine("xp_progression", 1_010_580_540);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("growth-producing attack should succeed");

    let actor = &engine.world().actors[0];
    let resources = &actor.character.as_ref().expect("character").resources;
    assert_eq!(
        (resources.hp, resources.max_hp, resources.peak_hp),
        (47, 47, 47)
    );
    assert_eq!((resources.mp, resources.max_mp), (0, 0));
    assert_eq!((resources.stamina, resources.max_stamina), (28, 28));
    assert_eq!(actor.hp, resources.hp);
    assert_eq!(actor.mp, resources.mp);
    assert_eq!(actor.stamina, resources.stamina);
    assert!(resources.hp <= resources.max_hp && resources.max_hp <= resources.peak_hp);
}

#[test]
fn level_growth_overflow_rolls_back_without_partial_resource_mutation() {
    let mut engine = fixture_engine("xp_progression", 7);
    engine.world_mut().actors[1].attack_ready_at = tme_rules::LogicalTime::new(99);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should succeed");
    {
        let actor = &mut engine.world_mut().actors[0];
        actor.hp = i32::MAX;
        let resources = &mut actor.character.as_mut().expect("character").resources;
        resources.hp = i32::MAX;
        resources.max_hp = i32::MAX;
        resources.peak_hp = i32::MAX;
    }

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("checked HP overflow should reject the whole step");
    assert!(error.to_string().contains("level HP growth overflow"));

    let actor = &engine.world().actors[0];
    let character = actor.character.as_ref().expect("character");
    assert_eq!(character.progression.experience, 0);
    assert_eq!(character.progression.level, 1);
    assert_eq!(actor.hp, i32::MAX);
    assert_eq!(character.resources.hp, i32::MAX);
    assert_eq!(character.resources.max_hp, i32::MAX);
    assert_eq!(character.resources.peak_hp, i32::MAX);
    assert_eq!(engine.world().actors[1].hp, 3, "defeat must also roll back");
}

#[test]
fn ordinary_recovery_clamps_to_current_max_without_raising_peak() {
    let mut parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(1);
    parts.rules_source_mut()["resources"]["inactive_hp_recovery"] = serde_json::json!(i32::MAX);
    let mut engine = parts.engine(7).expect("character engine should start");
    {
        let actor = &mut engine.world_mut().actors[0];
        actor.hp -= 1;
        actor.character.as_mut().expect("character").resources.hp = actor.hp;
    }
    let (max_before, peak_before) = {
        let resources = &engine.world().actors[0]
            .character
            .as_ref()
            .expect("character")
            .resources;
        (resources.max_hp, resources.peak_hp)
    };

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("recovery should succeed");
    let actor = &engine.world().actors[0];
    let resources = &actor.character.as_ref().expect("character").resources;
    assert_eq!(resources.hp, max_before);
    assert_eq!(resources.max_hp, max_before);
    assert_eq!(resources.peak_hp, peak_before);
    assert_eq!(actor.hp, resources.hp);
}
