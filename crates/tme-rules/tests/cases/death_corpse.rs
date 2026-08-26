use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, ActorKind, ActorLifeState, CarriedPosition, CorpseDisposition, CorpseId,
    DeathCause, Engine, Event, GoldRelocationReason, ItemLocation, ItemRelocationReason,
    LogicalTime, LootClaimBasis, LootOwnerId, PlayerIntent, ResurrectionMethod,
    ResurrectionRequest, SkillEntry, WorldPosition,
};

fn fixture_value() -> ContentParts {
    ContentParts::tracked("death_corpse", "profile/death_corpse")
}

fn engine() -> Engine {
    fixture_value()
        .engine(7)
        .expect("focused engine should start")
}

fn corpse_id(sequence: u64) -> CorpseId {
    CorpseId::parse(format!("corpse:{sequence}")).unwrap()
}

fn attack(engine: &mut Engine, target: &str) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: target.into(),
            },
        )
        .unwrap_or_else(|error| panic!("attack {target:?} should succeed: {error}"))
        .events
}

fn create_two_corpses(engine: &mut Engine) {
    attack(engine, "scavenger");
    attack(engine, "lookout");
}

fn search_two_corpses(engine: &mut Engine) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(corpse_id(2)),
        )
        .expect("newest corpse search should succeed");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(corpse_id(1)),
        )
        .expect("older corpse search should succeed");
}

fn defeat_player(engine: &mut Engine) -> Vec<Event> {
    create_two_corpses(engine);
    search_two_corpses(engine);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("brute should defeat the waiting player")
        .events
}

fn event_position(events: &[Event], predicate: impl Fn(&Event) -> bool) -> usize {
    events
        .iter()
        .position(predicate)
        .unwrap_or_else(|| panic!("expected event in {events:#?}"))
}

#[test]
fn four_contract_death_corpse_fixture_validates() {
    fixture_value()
        .validated_seed()
        .expect("death/corpse graph should validate");
}

#[test]
fn obsolete_flat_seed_schema_field_is_rejected_without_a_compatibility_parser() {
    let mut value = fixture_value();
    value.world_seed["schema_version"] = serde_json::json!(0);
    let error = value
        .validated_seed()
        .expect_err("obsolete flat seed schema field must be rejected");
    assert!(error.contains("unknown field `schema_version`"));
}

#[test]
fn every_actor_requires_an_exact_death_policy() {
    let mut missing = fixture_value();
    missing
        .actor_definition_mut(1)
        .as_object_mut()
        .expect("actor object")
        .remove("death");
    assert!(missing.validated_seed().is_err());

    let mut unknown = fixture_value();
    unknown.actor_definition_mut(1)["death"]["remains"] = serde_json::json!("mist");
    assert!(unknown.validated_seed().is_err());

    let mut extra = fixture_value();
    extra.actor_definition_mut(1)["death"]["delay"] = serde_json::json!(1);
    assert!(extra.validated_seed().is_err());
}

#[test]
fn player_must_author_searchable_remains() {
    let mut value = fixture_value();
    value.actor_definition_mut(0)["death"]["remains"] = serde_json::json!("none");
    let error = value
        .validated_seed()
        .expect_err("player no-remains policy must fail validation");
    assert!(error.contains("searchable_corpse"));
}

