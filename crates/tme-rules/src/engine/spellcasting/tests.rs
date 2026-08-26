use crate::Engine;
use crate::events::{AutomaticActorDecisionV1, AutomaticWaitReasonV1, Event};
use crate::model::{
    ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, ActorKind,
    CharacterAlignment, CreatureTrait, MonsterAbilityTargetPolicy, NpcState,
    ResistanceBoostSourceKind, SocialAlignmentSource, SocialBehavior, SocialNature,
    SocialOwnerRelation, SpellResistanceBoost,
};

use super::{HostileSpellReach, SpellEffectOutcome};

fn monster_actor_spell_engine(spell_id: &str, effect: serde_json::Value) -> Engine {
    let hostile_act = matches!(
        effect["family"].as_str(),
        Some(
            "banish"
                | "curse"
                | "direct_damage"
                | "instant_death"
                | "poison"
                | "turn_undead"
                | "control_status"
        )
    );
    let (mut catalog, profile, template, mut seed) =
        crate::engine::setup::test_parts("monster_spellcasting_special_attacks");
    let spell_key = catalog
        .profiles
        .get(&profile)
        .expect("test profile")
        .spells
        .iter()
        .find(|key| {
            catalog
                .spells
                .get(*key)
                .is_some_and(|spell| spell.id == "ember_spit")
        })
        .cloned()
        .expect("selected monster spell");
    let spell = catalog
        .spells
        .get_mut(&spell_key)
        .expect("selected monster spell definition");
    spell.id = spell_id.to_string();
    spell.name = "Monster Spell".to_string();
    spell.social.hostile_act = hostile_act;
    spell.effect =
        Some(serde_json::from_value(effect).expect("test spell effect should deserialize"));

    seed.actors
        .retain(|actor| matches!(actor.id.as_str(), "player" | "ember_imp"));
    let player_definition_id = seed
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("test player")
        .actor_definition_id
        .clone();
    let monster_definition_id = seed
        .actors
        .iter()
        .find(|actor| actor.id == "ember_imp")
        .expect("test monster")
        .actor_definition_id
        .clone();
    let player_definition = catalog
        .actor_definitions
        .values_mut()
        .find(|definition| definition.id == player_definition_id)
        .expect("test player definition");
    player_definition.stats.hp = 10;
    player_definition.stats.attack = 1;
    player_definition.stats.defense = 0;
    let player = seed
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("test player");
    player.location.position.x = 3;
    player.location.position.y = 1;
    player.active_effects.clear();
    player.carried.items.clear();

    let monster_definition = catalog
        .actor_definitions
        .values_mut()
        .find(|definition| definition.id == monster_definition_id)
        .expect("test monster definition");
    monster_definition.stats.hp = 6;
    monster_definition.stats.attack = 0;
    monster_definition.stats.defense = 0;
    monster_definition.monster_abilities = serde_json::from_value(serde_json::json!([
        {
            "id": "test_ability",
            "kind": "special_attack",
            "spell_id": spell_id,
            "cooldown_rounds": 2
        }
    ]))
    .expect("test monster ability should deserialize");
    let monster = seed
        .actors
        .iter_mut()
        .find(|actor| actor.id == "ember_imp")
        .expect("test monster");
    monster.location.position.x = 1;
    monster.location.position.y = 1;
    monster.carried.items.clear();

    seed.item_instances.clear();
    seed.ground_items.clear();
    seed.service_instances.clear();
    seed.merchant_inventories.clear();
    crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed)
}

fn direct_save_effect_value(mitigation: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "family": "direct_damage",
        "potency": 5,
        "damage_kind": "arcane",
        "resistance": {
            "role": "incoming",
            "tag": "arcane",
            "mitigation": mitigation
        }
    })
}

