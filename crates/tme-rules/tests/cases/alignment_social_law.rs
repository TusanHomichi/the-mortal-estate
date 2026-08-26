use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::view::{
    LawZoneViewV1, PublicHostilityReasonV1, SocialBehaviorViewV1, SocialOwnerRelationViewV1,
    SpellTownLawViewV1,
};
use tme_rules::{
    AccountMarkAssessmentReasonV1, ActorKind, AlignmentChangeReasonV1, AutomaticActorDecisionV1,
    AutomaticMovementPurposeV1, CharacterAlignment, CharacterId, ClassDemotionReasonV1, Coord,
    Direction, Engine, Event, KarmaChangeReasonV1, LogicalTime, NpcGrudgeReasonV1, NpcState,
    PhysicalAttackMode, PlayerIntent, SelfDefenseChangeReasonV1, SocialAlignmentSource,
    SocialBehavior, SocialNature, SocialOwnerRelation, SocialProfile, SpellTarget, WorldPosition,
};

fn fixture(case_id: &str) -> ContentParts {
    let profile = format!("profile/{case_id}");
    ContentParts::tracked(case_id, &profile)
}

fn engine(value: ContentParts, seed: u64) -> Engine {
    value
        .engine(seed)
        .expect("purpose-built graph should start")
}

fn character_id(value: &str) -> CharacterId {
    serde_json::from_value(json!(value)).expect("test character ID should deserialize")
}

fn turn_actor_into_character_player(
    engine: &mut Engine,
    actor_index: usize,
    stable_character_id: &str,
    current_class_id: &str,
    alignment: CharacterAlignment,
) {
    let mut character = engine.world().actors[0]
        .character
        .clone()
        .expect("fixture player should be character-backed");
    character.identity.base_class_id = current_class_id.to_string();
    character.identity.current_class_id = current_class_id.to_string();
    character.identity.display_class = match current_class_id {
        "fighter" => "Fighter",
        "knight" => "Knight",
        "thief" => "Thief",
        "martial_artist" => "Martial Artist",
        other => other,
    }
    .to_string();
    character.alignment_state.alignment = alignment;
    character.alignment_state.karma_points = 0;
    character.promotion_history.clear();

    let player_position = engine.world().actors[0].location.position;
    let target = &mut engine.world_mut().actors[actor_index];
    target.kind = ActorKind::Player;
    target.creature_traits.clear();
    target.social = SocialProfile {
        alignment_source: SocialAlignmentSource::Character {},
        nature: SocialNature::Human,
        behavior: SocialBehavior::Adventurer,
        owner_relation: SocialOwnerRelation::None,
    };
    target.location.position = player_position;
    target.home_location.position = player_position;
    target.ai = None;
    target.npc = None;
    target.xp_value = 0;
    target.character_id = Some(character_id(stable_character_id));
    target.hp = character.resources.hp;
    target.mp = character.resources.mp;
    target.stamina = character.resources.stamina;
    target.stats.hp = character.resources.max_hp;
    target.character = Some(character);
    target.monster_abilities.clear();
    target.summoned = None;
    target.timing.ready_at = LogicalTime::new(u64::MAX);
}

fn event_index(events: &[Event], predicate: impl Fn(&Event) -> bool, label: &str) -> usize {
    events
        .iter()
        .position(predicate)
        .unwrap_or_else(|| panic!("missing {label} event in {events:#?}"))
}