#[test]
fn physical_monster_defeat_uses_one_corpse_item_gold_and_claim_authority() {
    let mut engine = engine();
    let events = attack(&mut engine, "scavenger");
    let id = corpse_id(1);

    assert!(matches!(
        events.iter().find(|event| matches!(event, Event::ActorDefeated { .. })),
        Some(Event::ActorDefeated {
            actor_id,
            cause: DeathCause::Physical,
            credited_actor_id: Some(credited),
            loot_claim: Some(claim),
            ..
        }) if actor_id == "scavenger"
            && credited == "player"
            && claim.basis == LootClaimBasis::KillingBlow
            && matches!(&claim.owner, LootOwnerId::Character(character_id)
                if character_id.as_str() == "character:death_corpse:primary")
    ));

    let corpse = &engine.world().corpses[&id];
    assert_eq!(corpse.origin_actor_id, "scavenger");
    assert_eq!(corpse.origin_kind, ActorKind::Monster);
    assert_eq!(corpse.contents.len(), 1);
    assert_eq!(corpse.contents[&CarriedPosition::Belt1], "cloth_bundle");
    assert_eq!(corpse.gold, 3);
    assert!(!corpse.searched);
    assert_eq!(engine.world().next_corpse_sequence, 2);
    assert_eq!(engine.world().next_gold_sequence, 1);

    assert_eq!(
        engine.item_location("rusted_knife").unwrap(),
        ItemLocation::Ground {
            position: WorldPosition::new("realm_0", "room_0", (1, 1).into())
        }
    );
    assert_eq!(
        engine.item_location("cloth_bundle").unwrap(),
        ItemLocation::Corpse {
            corpse_id: id.clone(),
            position: CarriedPosition::Belt1,
        }
    );
    let dropped = engine
        .world()
        .ground_items
        .iter()
        .find(|item| item.item_instance_id == "rusted_knife")
        .expect("hand item should be on the ground");
    assert_eq!(dropped.loot_claim, corpse.loot_claim);
    assert_eq!(
        engine.world().item_instances["rusted_knife"].binding,
        tme_rules::ItemBindingState::Unrestricted
    );

    let defeated = event_position(
        &events,
        |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "scavenger"),
    );
    let created = event_position(
        &events,
        |event| matches!(event, Event::CorpseCreated { corpse_id, .. } if corpse_id == &id),
    );
    let hand_drop = event_position(&events, |event| {
        matches!(event, Event::ItemRelocated {
            item_instance_id,
            reason: ItemRelocationReason::DeathDrop,
            ..
        } if item_instance_id == "rusted_knife")
    });
    let retention = event_position(&events, |event| {
        matches!(event, Event::ItemRelocated {
            item_instance_id,
            reason: ItemRelocationReason::CorpseRetention,
            ..
        } if item_instance_id == "cloth_bundle")
    });
    let gold = event_position(&events, |event| {
        matches!(
            event,
            Event::GoldRelocated {
                amount: 3,
                reason: GoldRelocationReason::CorpseRetention,
                ..
            }
        )
    });
    let life = event_position(
        &events,
        |event| matches!(event, Event::ActorLifeStateChanged { actor_id, .. } if actor_id == "scavenger"),
    );
    assert!(defeated < created && created < hand_drop && hand_drop < retention);
    assert!(retention < gold && gold < life);
}

