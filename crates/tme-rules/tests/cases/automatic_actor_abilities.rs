use crate::ai_support::{
    ContentParts, automatic_actor, decision, engine_from_value, push_automatic_actor,
    set_actor_hidden, unrestricted, wait,
};
use tme_rules::model::{MonsterAbilityKind, MonsterAbilityTargetPolicy};
use tme_rules::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Coord, Direction,
    Engine, Event, LogicalTime, PlayerIntent, SpellTarget, WorldPosition,
};

fn ability_value(actor_id: &str) -> ContentParts {
    let mut parts = ContentParts::tracked(
        "monster_spellcasting_special_attacks",
        "profile/monster_spellcasting_special_attacks",
    );
    let actors = parts.world_seed["actors"]
        .as_array()
        .expect("actors array")
        .iter()
        .filter(|actor| actor["id"] == "player" || actor["id"] == actor_id)
        .cloned()
        .collect();
    *parts.actors_mut() = serde_json::Value::Array(actors);
    parts
}
#[test]
fn ready_ability_has_priority_over_movement_and_physical_attack() {
    let mut value = ability_value("ember_imp");
    let player_position = value.world_seed["actors"][0]["location"]["position"].clone();
    value.actors_mut()[1]["location"]["position"] = player_position;
    let mut engine = engine_from_value(value);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "ember_imp"),
        AutomaticActorDecisionV1::UseAbility {
            ability_id,
            spell_id,
            target_id: Some(target_id),
            ..
        } if ability_id == "ember_spit" && spell_id == "ember_spit" && target_id == "player"
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            caster_id,
            target_id,
            spell_id,
            ..
        } if caster_id == "ember_imp" && target_id == "player" && spell_id == "ember_spit"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Moved { actor_id, .. } | Event::Attacked { attacker_id: actor_id, .. }
            if actor_id == "ember_imp"
    )));
}

#[test]
fn cooldown_falls_back_to_chase_until_ability_is_ready() {
    let mut engine = engine_from_value(ability_value("ember_imp"));
    let first = wait(&mut engine);
    assert!(matches!(
        decision(&first, "ember_imp"),
        AutomaticActorDecisionV1::UseAbility { .. }
    ));
    let ready_at = engine.world().actors[1].monster_abilities[0].ready_at;
    assert!(ready_at > LogicalTime::ZERO);

    let second = wait(&mut engine);
    assert_eq!(
        decision(&second, "ember_imp"),
        &AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    );

    let third = wait(&mut engine);
    assert!(matches!(
        decision(&third, "ember_imp"),
        AutomaticActorDecisionV1::UseAbility { .. }
    ));
}

#[test]
fn line_of_sight_memory_blocks_ability_against_fresh_hidden_target() {
    let mut engine = engine_from_value(ability_value("ember_imp"));
    set_actor_hidden(&mut engine, "player", true);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "ember_imp"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged { caster_id, .. } if caster_id == "ember_imp"
    )));
}

#[test]
fn hold_ground_uses_ready_ability_before_holding() {
    let mut value = ability_value("ember_imp");
    value.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    let mut engine = engine_from_value(value);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "ember_imp"),
        AutomaticActorDecisionV1::UseAbility { .. }
    ));
}

#[test]
fn illegal_self_target_ability_is_rejected_before_runtime() {
    let mut value = ability_value("ember_imp");
    value.actor_definition_mut(1)["monster_abilities"][0]["target_policy"] =
        serde_json::json!("self");
    let error = value
        .validated_seed()
        .expect_err("direct damage resolved to self must be rejected");
    assert!(
        error
            .to_string()
            .contains("unsupported monster effect/target combination"),
        "{error}"
    );
}

#[test]
fn resisted_ability_still_enters_cooldown() {
    let mut engine = engine_from_value(ability_value("viperling"));
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "viperling"),
        AutomaticActorDecisionV1::UseAbility { spell_id, .. }
            if spell_id == "venom_bite"
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            actor_id,
            effect_id,
            resistance_tag,
            ..
        } if actor_id == "player" && effect_id == "venom_bite" && resistance_tag == "poison"
    )));
    assert!(engine.world().actors[1].monster_abilities[0].ready_at > LogicalTime::ZERO);
}