#[test]
fn neutral_thief_truth_is_private_and_only_locked_detection_changes_apparent_alignment() {
    let mut engine = engine(fixture("character_sheet"), 7);
    turn_actor_into_character_player(
        &mut engine,
        1,
        "character:alignment-social-law:thief",
        "thief",
        CharacterAlignment::Neutral,
    );

    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("normal observer snapshot should build");
    let thief = observed
        .actors
        .iter()
        .find(|actor| actor.id == "mireling")
        .expect("neutral Thief should be visible");
    assert_eq!(
        thief.social.attack_safety,
        tme_rules::AttackSafety::Protected
    );
    assert_eq!(
        thief.character, None,
        "foreign character truth must be absent"
    );

    let debug = engine.snapshot();
    let true_thief = debug
        .actors
        .iter()
        .find(|actor| actor.id == "mireling")
        .expect("debug snapshot should include Thief");
    assert_eq!(true_thief.social.alignment, CharacterAlignment::Neutral);
    assert!(true_thief.character.is_some());

    let observer = engine.world_mut().actors[0]
        .character
        .as_mut()
        .expect("observer should be character-backed");
    observer.identity.current_class_id = "knight".to_string();
    observer.identity.display_class = "Knight".to_string();
    let detected = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("Knight observer snapshot should build");
    let thief = detected
        .actors
        .iter()
        .find(|actor| actor.id == "mireling")
        .expect("neutral Thief should remain visible");
    assert_eq!(
        thief.social.attack_safety,
        tme_rules::AttackSafety::Protected
    );
    assert_eq!(
        thief.character, None,
        "detection must not disclose character truth"
    );
}

#[test]
fn observed_hostility_reports_the_targets_direction_toward_the_observer() {
    let engine = engine(fixture("summons_created_creature_lifecycle"), 7);
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should build");
    let ash_imp = observed
        .actors
        .iter()
        .find(|actor| actor.id == "ash_imp")
        .expect("chaotic alignment creature should be visible");

    assert!(ash_imp.social.hostile_to_observer);
    assert_eq!(
        ash_imp.social.hostility_reason,
        PublicHostilityReasonV1::ChaoticOpposition
    );
    assert_eq!(
        ash_imp.social.attack_safety,
        tme_rules::AttackSafety::OpenHostile
    );
}