#[test]
fn stacked_corpses_project_newest_first_and_search_independently_and_atomically() {
    let mut engine = engine();
    create_two_corpses(&mut engine);
    let first_corpse = corpse_id(1);
    let second_corpse = corpse_id(2);

    let before_reads = engine.world().clone();
    let snapshot = engine.snapshot();
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot");
    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("action options");
    assert_eq!(engine.world(), &before_reads, "read surfaces must be pure");

    assert_eq!(
        context
            .corpses_here
            .iter()
            .map(|corpse| (corpse.corpse_id.as_str(), corpse.pile_index))
            .collect::<Vec<_>>(),
        vec![("corpse:2", 1), ("corpse:1", 2)]
    );
    assert_eq!(snapshot.corpses.len(), 2);
    assert_eq!(observed.corpses.len(), 2);
    let snapshot_json = serde_json::to_value(&snapshot).unwrap();
    for corpse in snapshot_json["corpses"].as_array().unwrap() {
        assert!(corpse.get("contents").is_none());
        assert!(corpse.get("sack_gold").is_none());
    }
    assert!(options.iter().any(|option| {
        option.id == "search_corpse:corpse:2"
            && matches!(
                option.command.as_ref().map(|command| &command.intent),
                Some(tme_rules::PlayerIntentPayloadV1::SearchCorpse { corpse_id })
                    if corpse_id == &second_corpse
            )
    }));

    let newest_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(second_corpse.clone()),
        )
        .expect("newest empty corpse should be searchable");
    assert!(matches!(
        newest_events
            .events
            .iter()
            .find(|event| matches!(event, Event::CorpseSearched { .. })),
        Some(Event::CorpseSearched {
            corpse_id: event_corpse_id,
            items_released: 0,
            gold_released: 0,
            ..
        }) if event_corpse_id == &second_corpse
    ));
    assert!(engine.world().corpses[&second_corpse].searched);
    assert!(!engine.world().corpses[&first_corpse].searched);

    let older_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(first_corpse.clone()),
        )
        .expect("older retained-loot corpse should be searchable");
    let item = event_position(&older_events.events, |event| {
        matches!(event, Event::ItemRelocated {
            item_instance_id,
            reason: ItemRelocationReason::CorpseSearch,
            ..
        } if item_instance_id == "cloth_bundle")
    });
    let gold = event_position(&older_events.events, |event| {
        matches!(
            event,
            Event::GoldRelocated {
                amount: 3,
                reason: GoldRelocationReason::CorpseSearch,
                ..
            }
        )
    });
    let searched = event_position(&older_events.events, |event| {
        matches!(event, Event::CorpseSearched {
            corpse_id: event_corpse_id,
            items_released: 1,
            gold_released: 3,
            ..
        } if event_corpse_id == &first_corpse)
    });
    assert!(item < gold && gold < searched);
    let older = &engine.world().corpses[&first_corpse];
    assert!(older.searched);
    assert!(older.contents.is_empty());
    assert_eq!(older.gold, 0);
    assert_eq!(engine.world().ground_gold.len(), 1);
    assert_eq!(
        engine.world().ground_gold.values().next().unwrap().amount,
        3
    );
    assert_eq!(engine.world().next_gold_sequence, 2);

    let before_repeat = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(first_corpse),
        )
        .expect_err("a corpse may be searched only once");
    assert!(error.to_string().contains("corpse already searched"));
    assert_eq!(engine.world(), &before_repeat);
}

#[test]
fn player_defeat_creates_a_stationary_ghost_and_self_claimed_death_pile() {
    let mut engine = engine();
    let events = defeat_player(&mut engine);
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    let player_corpse_id = corpse_id(3);

    assert_eq!(player.hp, 0);
    assert_eq!(player.location.position, (1, 1).into());
    assert!(matches!(
        &player.life_state,
        ActorLifeState::Ghost {
            corpse_id,
            defeated_at: LogicalTime { .. },
        } if corpse_id == &player_corpse_id
    ));
    assert_eq!(player.character.as_ref().unwrap().resources.hp, 0);
    let corpse = &engine.world().corpses[&player_corpse_id];
    assert_eq!(corpse.contents[&CarriedPosition::SackItem1], "flint");
    assert_eq!(corpse.gold, 2);
    let claim = corpse.loot_claim.as_ref().expect("player death claim");
    assert_eq!(claim.basis, LootClaimBasis::CharacterDeathPile);
    assert!(matches!(
        &claim.owner,
        LootOwnerId::Character(character_id)
            if character_id.as_str() == "character:death_corpse:primary"
    ));
    assert_eq!(
        engine.item_location("oak_club").unwrap(),
        ItemLocation::Ground {
            position: WorldPosition::new("realm_0", "room_0", (1, 1).into())
        }
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ResurrectionRequested { .. } | Event::ActorResurrected { .. }
    )));

    let attacked = event_position(
        &events,
        |event| matches!(event, Event::Attacked { defender_id, defender_hp: 0, .. } if defender_id == "player"),
    );
    let defeated = event_position(&events, |event| {
        matches!(event, Event::ActorDefeated {
            actor_id,
            cause: DeathCause::Physical,
            credited_actor_id: Some(credited),
            ..
        } if actor_id == "player" && credited == "brute")
    });
    let created = event_position(
        &events,
        |event| matches!(event, Event::CorpseCreated { corpse_id, .. } if corpse_id == &player_corpse_id),
    );
    let life = event_position(
        &events,
        |event| matches!(event, Event::ActorLifeStateChanged { actor_id, .. } if actor_id == "player"),
    );
    assert!(attacked < defeated && defeated < created && created < life);

    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("ghost context remains readable");
    assert!(!context.can_act);
    assert!(matches!(
        context.life_state,
        tme_rules::ActorLifeStateViewV1::Ghost { .. }
    ));
    assert!(
        engine
            .actor_action_options(&tme_rules::ActorId::from("player"))
            .expect("ghost options")
            .iter()
            .all(|option| !option.enabled)
    );
    let command = engine
        .actor_command_for_intent(&tme_rules::ActorId::from("player"), &PlayerIntent::Wait)
        .expect("typed wait command");
    let status = engine.validate_actor_command(&command).expect("validation");
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::ActorNotLiving)
    );
    let before_rejected_step = engine.world().clone();
    assert!(
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .is_err()
    );
    assert_eq!(engine.world(), &before_rejected_step);
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("ghost observation remains available");
    assert_eq!(observed.observation_center.position, (1, 1).into());
    assert_eq!(observed.observation_radius, 7);
}

