use super::*;

#[test]
fn trace_v2_remaining_spell_effect_families_exposes_all_dy_routes() {
    let trace = run_trace_v2("remaining_spell_effect_families.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );

    let mut cast_spell_ids = trace
        .steps
        .iter()
        .filter_map(|step| match &step.command.intent {
            tme_rules::PlayerIntentPayloadV1::CastSpell { spell_id, .. } => Some(spell_id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    cast_spell_ids.sort();
    cast_spell_ids.dedup();
    assert_eq!(
        cast_spell_ids,
        vec![
            "banish",
            "breathe_water",
            "call_demon",
            "death",
            "feather_fall",
            "hide_door",
            "hide_in_shadows",
            "night_vision",
            "raise_dead",
            "sense_secret",
            "shadow_cloud",
            "speed",
            "turn_undead",
        ]
    );

    let events = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .collect::<Vec<_>>();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "remaining-family gallery should not emit SpellCastStubbed"
    );
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ActorDefeated { actor_id, .. } if actor_id == "fallen_ally"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::RaiseDeadEvaluated { spell_id, .. } if spell_id == "raise_dead"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::TileEffectApplied { effect_id, .. }
            if effect_id == "shadow_cloud"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ActorSummoned { actor_id, .. }
            if actor_id == "summon:call_demon:1:bound_demon"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::BanishEvaluated {
            target_id,
            owned_by_caster: false,
            ..
        } if target_id == "foreign_demon"
    )));
    assert!(
        !events.iter().any(|event| matches!(
            event,
            tme_rules::Event::ActorBanished { actor_id, .. }
                if actor_id == "summon:call_demon:1:bound_demon"
        )),
        "the caster's owned summon remains an invalid hostile target"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::TransitionConcealed { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::TransitionConcealmentRemoved { .. }))
    );
    for effect_id in ["night_vision", "feather_fall", "speed", "breathe_water"] {
        assert!(events.iter().any(|event| matches!(
            event,
            tme_rules::Event::EffectApplied { effect_id: applied, .. }
                if applied == effect_id
        )));
    }
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::TurnUndeadResolved { moved_actor_ids, .. }
            if moved_actor_ids == &vec!["mobile_undead".to_string()]
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ActorHidden { actor_id, .. } if actor_id == "player"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::HideBroken { actor_id, .. } if actor_id == "player"
    )));

    let final_actors = &trace.r#final.final_debug_snapshot.actors;
    assert!(
        final_actors
            .iter()
            .any(|actor| actor.id == "summon:call_demon:1:bound_demon"),
        "owned summon must survive because no hostile command may target it"
    );
    let fallen = final_actors
        .iter()
        .find(|actor| actor.id == "fallen_ally")
        .expect("fallen ally remains in the final debug snapshot");
    assert!(matches!(
        &fallen.life_state,
        tme_rules::ActorLifeStateViewV1::Dead
    ));
    let player = final_actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player remains in final state");
    assert_eq!(player.location.position, tme_rules::Coord { x: 3, y: 1 });
}

