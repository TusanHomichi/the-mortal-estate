use super::*;

#[test]
fn lethal_fire_spell_passes_fire_credit_and_suppresses_corpse_creation() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["spark"], 10, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(3);
        parts.selected_by_runtime_id_mut("spells", "spark")["effect"]["damage_kind"] =
            serde_json::json!("fire");
    });

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("lethal fire spell should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated {
            actor_id,
            cause: tme_rules::DeathCause::Fire,
            credited_actor_id: Some(credited_actor_id),
            ..
        } if actor_id == "target" && credited_actor_id == "player"
    )));
    assert!(engine.world().corpses.is_empty());
    assert_eq!(engine.world().next_corpse_sequence, 1);
    assert!(matches!(
        engine
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == "target")
            .unwrap()
            .life_state,
        tme_rules::ActorLifeState::Dead
    ));
}