#[test]
fn corpse_backed_resurrection_restores_only_retained_state_and_is_transactional() {
    let mut engine = engine();
    defeat_player(&mut engine);
    let id = corpse_id(3);

    let before_invalid = engine.world().clone();
    let invalid = ResurrectionRequest {
        actor_id: "player".into(),
        corpse_id: Some(id.clone()),
        method: ResurrectionMethod::Priest,
        destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
        current_hp: 0,
        current_stamina: 7,
    };
    assert!(engine.resurrect(invalid).is_err());
    assert_eq!(engine.world(), &before_invalid);

    let request = ResurrectionRequest {
        actor_id: "player".into(),
        corpse_id: Some(id.clone()),
        method: ResurrectionMethod::Priest,
        destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
        current_hp: 4,
        current_stamina: 7,
    };
    let events = engine.resurrect(request).expect("valid resurrection");

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(player.life_state, ActorLifeState::Alive);
    assert_eq!(player.location.position, (2, 1).into());
    assert_eq!(player.hp, 4);
    assert_eq!(player.character.as_ref().unwrap().resources.hp, 4);
    assert_eq!(player.character.as_ref().unwrap().resources.stamina, 7);
    assert_eq!(player.carried.gold.sack, 2);
    assert_eq!(player.carried.items[&CarriedPosition::SackItem1], "flint");
    assert!(!engine.world().corpses.contains_key(&id));
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "oak_club")
    );
    assert_eq!(
        engine.world().ground_gold.values().next().unwrap().amount,
        3
    );

    let item = event_position(&events, |event| {
        matches!(event, Event::ItemRelocated {
            item_instance_id,
            reason: ItemRelocationReason::ResurrectionReturn,
            ..
        } if item_instance_id == "flint")
    });
    let gold = event_position(&events, |event| {
        matches!(
            event,
            Event::GoldRelocated {
                amount: 2,
                reason: GoldRelocationReason::ResurrectionReturn,
                ..
            }
        )
    });
    let removed = event_position(
        &events,
        |event| matches!(event, Event::CorpseRemoved { corpse_id, .. } if corpse_id == &id),
    );
    let life = event_position(
        &events,
        |event| matches!(event, Event::ActorLifeStateChanged { actor_id, .. } if actor_id == "player"),
    );
    let resurrected = event_position(
        &events,
        |event| matches!(event, Event::ActorResurrected { actor_id, .. } if actor_id == "player"),
    );
    assert!(item < gold && gold < removed && removed < life && life < resurrected);
}