#[test]
fn trace_v2_magic_profession_gallery_exposes_dx_contract() {
    let trace = run_trace_v2("magic_profession_gallery.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );

    let initial_player = trace
        .header
        .initial_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player0")
        .expect("initial player");
    let initial_character = initial_player.character.as_ref().expect("character sheet");
    assert_eq!(initial_character.resources.mp, 40);
    assert_eq!(initial_player.carried.gold.sack, 20);
    assert!(
        !initial_character
            .known_spells
            .iter()
            .any(|spell| spell.spell_id == "shadow_sting"),
        "fixture should learn shadow_sting during the script"
    );

    let mut saw_self_protection_cast = false;
    let mut saw_coordinate_cast = false;
    let mut saw_actor_damage_cast = false;
    let mut saw_actor_curse_cast = false;
    let mut saw_area_overlay_cast = false;
    let mut saw_self_control_cast = false;
    let mut saw_learn_command = false;
    let mut saw_hide_command = false;
    let mut saw_move_path_command = false;

    for step in &trace.steps {
        match &step.command.intent {
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::SelfTarget),
                ..
            } if spell_id == "toxin_ward" => saw_self_protection_cast = true,
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Coordinate { position }),
                ..
            } if spell_id == "shadow_veil"
                && position.level == "room_0"
                && position.position == (tme_rules::Coord { x: 1, y: 2 }) =>
            {
                saw_coordinate_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Actor { actor_id }),
                ..
            } if spell_id == "shadow_sting" && actor_id == "target_dummy" => {
                saw_actor_damage_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Actor { actor_id }),
                ..
            } if spell_id == "dimming_hex" && actor_id == "target_dummy" => {
                saw_actor_curse_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Area { center }),
                ..
            } if spell_id == "web_field"
                && center.level == "room_0"
                && center.position == (tme_rules::Coord { x: 2, y: 2 }) =>
            {
                saw_area_overlay_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::SelfTarget),
                ..
            } if spell_id == "self_hold" => saw_self_control_cast = true,
            tme_rules::PlayerIntentPayloadV1::LearnSpell { spell_id }
                if spell_id == "shadow_sting" =>
            {
                saw_learn_command = true;
            }
            tme_rules::PlayerIntentPayloadV1::Hide => saw_hide_command = true,
            tme_rules::PlayerIntentPayloadV1::MovePath { path }
                if path == &[tme_rules::Direction::East] =>
            {
                saw_move_path_command = true;
            }
            _ => {}
        }
    }

    assert!(
        saw_self_protection_cast,
        "trace should expose self protection cast"
    );
    assert!(saw_coordinate_cast, "trace should expose coordinate cast");
    assert!(
        saw_actor_damage_cast,
        "trace should expose actor-targeted damage cast"
    );
    assert!(
        saw_actor_curse_cast,
        "trace should expose actor-targeted Curse cast"
    );
    assert!(
        saw_area_overlay_cast,
        "trace should expose area-targeted overlay cast"
    );
    assert!(
        saw_self_control_cast,
        "trace should expose self control cast"
    );
    assert!(saw_learn_command, "trace should expose learn_spell command");
    assert!(saw_hide_command, "trace should expose hide command");
    assert!(
        saw_move_path_command,
        "trace should expose move_path command"
    );

    let mut saw_learning_book_retained = false;
    let mut saw_spell_learned = false;
    let mut saw_gold_spent = false;
    let mut saw_spell_damage = false;
    let mut saw_magic_practice_receipt = false;
    let mut saw_skill_practice = false;
    let mut saw_effect_applied = false;
    let mut saw_curse_applied = false;
    let mut saw_control_applied = false;
    let mut saw_effect_ticked = false;
    let mut saw_effect_expired = false;
    let mut saw_spell_save = false;
    let mut saw_action_suppressed = false;
    let mut saw_tile_effect_applied = false;
    let mut saw_tile_effect_expired = false;
    let mut saw_actor_hidden = false;
    let mut saw_hide_broken = false;
    let mut saw_monster_intent = false;
    let mut saw_web_movement_cost = false;

    for event in trace.steps.iter().flat_map(|step| step.events.iter()) {
        match event {
            tme_rules::Event::SpellLearned {
                actor_id,
                spell_id,
                lane,
                gold_cost,
                spell_book_item_instance_id,
                spell_book_item_definition_id,
                ..
            } if actor_id == "player0"
                && spell_id == "shadow_sting"
                && lane == "thief_magic"
                && *gold_cost == 10
                && spell_book_item_instance_id == "spell_book"
                && spell_book_item_definition_id == "spell_book" =>
            {
                saw_spell_learned = true;
                saw_learning_book_retained = true;
            }
            tme_rules::Event::GoldChanged {
                actor_id,
                amount,
                new_total,
                ..
            } if actor_id == "player0" && *amount == -10 && *new_total == 10 => {
                saw_gold_spent = true;
            }
            tme_rules::Event::SpellDamaged {
                caster_id,
                spell_id,
                target_id,
                damage_kind,
                damage,
                ..
            } if caster_id == "player0"
                && spell_id == "shadow_sting"
                && target_id == "target_dummy"
                && damage_kind.as_deref() == Some("shadow")
                && *damage == 3 =>
            {
                saw_spell_damage = true;
            }
            tme_rules::Event::SkillPracticeAwarded {
                actor_id,
                track_id,
                raw_amount,
                learning_rate,
                credited_amount,
                ..
            } if actor_id == "player0"
                && track_id == "thief_magic"
                && *raw_amount == 3
                && *learning_rate == 1
                && *credited_amount == 3 =>
            {
                saw_skill_practice = true;
            }
            tme_rules::Event::MagicPracticeEvaluated {
                actor_id,
                current_class_id,
                spell_id,
                track_id,
                mp_cost,
                primary_attribute: Some(tme_rules::MagicPrimaryAttribute::Intelligence),
                primary_attribute_value: Some(11),
                base_raw_points,
                primary_attribute_bonus_raw_points,
                total_raw_points,
                risk_applied,
                reason,
                ..
            } if actor_id == "player0"
                && current_class_id == "thief"
                && spell_id == "shadow_sting"
                && track_id == "thief_magic"
                && *mp_cost == 2
                && *base_raw_points == 2
                && *primary_attribute_bonus_raw_points == 1
                && *total_raw_points == 3
                && !risk_applied
                && reason == "eligible_successful_cast" =>
            {
                saw_magic_practice_receipt = true;
            }
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "toxin_ward" && kind == "protection" => {
                saw_effect_applied = true;
            }
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "target_dummy" && effect_id == "dimming_hex" && kind == "curse" => {
                saw_curse_applied = true;
            }
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "self_hold" && kind == "control_status" => {
                saw_control_applied = true;
            }
            tme_rules::Event::EffectTicked {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "toxin_ward" && kind == "protection" => {
                saw_effect_ticked = true;
            }
            tme_rules::Event::EffectExpired {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "toxin_ward" && kind == "protection" => {
                saw_effect_expired = true;
            }
            tme_rules::Event::SpellSaveResolved {
                actor_id,
                effect_id,
                resistance_tag,
                ..
            } if actor_id == "player0"
                && effect_id == "venom_bite"
                && resistance_tag == "poison" =>
            {
                saw_spell_save = true;
            }
            tme_rules::Event::ActionSuppressedByStatus {
                actor_id,
                intent,
                effect_id,
                kind,
                ..
            } if actor_id == "player0"
                && intent == "walk east"
                && effect_id == "self_hold"
                && kind == "control_status" =>
            {
                saw_action_suppressed = true;
            }
            tme_rules::Event::TileEffectApplied {
                effect_id,
                location,
                sight,
                move_cost,
                ..
            } if effect_id == "web_field"
                && location.position == (tme_rules::Coord { x: 2, y: 2 })
                && sight.as_deref() == Some("obscured")
                && *move_cost == Some(2) =>
            {
                saw_tile_effect_applied = true;
            }
            tme_rules::Event::TileEffectExpired { effect_id, .. } if effect_id == "web_field" => {
                saw_tile_effect_expired = true;
            }
            tme_rules::Event::ActorHidden {
                actor_id,
                effect_id,
                ..
            } if actor_id == "player0" && effect_id == "hidden" => {
                saw_actor_hidden = true;
            }
            tme_rules::Event::HideBroken {
                actor_id,
                effect_id,
                reason,
                ..
            } if actor_id == "player0" && effect_id == "hidden" && reason == "active_item_move" => {
                saw_hide_broken = true;
            }
            tme_rules::Event::AutomaticActorDecision {
                actor_id,
                decision: tme_rules::AutomaticActorDecisionV1::UseAbility { spell_name, .. },
                ..
            } if actor_id == "viperling" && spell_name == "Venom Bite" => {
                saw_monster_intent = true;
            }
            tme_rules::Event::MovementCostPaid {
                actor_id,
                terrain,
                cost,
                destination,
                ..
            } if actor_id == "player0"
                && terrain.contains("web_field")
                && *cost == 2
                && destination.position == (tme_rules::Coord { x: 2, y: 2 }) =>
            {
                saw_web_movement_cost = true;
            }
            _ => {}
        }
    }

    assert!(
        !trace
            .steps
            .iter()
            .flat_map(|step| step.events.iter())
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "gallery should not emit SpellCastStubbed"
    );
    assert!(
        saw_learning_book_retained,
        "gallery should retain and identify the exact Spell Book"
    );
    assert!(saw_spell_learned, "gallery should emit SpellLearned");
    assert!(saw_gold_spent, "gallery should emit GoldChanged");
    assert!(
        saw_spell_damage,
        "gallery should emit targeted spell damage"
    );
    assert!(
        saw_magic_practice_receipt,
        "gallery should emit the exact magic practice calculation receipt"
    );
    assert!(
        saw_skill_practice,
        "gallery should emit casting skill practice"
    );
    assert!(saw_effect_applied, "gallery should apply protection effect");
    assert!(
        saw_curse_applied,
        "gallery should apply original Curse effect"
    );
    assert!(saw_control_applied, "gallery should apply control effect");
    assert!(saw_effect_ticked, "gallery should tick active effects");
    assert!(saw_effect_expired, "gallery should expire active effects");
    assert!(saw_spell_save, "gallery should emit a spell-save receipt");
    assert!(saw_action_suppressed, "gallery should suppress an action");
    assert!(saw_tile_effect_applied, "gallery should apply tile overlay");
    assert!(
        saw_tile_effect_expired,
        "gallery should expire tile overlay"
    );
    assert!(saw_actor_hidden, "gallery should emit ActorHidden");
    assert!(saw_hide_broken, "gallery should emit HideBroken");
    assert!(
        saw_monster_intent,
        "gallery should emit monster ability intent"
    );
    assert!(
        saw_web_movement_cost,
        "gallery should charge web overlay movement cost"
    );

    let final_player = trace
        .r#final
        .final_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player0")
        .expect("final player");
    let final_character = final_player.character.as_ref().expect("character sheet");
    assert!(
        final_character.resources.mp < initial_character.resources.mp,
        "spell casting should spend MP across the scenario"
    );
    assert_eq!(final_player.carried.gold.sack, 10);
    assert!(
        final_character
            .known_spells
            .iter()
            .any(|spell| spell.spell_id == "shadow_sting" && spell.lane == "thief_magic"),
        "learned spell should appear in final snapshot"
    );
}