#[test]
fn force_must_match_apparent_alignment_and_missed_contact_grants_exact_self_defense() {
    let mut engine = engine(fixture("character_sheet"), 7);
    turn_actor_into_character_player(
        &mut engine,
        1,
        "character:alignment-social-law:defender",
        "fighter",
        CharacterAlignment::Lawful,
    );
    engine.world_mut().actors[0].stats.attack = 0;
    engine.world_mut().actors[1].stats.defense = 100;

    let kick = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context should build")
        .attack_targets
        .into_iter()
        .find(|target| target.actor_id == "mireling")
        .expect("lawful target should be actionable")
        .physical_attacks
        .into_iter()
        .find(|option| option.mode == PhysicalAttackMode::Kick)
        .expect("kick option should be present");
    assert_eq!(kick.attack_safety, tme_rules::AttackSafety::Protected);
    assert!(kick.enabled);

    let mismatch = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "mireling".into(),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("apparently lawful target should require force");
    assert!(
        mismatch
            .message()
            .contains("protected_target_requires_confirmation")
    );
    assert!(engine.snapshot().social_relations.self_defense.is_empty());

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "mireling".into(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("matching force should commit attack");
    let relation_index = event_index(
        &events.events,
        |event| {
            matches!(
                event,
                Event::SelfDefenseChanged(change)
                    if change.victim_actor_id == "mireling"
                        && change.after_attacker_character_id.as_ref()
                            == engine.world().actors[0].character_id.as_ref()
                        && change.reason == SelfDefenseChangeReasonV1::Established
            )
        },
        "self-defense establishment",
    );
    let miss_index = event_index(
        &events.events,
        |event| matches!(event, Event::AttackMissed { defender_id, .. } if defender_id == "mireling"),
        "miss",
    );
    assert!(
        relation_index < miss_index,
        "miss is still physical contact"
    );
    assert!(
        engine
            .snapshot()
            .social_relations
            .self_defense
            .iter()
            .any(|relation| relation.victim_character_id
                == *engine.world().actors[1].character_id.as_ref().unwrap()
                && relation.attacker_character_id
                    == *engine.world().actors[0].character_id.as_ref().unwrap())
    );

    let now = engine.world().timing.now;
    engine.world_mut().actors[0].timing.ready_at = LogicalTime::new(u64::MAX);
    engine.world_mut().actors[1].timing.ready_at = now;
    engine.world_mut().actors[1].attack_ready_at = now;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("mireling"),
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "player".into(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("confirmed unsafe remains valid after the target becomes open");
}

#[test]
fn blocked_contact_also_establishes_self_defense_before_the_block_receipt() {
    let mut engine = engine(fixture("martial_hand_block_actions"), 7);
    turn_actor_into_character_player(
        &mut engine,
        1,
        "character:alignment-social-law:block-attacker",
        "fighter",
        CharacterAlignment::Lawful,
    );
    let now = engine.world().timing.now;
    engine.world_mut().actors[0].timing.ready_at = LogicalTime::new(u64::MAX);
    engine.world_mut().actors[1].timing.ready_at = now;
    engine.world_mut().actors[1].attack_ready_at = now;

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("sparring_beast"),
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "player0".into(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("forced lawful attack should resolve into martial block");
    let relation_index = event_index(
        &events.events,
        |event| {
            matches!(
                event,
                Event::SelfDefenseChanged(change)
                    if change.victim_actor_id == "player0"
                        && change.after_attacker_character_id.as_ref()
                            == engine.world().actors[1].character_id.as_ref()
            )
        },
        "blocked-contact relation",
    );
    let block_index = event_index(
        &events.events,
        |event| matches!(event, Event::AttackBlocked { defender_id, .. } if defender_id == "player0"),
        "attack block",
    );
    assert!(
        relation_index < block_index,
        "block is still physical contact"
    );
}

#[test]
fn hostile_spell_contacts_lawful_npc_before_resistance_and_debug_surfaces_are_authoritative() {
    let mut engine = engine(fixture("spell_effects"), 7);
    {
        let npc = &mut engine.world_mut().actors[1];
        npc.kind = ActorKind::Npc;
        npc.social = SocialProfile {
            alignment_source: SocialAlignmentSource::Inherent {
                alignment: CharacterAlignment::Lawful,
            },
            nature: SocialNature::Human,
            behavior: SocialBehavior::Civilian,
            owner_relation: SocialOwnerRelation::None,
        };
        npc.npc = Some(NpcState {
            follow_cadence_units: 1,
            interactions: Vec::new(),
            following_character_id: None,
        });
        npc.magic_resistance.natural_save_twentieths = 20;
    }

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("hostile spell should resolve against lawful NPC");
    let grudge_index = event_index(
        &events.events,
        |event| {
            matches!(
                event,
                Event::NpcGrudgeEstablished {
                    npc_actor_id,
                    attacker_actor_id,
                    reason: NpcGrudgeReasonV1::HostileSpellContact,
                } if npc_actor_id == "target" && attacker_actor_id == "player"
            )
        },
        "NPC hostile-spell grudge",
    );
    let save_index = event_index(
        &events.events,
        |event| {
            matches!(
                event,
                Event::SpellSaveResolved {
                    actor_id,
                    effect_id,
                    natural_save_twentieths: 20,
                    success: true,
                    ..
                } if actor_id == "target" && effect_id == "spark"
            )
        },
        "spell resistance",
    );
    assert!(
        grudge_index < save_index,
        "contact must commit before resistance"
    );

    let debug = engine.snapshot();
    assert!(debug.social_relations.npc_grudges.iter().any(|relation| {
        relation.npc_actor_id == "target" && relation.attacker_actor_id == "player"
    }));
    let spark = debug
        .spell_social
        .iter()
        .find(|spell| spell.spell_id == "spark")
        .expect("debug spell-social catalog should contain Spark");
    assert!(spark.social.hostile_act);
    assert_eq!(spark.social.town_law, SpellTownLawViewV1::Permitted);
}

#[test]
fn late_hostile_spell_failure_restores_social_spell_timing_and_rng_state() {
    let mut engine = engine(fixture("spell_effects"), 1_010_580_540);
    {
        let player = engine.world_mut().actors[0]
            .character
            .as_mut()
            .expect("spell caster should be character-backed");
        player.alignment_state.karma_points = u32::MAX;
    }
    {
        let target = &mut engine.world_mut().actors[1];
        target.kind = ActorKind::Npc;
        target.social = SocialProfile {
            alignment_source: SocialAlignmentSource::Inherent {
                alignment: CharacterAlignment::Lawful,
            },
            nature: SocialNature::Human,
            behavior: SocialBehavior::Civilian,
            owner_relation: SocialOwnerRelation::None,
        };
        target.npc = Some(NpcState {
            follow_cadence_units: 1,
            interactions: Vec::new(),
            following_character_id: None,
        });
        target.hp = 1;
        target.stats.hp = 1;
    }
    let mut control = engine.clone();
    let before_world = engine.world().clone();
    let before_snapshot = engine.snapshot();
    let intent = PlayerIntent::CastSpell {
        spell_id: "spark".to_string(),
        target: Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
        authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
    };

    let error = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent.clone())
        .expect_err("karma overflow should fail after hostile contact and lethal damage");
    assert!(error.to_string().contains("karma overflow"), "{error}");
    assert_eq!(engine.world(), &before_world);
    assert_eq!(engine.snapshot(), before_snapshot);

    for candidate in [&mut engine, &mut control] {
        candidate.world_mut().actors[0]
            .character
            .as_mut()
            .expect("spell caster should remain character-backed")
            .alignment_state
            .karma_points = 0;
    }
    let actual = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent.clone())
        .expect("retry should succeed after removing the overflow");
    let expected = control
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("control cast should succeed");
    assert_eq!(
        actual, expected,
        "retry must replay restored spell RNG exactly"
    );
    assert_eq!(engine.world(), control.world());
}

#[test]
fn unjust_lawful_human_kill_changes_alignment_karma_class_and_assesses_mark() {
    let mut engine = engine(fixture("knight_support_actions"), 3);
    turn_actor_into_character_player(
        &mut engine,
        1,
        "character:alignment-social-law:lawful-victim",
        "fighter",
        CharacterAlignment::Lawful,
    );
    engine.world_mut().actors[0].stats.attack = 100;
    engine.world_mut().actors[1].hp = 1;
    engine.world_mut().actors[1].stats.hp = 1;
    let victim_character = engine.world_mut().actors[1]
        .character
        .as_mut()
        .expect("victim should be character-backed");
    victim_character.resources.hp = 1;
    victim_character.resources.max_hp = 1;
    victim_character.resources.peak_hp = 1;
    let history_before = engine.world().actors[0]
        .character
        .as_ref()
        .expect("Knight character")
        .promotion_history
        .clone();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "training_dummy".into(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("unjust lawful-human lethal attack should resolve");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::AlignmentChanged {
            actor_id,
            before: CharacterAlignment::Lawful,
            after: CharacterAlignment::Neutral,
            reason: AlignmentChangeReasonV1::UnjustLawfulHumanKill,
            ..
        } if actor_id == "player"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::KarmaChanged {
            actor_id,
            before: 0,
            after: 1,
            delta: 1,
            reason: KarmaChangeReasonV1::UnjustLawfulHumanKill,
            victim_actor_id,
            ..
        } if actor_id == "player" && victim_actor_id == "training_dummy"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AccountMarkAssessed {
            killer_actor_id,
            victim_actor_id,
            credited_source_actor_id,
            assessed: true,
            reason: AccountMarkAssessmentReasonV1::AddForPlayerKill,
            ..
        } if killer_actor_id == "player"
            && victim_actor_id == "training_dummy"
            && credited_source_actor_id == "player"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ClassDemoted {
            actor_id,
            from_class_id,
            to_class_id,
            reason: ClassDemotionReasonV1::UnjustLawfulHumanKill,
            victim_actor_id,
            ..
        } if actor_id == "player"
            && from_class_id == "knight"
            && to_class_id == "fighter"
            && victim_actor_id == "training_dummy"
    )));

    let knight = engine.world().actors[0]
        .character
        .as_ref()
        .expect("killer remains character-backed");
    assert_eq!(
        knight.alignment_state.alignment,
        CharacterAlignment::Neutral
    );
    assert_eq!(knight.alignment_state.karma_points, 1);
    assert_eq!(knight.identity.current_class_id, "fighter");
    assert_eq!(knight.identity.display_class, "Fighter");
    assert_eq!(knight.promotion_history, history_before);
    assert!(
        engine.world().actors[0]
            .carried
            .items
            .values()
            .any(|item| item == "oath_ring")
    );
}