fn seeded_boost(instance_id: &str, tag: &str, bonus_twentieths: u32) -> ActiveEffectState {
    ActiveEffectState {
        source_actor_id: None,
        hostile_authority: None,
        spell_damage_credit: None,
        instance_id: instance_id.to_string(),
        effect_id: "test_ward".to_string(),
        source: ActiveEffectSource {
            kind: "test".to_string(),
            id: instance_id.to_string(),
        },
        kind: "resistance".to_string(),
        tags: vec!["ward".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        until_condition: None,
        stacking: ActiveEffectStackingPolicy::RefreshDuration,
        start_delay_rounds: 0,
        tick_interval_rounds: 1,
        suppresses_action: false,
        resistance_boosts: vec![SpellResistanceBoost {
            tag: tag.to_string(),
            bonus_twentieths,
        }],
        last_ticked_at: crate::LogicalTime::new(0),
    }
}

#[test]
fn resistance_plan_is_read_only_and_equality_succeeds_on_one_shared_roll() {
    let effect_value = direct_save_effect_value(serde_json::json!({
        "mode": "half_damage",
        "rounding": "down",
        "minimum_damage": 1
    }));
    let effect = serde_json::from_value(effect_value.clone()).expect("direct save effect");
    let mut engine = monster_actor_spell_engine("save_probe", effect_value);
    engine.world.actors[0]
        .magic_resistance
        .natural_save_twentieths = 11;
    let rng_before = engine.rng.clone();
    let plan = engine
        .plan_spell_resistance(0, "save_probe", &effect, Some(5))
        .expect("incoming effect has a save plan");
    assert_eq!(engine.rng, rng_before, "planning must not consume RNG");
    assert_eq!(plan.natural_save_twentieths, 11);
    assert_eq!(plan.save_twentieths, 11);

    let mut events = Vec::new();
    let resolution = engine.commit_spell_resistance(plan, &mut events);
    assert!(resolution.success, "seed 7 rolls 11, so equality succeeds");
    assert_eq!(resolution.resolved_damage, Some(2));
    assert_ne!(
        engine.rng, rng_before,
        "commit consumes the one shared roll"
    );
    assert!(matches!(
        events.as_slice(),
        [Event::SpellSaveResolved {
            roll: 11,
            save_twentieths: 11,
            success: true,
            requested_damage: Some(5),
            resolved_damage: Some(2),
            ..
        }]
    ));
}

#[test]
fn resistance_boundaries_zero_one_above_and_denominator_all_still_roll() {
    let effect_value = direct_save_effect_value(serde_json::json!({"mode": "negate"}));
    let effect = serde_json::from_value(effect_value.clone()).expect("direct save effect");
    for (natural, expected_success) in [(0, false), (10, false), (20, true)] {
        let mut engine = monster_actor_spell_engine("save_probe", effect_value.clone());
        engine.world.actors[0]
            .magic_resistance
            .natural_save_twentieths = natural;
        let rng_before = engine.rng.clone();
        let plan = engine
            .plan_spell_resistance(0, "save_probe", &effect, Some(5))
            .expect("save plan");
        let mut events = Vec::new();
        let resolution = engine.commit_spell_resistance(plan, &mut events);
        assert_eq!(resolution.success, expected_success);
        assert_ne!(engine.rng, rng_before);
        assert!(matches!(
            events.as_slice(),
            [Event::SpellSaveResolved { roll: 11, .. }]
        ));
    }
}

#[test]
fn resistance_selects_highest_matching_boost_with_stable_tie_source() {
    let effect_value = direct_save_effect_value(serde_json::json!({"mode": "negate"}));
    let effect = serde_json::from_value(effect_value.clone()).expect("direct save effect");
    let mut engine = monster_actor_spell_engine("save_probe", effect_value);
    engine.world.actors[0]
        .magic_resistance
        .natural_save_twentieths = 2;
    engine.world.actors[0].active_effects.extend([
        seeded_boost("ward:z", "arcane", 4),
        seeded_boost("ward:mismatch", "fire", 20),
        seeded_boost("ward:a", "arcane", 4),
        seeded_boost("ward:low", "arcane", 3),
    ]);
    let plan = engine
        .plan_spell_resistance(0, "save_probe", &effect, Some(5))
        .expect("save plan");
    assert_eq!(plan.save_twentieths, 6);
    let selected = plan.selected_boost.expect("matching boost");
    assert_eq!(selected.bonus_twentieths, 4);
    assert_eq!(
        selected.source_kind,
        ResistanceBoostSourceKind::ActiveEffect
    );
    assert_eq!(selected.source_id, "ward:a");
}

#[test]
fn resistance_mitigation_formulas_never_increase_damage() {
    for (mitigation, requested, expected) in [
        (
            serde_json::json!({"mode": "half_damage", "rounding": "down", "minimum_damage": 1}),
            5,
            2,
        ),
        (
            serde_json::json!({"mode": "half_damage", "rounding": "down", "minimum_damage": 3}),
            1,
            1,
        ),
        (
            serde_json::json!({"mode": "minimum_damage", "damage": 3}),
            5,
            3,
        ),
        (
            serde_json::json!({"mode": "minimum_damage", "damage": 3}),
            1,
            1,
        ),
        (serde_json::json!({"mode": "negate"}), 5, 0),
    ] {
        let effect_value = direct_save_effect_value(mitigation);
        let effect = serde_json::from_value(effect_value.clone()).expect("direct save effect");
        let mut engine = monster_actor_spell_engine("save_probe", effect_value);
        engine.world.actors[0]
            .magic_resistance
            .natural_save_twentieths = 20;
        let plan = engine
            .plan_spell_resistance(0, "save_probe", &effect, Some(requested))
            .expect("save plan");
        let resolution = engine.commit_spell_resistance(plan, &mut Vec::new());
        assert_eq!(resolution.resolved_damage, Some(expected));
        assert!(expected <= requested);
    }
}

#[test]
fn effect_without_incoming_resistance_has_no_plan_and_consumes_no_rng() {
    let effect: crate::content::SpellEffectDef = serde_json::from_value(serde_json::json!({
        "family": "healing",
        "potency": 2
    }))
    .expect("healing effect");
    let engine = monster_actor_spell_engine(
        "healing_probe",
        serde_json::json!({"family": "healing", "potency": 2}),
    );
    let rng_before = engine.rng.clone();
    assert!(
        engine
            .plan_spell_resistance(0, "healing_probe", &effect, None)
            .is_none()
    );
    assert_eq!(engine.rng, rng_before);
}

#[test]
fn monster_actor_spell_direct_damage_executes_without_player_spellbook_or_resources() {
    let mut engine = monster_actor_spell_engine(
        "ember_spit",
        serde_json::json!({
            "family": "direct_damage",
            "potency": 2,
            "damage_kind": "fire",
            "resistance": {"role": "incoming", "tag": "fire", "mitigation": {"mode": "minimum_damage", "damage": 2}}
        }),
    );
    let caster_index = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "ember_imp")
        .expect("caster");
    let target_index = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("target");

    let plan = engine
        .monster_spell_plan(
            caster_index,
            Some(target_index),
            "ember_spit",
            MonsterAbilityTargetPolicy::NearestHostile,
        )
        .expect("monster plan should bypass player spellbook gates");
    let mut events = Vec::new();
    engine
        .execute_actor_spell_effect(caster_index, &plan, &mut events)
        .expect("monster direct damage should execute");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            caster_id,
            spell_id,
            target_id,
            damage,
            hp,
            ..
        } if caster_id == "ember_imp"
            && spell_id == "ember_spit"
            && target_id == "player"
            && *damage == 2
            && *hp == 8
    )));
    assert!(!events.iter().any(|event| {
        matches!(event, Event::SkillPracticeAwarded { actor_id, .. } if actor_id == "ember_imp")
    }));
    assert_eq!(engine.world.actors[target_index].hp, 8);
}