#[test]
fn pack_forager_and_web_ambush_do_not_inherit_ability_priority() {
    let mut pack_value = ability_value("ember_imp");
    pack_value.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("pack_forager");
    let mut pack = engine_from_value(pack_value);
    assert!(matches!(
        decision(&wait(&mut pack), "ember_imp"),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Chase,
            ..
        }
    ));

    let mut web_value = ability_value("ember_imp");
    web_value.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 3, "y": 2});
    web_value.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("web_ambush");
    let mut web = engine_from_value(web_value);
    assert!(matches!(
        decision(&wait(&mut web), "ember_imp"),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Chase,
            ..
        }
    ));
}

#[test]
fn nearest_hostile_ability_uses_the_same_selected_target() {
    let mut value = ability_value("ember_imp");
    push_automatic_actor(
        &mut value,
        automatic_actor(
            "guardian",
            "lawful",
            Coord { x: 4, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    );
    let mut engine = engine_from_value(value);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "ember_imp"),
        AutomaticActorDecisionV1::UseAbility {
            target_id: Some(target_id),
            ..
        } if target_id == "guardian"
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            caster_id,
            target_id,
            ..
        } if caster_id == "ember_imp" && target_id == "guardian"
    )));
}

fn monster_ability_runtime_engine() -> Engine {
    let mut parts = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    );

    let monster_index = parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .iter()
        .position(|actor| actor["id"] == "ash_imp")
        .expect("tracked monster");
    parts.actors_mut()[monster_index]["id"] = serde_json::json!("ember_imp");
    let monster_definition = parts.actor_definition_mut(monster_index);
    monster_definition["name"] = serde_json::json!("Ember Imp");
    monster_definition["monster_abilities"] = serde_json::json!([
        {
            "id": "ember",
            "kind": "special_attack",
            "spell_id": "ember_spit",
            "cooldown_rounds": 2,
            "target_policy": "nearest_hostile"
        }
    ]);

    parts.summon_actor_definition_mut(0)["monster_abilities"] = serde_json::json!([
        {
            "id": "echo_mend",
            "kind": "spell",
            "spell_id": "ember_mend",
            "cooldown_rounds": 3,
            "target_policy": "self"
        }
    ]);
    parts.profile_value_mut()["spells"]
        .as_array_mut()
        .expect("selected spells")
        .push(serde_json::Value::String(
            "spell/ember_spit/monster_spellcasting_special_attacks".to_string(),
        ));
    parts.push_selected(
        "spells",
        "spell/ember_mend/automatic_actor_abilities",
        serde_json::json!({
            "social": {"hostile_act": false, "town_law": "permitted"},
            "id": "ember_mend",
            "name": "Ember Mend",
            "status": "draft",
            "lane": "monster_special",
            "effect": {"family": "healing", "potency": 2},
            "target": {"kind": "actor", "range": 4, "requires_visible": true},
            "casting": {"method": "direct", "cast_class": "not_applicable"}
        }),
    );
    parts
        .engine(7)
        .expect("monster ability engine should start")
}
#[test]
fn monster_spell_plan_maps_runtime_state_from_actor_and_summon_abilities() {
    let mut engine = monster_ability_runtime_engine();

    let monster = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "ember_imp")
        .expect("monster actor");
    assert!(monster.character.is_none());
    assert_eq!(monster.monster_abilities.len(), 1);
    let ability = &monster.monster_abilities[0];
    assert_eq!(ability.id, "ember");
    assert_eq!(ability.kind, MonsterAbilityKind::SpecialAttack);
    assert_eq!(ability.spell_id, "ember_spit");
    assert_eq!(ability.cooldown_rounds, 2);
    assert_eq!(
        ability.target_policy,
        MonsterAbilityTargetPolicy::NearestHostile
    );
    assert_eq!(ability.ready_at, tme_rules::LogicalTime::ZERO);

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
    let summoned = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id.starts_with("summon:call_echo:"))
        .expect("summoned actor");
    assert_eq!(summoned.monster_abilities.len(), 1);
    let summoned_ability = &summoned.monster_abilities[0];
    assert_eq!(summoned_ability.id, "echo_mend");
    assert_eq!(summoned_ability.kind, MonsterAbilityKind::Spell);
    assert_eq!(summoned_ability.spell_id, "ember_mend");
    assert_eq!(summoned_ability.cooldown_rounds, 3);
    assert_eq!(
        summoned_ability.target_policy,
        MonsterAbilityTargetPolicy::SelfTarget
    );
    assert_eq!(summoned_ability.ready_at, tme_rules::LogicalTime::ZERO);
}