#[test]
fn lawful_animal_kill_changes_alignment_without_karma_or_mark() {
    let mut engine = engine(fixture("character_sheet"), 11);
    let player_position = engine.world().actors[0].location.position;
    {
        let player = &mut engine.world_mut().actors[0];
        player.stats.attack = 100;
    }
    {
        let animal = &mut engine.world_mut().actors[1];
        animal.social = SocialProfile {
            alignment_source: SocialAlignmentSource::Inherent {
                alignment: CharacterAlignment::Lawful,
            },
            nature: SocialNature::Animal,
            behavior: SocialBehavior::Passive,
            owner_relation: SocialOwnerRelation::None,
        };
        animal.location.position = player_position;
        animal.hp = 1;
        animal.stats.hp = 1;
    }

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "mireling".into(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("lawful-animal lethal attack should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AlignmentChanged {
            before: CharacterAlignment::Lawful,
            after: CharacterAlignment::Neutral,
            reason: AlignmentChangeReasonV1::UnjustLawfulAnimalKill,
            ..
        }
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::KarmaChanged { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::AccountMarkAssessed { .. }))
    );
    let alignment = &engine.world().actors[0]
        .character
        .as_ref()
        .expect("player character")
        .alignment_state;
    assert_eq!(alignment.alignment, CharacterAlignment::Neutral);
    assert_eq!(alignment.karma_points, 0);
}