#[test]
fn invalid_resurrection_origins_resources_destinations_and_capacity_never_mutate() {
    let mut base = engine();
    defeat_player(&mut base);
    let id = corpse_id(3);
    let valid_request = || ResurrectionRequest {
        actor_id: "player".into(),
        corpse_id: Some(id.clone()),
        method: ResurrectionMethod::Thaumaturge,
        destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
        current_hp: 3,
        current_stamina: 0,
    };

    let mut requests = Vec::new();
    let mut wrong_actor = valid_request();
    wrong_actor.actor_id = "brute".into();
    requests.push(wrong_actor);
    let mut wrong_corpse = valid_request();
    wrong_corpse.corpse_id = Some(corpse_id(1));
    requests.push(wrong_corpse);
    let mut no_corpse = valid_request();
    no_corpse.corpse_id = None;
    requests.push(no_corpse);
    let mut wall = valid_request();
    wall.destination = WorldPosition::new("realm_0", "room_0", (0, 0).into());
    requests.push(wall);
    let mut zero_hp = valid_request();
    zero_hp.current_hp = 0;
    requests.push(zero_hp);
    let mut over_hp = valid_request();
    over_hp.current_hp = 7;
    requests.push(over_hp);
    let mut bad_stamina = valid_request();
    bad_stamina.current_stamina = 11;
    requests.push(bad_stamina);

    for request in requests {
        let mut candidate = base.clone();
        let before = candidate.world().clone();
        assert!(candidate.resurrect(request).is_err());
        assert_eq!(candidate.world(), &before);
    }

    let mut wrong_origin = base.clone();
    wrong_origin
        .world_mut()
        .corpses
        .get_mut(&id)
        .unwrap()
        .origin_actor_id = "brute".into();
    let before = wrong_origin.world().clone();
    assert!(wrong_origin.resurrect(valid_request()).is_err());
    assert_eq!(wrong_origin.world(), &before);

    let mut missing = base.clone();
    missing.world_mut().corpses.remove(&id);
    let before = missing.world().clone();
    assert!(missing.resurrect(valid_request()).is_err());
    assert_eq!(missing.world(), &before);

    let mut overflow = base.clone();
    overflow.world_mut().actors[0].carried.gold.sack = i64::MAX;
    let before = overflow.world().clone();
    assert!(overflow.resurrect(valid_request()).is_err());
    assert_eq!(overflow.world(), &before);

    let mut occupied = base.clone();
    occupied
        .world_mut()
        .ground_items
        .retain(|item| item.item_instance_id != "rusted_knife");
    occupied.world_mut().actors[0]
        .carried
        .items
        .insert(CarriedPosition::SackItem1, "rusted_knife".to_string());
    let before = occupied.world().clone();
    assert!(occupied.resurrect(valid_request()).is_err());
    assert_eq!(occupied.world(), &before);
}

#[test]
fn priest_resurrection_preserves_protected_progression_mp_dexterity_skills_and_peak_hp() {
    let mut engine = engine();
    {
        let actor = &mut engine.world_mut().actors[0];
        actor.mp = 3;
        actor.stamina = 0;
        let character = actor.character.as_mut().unwrap();
        character.progression.experience = 100;
        character.attributes.dexterity = 14;
        character.resources.mp = 3;
        character.resources.max_mp = 5;
        character.resources.stamina = 0;
        character.resources.peak_hp = 10;
        character.skill_ledger.push(SkillEntry {
            track_id: "priest_test".to_string(),
            level: 2,
            critique_rank: 1,
            practice_points: 7,
            learning_rate: 3,
        });
    }
    defeat_player(&mut engine);
    let protected = engine.world().actors[0].character.as_ref().unwrap().clone();
    let events = engine
        .resurrect(ResurrectionRequest {
            actor_id: "player".into(),
            corpse_id: Some(corpse_id(3)),
            method: ResurrectionMethod::Priest,
            destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
            current_hp: 4,
            current_stamina: 7,
        })
        .expect("Priest resurrection should succeed");
    let character = engine.world().actors[0].character.as_ref().unwrap();
    assert_eq!(character.progression, protected.progression);
    assert_eq!(character.resources.mp, protected.resources.mp);
    assert_eq!(character.resources.max_mp, protected.resources.max_mp);
    assert_eq!(
        character.attributes.dexterity,
        protected.attributes.dexterity
    );
    assert_eq!(character.skill_ledger, protected.skill_ledger);
    assert_eq!(character.resources.peak_hp, protected.resources.peak_hp);
    assert_eq!(character.progression.level, 1);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::LevelGained { .. }))
    );
}