#[test]
fn monster_actor_spell_status_executes_with_monster_source() {
    let mut engine = monster_actor_spell_engine(
        "stone_gaze",
        serde_json::json!({
            "family": "control_status",
            "status_kind": "stun",
            "resistance": {"role": "incoming", "tag": "stun", "mitigation": {"mode": "negate"}},
            "duration": {"policy": "rounds", "rounds": 1},
            "suppresses_action": true,
            "stacking": "refresh_duration"
        }),
    );
    let caster_index = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "ember_imp")
        .expect("caster");
    let target_index = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("target");

    let plan = engine
        .monster_spell_plan(
            caster_index,
            Some(target_index),
            "stone_gaze",
            MonsterAbilityTargetPolicy::NearestHostile,
        )
        .expect("monster plan should bypass player spellbook gates");
    let mut events = Vec::new();
    engine
        .execute_actor_spell_effect(caster_index, &plan, &mut events)
        .expect("monster status should execute");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            effect_id,
            source_kind,
            source_id,
            kind,
            tags,
            ..
        } if actor_id == "player"
            && effect_id == "stone_gaze"
            && source_kind == "spell"
            && source_id == "stone_gaze"
            && kind == "control_status"
            && tags.iter().any(|tag| tag == "stun")
    )));
    assert!(
        engine.world.actors[target_index]
            .active_effects
            .iter()
            .any(|effect| effect.source.id == "stone_gaze" && effect.suppresses_action)
    );
}