#[test]
fn town_terrain_cast_changes_alignment_but_preserves_knight_class_karma_and_ring() {
    let mut value = fixture("knight_support_actions");
    value.template_levels_source_mut()["room_0"]["law_zone"] = json!("town");
    let beacon = value.selected_by_runtime_id_mut("spells", "beacon");
    beacon["social"]["town_law"] = json!("terrain_alignment_violation");
    let mut engine = engine(value, 7);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "beacon".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("town terrain cast should commit");
    let alignment_index = event_index(
        &events.events,
        |event| {
            matches!(
                event,
                Event::AlignmentChanged {
                    reason: AlignmentChangeReasonV1::TownTerrainCast,
                    before: CharacterAlignment::Lawful,
                    after: CharacterAlignment::Neutral,
                    ..
                }
            )
        },
        "town alignment consequence",
    );
    let effect_index = event_index(
        &events.events,
        |event| matches!(event, Event::TileEffectApplied { effect_id, .. } if effect_id == "beacon"),
        "Beacon tile effect",
    );
    assert!(
        alignment_index < effect_index,
        "town law commits before spell effect"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::KarmaChanged { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ClassDemoted { .. }))
    );

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.realms[0].levels[0].law_zone, LawZoneViewV1::Town);
    let beacon_social = snapshot
        .spell_social
        .iter()
        .find(|spell| spell.spell_id == "beacon")
        .expect("debug spell-social catalog should contain Beacon");
    assert_eq!(
        beacon_social.social.town_law,
        SpellTownLawViewV1::TerrainAlignmentViolation
    );
    let character = engine.world().actors[0]
        .character
        .as_ref()
        .expect("Knight should remain character-backed");
    assert_eq!(
        character.alignment_state.alignment,
        CharacterAlignment::Neutral
    );
    assert_eq!(character.alignment_state.karma_points, 0);
    assert_eq!(character.identity.current_class_id, "knight");
    assert!(
        engine.world().actors[0]
            .carried
            .items
            .values()
            .any(|item| item == "oath_ring")
    );
}

