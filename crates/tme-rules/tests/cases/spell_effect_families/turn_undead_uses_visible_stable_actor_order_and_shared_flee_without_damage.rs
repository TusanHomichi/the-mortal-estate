use super::*;

#[test]
fn turn_undead_uses_visible_stable_actor_order_and_shared_flee_without_damage() {
    let lane = "thaumaturge_magic";
    let turn = spell(
        "turn_undead",
        lane,
        json!({"family": "turn_undead", "turn_undead": {"eligible_trait": "undead"}}),
        json!({"kind": "none"}),
        "not_applicable",
    );
    let mut engine = family_engine("thaumaturge", lane, vec![turn], 7, |parts| {
        parts.template_levels_source_mut()["room_0"]["width"] = json!(7);
        parts.template_levels_source_mut()["room_0"]["cells"] =
            layered_cells(&["#######", "#.....#", "#######"]);
        parts.actors_mut()[0]["location"]["position"] = json!({"x": 1, "y": 1});
        let mut actors = parts.actors_mut().as_array().expect("actors").clone();
        actors[1]["id"] = json!("z_undead");
        actors[1]["location"]["position"] = json!({"x": 3, "y": 1});
        let mut second = actors[1].clone();
        second["id"] = json!("a_undead");
        second["location"]["position"] = json!({"x": 4, "y": 1});
        actors.push(second);
        let mut living = actors[1].clone();
        living["id"] = json!("living");
        living["actor_definition_id"] = json!("actor/test/living");
        living["location"]["position"] = json!({"x": 2, "y": 1});
        actors.push(living);
        *parts.actors_mut() = Value::Array(actors);
        let mut living_definition = parts.actor_definition_mut(1).clone();
        living_definition["id"] = json!("actor/test/living");
        living_definition["name"] = json!("Living");
        living_definition["creature_traits"] = json!([]);
        parts.actor_definition_mut(1)["name"] = json!("Undead");
        parts.actor_definition_mut(1)["creature_traits"] = json!(["undead"]);
        parts.actor_definition_mut(1)["stats"]["hp"] = json!(12);
        parts.push_selected(
            "actor_definitions",
            "actor/test/living/turn_undead",
            living_definition,
        );
    });
    let hp_before = engine
        .world()
        .actors
        .iter()
        .filter(|actor| actor.creature_traits.contains(&CreatureTrait::Undead))
        .map(|actor| (actor.id.clone(), actor.hp))
        .collect::<BTreeMap<_, _>>();
    let events = cast(&mut engine, "turn_undead", None);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TurnUndeadResolved {
            considered_actor_ids,
            moved_actor_ids,
            blocked_actor_ids,
            ..
        } if considered_actor_ids == &vec!["a_undead".to_string(), "z_undead".to_string()]
            && moved_actor_ids == considered_actor_ids
            && blocked_actor_ids.is_empty()
    )));
    assert!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Event::AutomaticActorDecision {
                    decision: AutomaticActorDecisionV1::Move {
                        purpose: AutomaticMovementPurposeV1::Turned,
                        ..
                    },
                    ..
                }
            ))
            .count()
            >= 2
    );
    for actor in engine
        .world()
        .actors
        .iter()
        .filter(|actor| actor.creature_traits.contains(&CreatureTrait::Undead))
    {
        assert_eq!(Some(&actor.hp), hp_before.get(&actor.id));
    }
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated { .. }
            | Event::SpellDamaged { .. }
            | Event::DefeatRewardEvaluated { .. }
    )));
}