#[test]
fn thaumaturge_requires_rounded_up_half_max_hp_and_zero_stamina() {
    let mut base = engine();
    {
        let actor = &mut base.world_mut().actors[0];
        actor.hp = 7;
        let resources = &mut actor.character.as_mut().unwrap().resources;
        resources.hp = 7;
        resources.max_hp = 7;
        resources.peak_hp = 7;
    }
    defeat_player(&mut base);
    let id = corpse_id(3);

    for (hp, stamina) in [(3, 0), (4, 1), (5, 0)] {
        let mut candidate = base.clone();
        let before = candidate.world().clone();
        assert!(
            candidate
                .resurrect(ResurrectionRequest {
                    actor_id: "player".into(),
                    corpse_id: Some(id.clone()),
                    method: ResurrectionMethod::Thaumaturge,
                    destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
                    current_hp: hp,
                    current_stamina: stamina,
                })
                .is_err()
        );
        assert_eq!(candidate.world(), &before);
    }

    let events = base
        .resurrect(ResurrectionRequest {
            actor_id: "player".into(),
            corpse_id: Some(id),
            method: ResurrectionMethod::Thaumaturge,
            destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
            current_hp: 4,
            current_stamina: 0,
        })
        .expect("odd maximum rounds upward for the project transaction");
    assert_eq!(base.world().actors[0].hp, 4);
    assert_eq!(base.world().actors[0].stamina, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::LevelGained { .. }))
    );
}

#[test]
fn gods_and_priest_reject_full_current_pools_and_leave_pending_level_unapplied() {
    let mut gods_base = engine();
    gods_base.world_mut().actors[0].corpse_disposition = CorpseDisposition::None;
    set_pending_and_spent_stamina(&mut gods_base);
    defeat_player(&mut gods_base);
    assert!(matches!(
        gods_base.world().actors[0].life_state,
        ActorLifeState::AwaitingResurrection { .. }
    ));

    for (hp, stamina) in [(6, 7), (4, 10)] {
        let mut candidate = gods_base.clone();
        let before = candidate.world().clone();
        assert!(
            candidate
                .resurrect(ResurrectionRequest {
                    actor_id: "player".into(),
                    corpse_id: None,
                    method: ResurrectionMethod::Gods,
                    destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
                    current_hp: hp,
                    current_stamina: stamina,
                })
                .is_err()
        );
        assert_eq!(candidate.world(), &before);
    }
    let gods_events = gods_base
        .resurrect(ResurrectionRequest {
            actor_id: "player".into(),
            corpse_id: None,
            method: ResurrectionMethod::Gods,
            destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
            current_hp: 4,
            current_stamina: 7,
        })
        .expect("below-maximum gods result should succeed");
    assert!(
        !gods_events
            .iter()
            .any(|event| matches!(event, Event::LevelGained { .. }))
    );
    assert_eq!(
        gods_base.world().actors[0]
            .character
            .as_ref()
            .unwrap()
            .progression
            .level,
        1
    );

    let mut priest_base = engine();
    set_pending_and_spent_stamina(&mut priest_base);
    defeat_player(&mut priest_base);
    let id = corpse_id(3);
    for (hp, stamina) in [(6, 7), (4, 10)] {
        let mut candidate = priest_base.clone();
        let before = candidate.world().clone();
        assert!(
            candidate
                .resurrect(ResurrectionRequest {
                    actor_id: "player".into(),
                    corpse_id: Some(id.clone()),
                    method: ResurrectionMethod::Priest,
                    destination: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
                    current_hp: hp,
                    current_stamina: stamina,
                })
                .is_err()
        );
        assert_eq!(candidate.world(), &before);
    }
}

fn set_pending_and_spent_stamina(engine: &mut Engine) {
    let actor = &mut engine.world_mut().actors[0];
    actor.stamina = 0;
    let character = actor.character.as_mut().unwrap();
    character.resources.stamina = 0;
    character.progression.experience = 100;
}