#[test]
fn late_town_law_spell_failure_restores_alignment_tile_cost_and_time() {
    let mut value = fixture("area_path_terrain_spells");
    value.template_levels_source_mut()["room_0"]["law_zone"] = json!("town");
    let web = value.selected_by_runtime_id_mut("spells", "web_field");
    web["social"]["town_law"] = json!("terrain_alignment_violation");
    let mut engine = engine(value, 7);
    engine.world_mut().actors[0]
        .character
        .as_mut()
        .expect("caster should remain character-backed")
        .skill_ledger[0]
        .practice_points = u64::MAX;
    let mut control = engine.clone();
    let before_world = engine.world().clone();
    let before_snapshot = engine.snapshot();
    let intent = PlayerIntent::CastSpell {
        spell_id: "web_field".to_string(),
        target: Some(SpellTarget::Area {
            center: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 2 }),
        }),
        authorization: tme_rules::HostilityAuthorization::Safe,
    };

    let error = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent.clone())
        .expect_err("casting-practice overflow should fail after town law and tile mutation");
    assert!(
        error
            .to_string()
            .contains("practice pool must not overflow"),
        "{error}"
    );
    assert_eq!(engine.world(), &before_world);
    assert_eq!(engine.snapshot(), before_snapshot);

    for candidate in [&mut engine, &mut control] {
        candidate.world_mut().actors[0]
            .character
            .as_mut()
            .expect("caster should remain character-backed")
            .skill_ledger[0]
            .practice_points = 0;
    }
    let actual = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent.clone())
        .expect("retry should succeed after clearing the overflow");
    let expected = control
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("control town-law cast should succeed");
    assert_eq!(actual, expected, "retry must replay restored RNG exactly");
    assert_eq!(engine.world(), control.world());
}

#[test]
fn summoned_alignment_creature_protects_owner_and_targets_other_opposition() {
    let mut value = fixture("summons_created_creature_lifecycle");
    value.summon_actor_definition_mut(0)["social"]["alignment_source"] =
        json!({"kind": "inherent", "alignment": "chaotic"});
    value.actor_definition_mut(1)["social"]["alignment_source"] =
        json!({"kind": "inherent", "alignment": "neutral"});
    let mut engine = engine(value, 7);
    let owner_hp = engine.world().actors[0].hp;
    let events = engine
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
        .expect("summon cast should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorSummoned {
            actor_id,
            owner_id,
            social,
            ..
        } if actor_id == "summon:call_echo:1:echo_guardian"
            && owner_id == "player"
            && social.alignment_source == SocialAlignmentSource::Inherent {
                alignment: CharacterAlignment::Chaotic,
            }
            && social.owner_relation == SocialOwnerRelation::Summoner
    )));
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::AutomaticActorDecision {
                actor_id,
                decision: AutomaticActorDecisionV1::Move {
                    direction: Direction::East,
                    purpose: AutomaticMovementPurposeV1::Chase,
                },
                ..
            } if actor_id == "summon:call_echo:1:echo_guardian"
        )),
        "owned summon should chase the opposing actor, not its owner: {events:#?}"
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::PhysicalAttack { target_id, .. },
            ..
        } if actor_id == "summon:call_echo:1:echo_guardian" && target_id == "player"
    )));
    assert_eq!(engine.world().actors[0].hp, owner_hp);

    let summon = engine
        .snapshot()
        .actors
        .into_iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summon should remain in debug snapshot");
    assert_eq!(
        summon.social.owner_relation,
        SocialOwnerRelationViewV1::Summoner
    );
    assert_eq!(
        summon.social.behavior,
        SocialBehaviorViewV1::AlignmentCreature
    );
}