#[test]
fn turn_undead_commits_each_contact_immediately_before_that_targets_effect() {
    let (catalog, profile, mut template, seed) =
        crate::engine::setup::test_parts("remaining_spell_effect_families");
    let level = template
        .realms
        .get_mut("realm_0")
        .expect("test realm")
        .levels
        .get_mut("room_0")
        .expect("test level");
    level.width = 7;
    level.height = 5;
    level.cells = ["#######", "#D....#", "#....##", "#.....#", "#######"]
        .into_iter()
        .map(|row| {
            row.chars()
                .map(|glyph| {
                    vec![Some(
                        match glyph {
                            '#' => "stone_wall",
                            'D' => "bronze_door",
                            '.' => "flagstone",
                            _ => unreachable!("test glyph"),
                        }
                        .to_string(),
                    )]
                })
                .collect()
        })
        .collect();
    let mut engine = crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed);
    let caster_index = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("caster");
    {
        let world = &mut engine.world;
        world.actors.retain(|actor| {
            matches!(
                actor.id.as_str(),
                "player" | "mobile_undead" | "cornered_undead" | "foreign_demon"
            )
        });
        world.actors.sort_by_key(|actor| match actor.id.as_str() {
            "player" => 0,
            "mobile_undead" => 1,
            "cornered_undead" => 2,
            "foreign_demon" => 3,
            _ => unreachable!("retained actor has a known order"),
        });
        let caster = world
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("retained caster");
        caster.location.position.x = 1;
        caster.location.position.y = 2;
        caster.home_location.position = caster.location.position;
        for (target_id, x, y) in [
            ("mobile_undead", 3, 1),
            ("cornered_undead", 2, 3),
            ("foreign_demon", 4, 2),
        ] {
            let target = world
                .actors
                .iter_mut()
                .find(|actor| actor.id == target_id)
                .unwrap_or_else(|| panic!("missing {target_id}"));
            target.kind = ActorKind::Npc;
            target.creature_traits = vec![CreatureTrait::Undead];
            target.social.alignment_source = SocialAlignmentSource::Inherent {
                alignment: CharacterAlignment::Lawful,
            };
            target.social.nature = SocialNature::Human;
            target.social.behavior = SocialBehavior::Civilian;
            target.social.owner_relation = SocialOwnerRelation::None;
            target.npc = Some(NpcState {
                follow_cadence_units: 1,
                interactions: Vec::new(),
                following_character_id: None,
            });
            target.location.position.x = x;
            target.location.position.y = y;
            target.home_location = target.location.clone();
        }
    }

    let plan = engine
        .shared_player_spell_plan(caster_index, "turn_undead")
        .expect("turn-undead plan");
    let mut events = Vec::new();
    let execution = engine
        .execute_actor_spell_effect(caster_index, &plan, &mut events)
        .expect("turn undead should execute");

    assert_eq!(execution.outcome, SpellEffectOutcome::Applied);
    assert_eq!(execution.hostile_spell_outcomes.len(), 3);
    let grudge_index = |target_id: &str| {
        events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::NpcGrudgeEstablished { npc_actor_id, .. }
                        if npc_actor_id == target_id
                )
            })
            .unwrap_or_else(|| panic!("missing grudge for {target_id}"))
    };
    let first_effect_index = |target_id: &str| {
        events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::AutomaticActorDecision { actor_id, .. }
                        if actor_id == target_id
                )
            })
            .unwrap_or_else(|| panic!("missing turn effect for {target_id}"))
    };

    let first_grudge = grudge_index("cornered_undead");
    let first_effect = first_effect_index("cornered_undead");
    let blocked_grudge = grudge_index("foreign_demon");
    let blocked_effect = first_effect_index("foreign_demon");
    let second_grudge = grudge_index("mobile_undead");
    let second_effect = first_effect_index("mobile_undead");
    let resolved_index = events
        .iter()
        .position(|event| matches!(event, Event::TurnUndeadResolved { .. }))
        .expect("turn-undead summary");
    assert!(
        first_grudge < first_effect
            && first_effect < blocked_grudge
            && blocked_grudge < blocked_effect
            && blocked_effect < second_grudge
            && second_grudge < second_effect,
        "contacts must interleave with each target's effect instead of being front-loaded"
    );
    assert_eq!(first_grudge + 1, first_effect);
    assert_eq!(second_grudge + 1, second_effect);
    assert_eq!(blocked_grudge + 1, blocked_effect);
    assert!(matches!(
        &events[blocked_effect],
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Blocked,
            },
            ..
        } if actor_id == "foreign_demon"
    ));
    assert!(matches!(
        &events[resolved_index],
        Event::TurnUndeadResolved {
            blocked_actor_ids,
            ..
        } if blocked_actor_ids == &["foreign_demon".to_string()]
    ));

    for (receipt, target_id) in execution.hostile_spell_outcomes.iter().zip([
        "cornered_undead",
        "foreign_demon",
        "mobile_undead",
    ]) {
        assert_eq!(receipt.target_actor_id, target_id);
        assert_eq!(receipt.reach, HostileSpellReach::TurnUndeadVisibility);
        assert_eq!(receipt.outcome, SpellEffectOutcome::Applied);
        assert!(
            receipt.first_outcome_event_index < receipt.one_past_last_outcome_event_index,
            "moving target should have a non-empty exact outcome range"
        );
        assert!(
            events[receipt.first_outcome_event_index..receipt.one_past_last_outcome_event_index]
                .iter()
                .all(|event| !matches!(
                    event,
                    Event::NpcGrudgeEstablished { .. } | Event::TurnUndeadResolved { .. }
                ))
        );
        assert!(matches!(
            &events[receipt.first_outcome_event_index],
            Event::AutomaticActorDecision { actor_id, .. } if actor_id == target_id
        ));
    }
    assert!(
        execution.hostile_spell_outcomes[0].one_past_last_outcome_event_index
            <= execution.hostile_spell_outcomes[1].first_outcome_event_index,
        "per-target outcome ranges must not overlap"
    );
    assert_eq!(
        execution.hostile_spell_outcomes[0].first_outcome_event_index,
        first_effect
    );
    assert_eq!(
        execution.hostile_spell_outcomes[0].one_past_last_outcome_event_index,
        blocked_grudge
    );
    assert_eq!(
        execution.hostile_spell_outcomes[1].first_outcome_event_index,
        blocked_effect
    );
    assert_eq!(
        execution.hostile_spell_outcomes[1].one_past_last_outcome_event_index,
        second_grudge
    );
    assert_eq!(
        execution.hostile_spell_outcomes[2].first_outcome_event_index,
        second_effect
    );
    assert_eq!(
        execution.hostile_spell_outcomes[2].one_past_last_outcome_event_index,
        resolved_index
    );
}
