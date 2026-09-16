//! Issue #76: an admitted self-targeted direct-damage spell consumed casting
//! resources and then reported `SpellCastStubbed`.
//!
//! The effect families execute against an actor target. A `self` target names
//! the caster but resolves to no actor target, so the direct-damage family could
//! only stub after the MP and action were already spent. Content now refuses the
//! pair, and these cases prove both halves: unsupported content cannot be
//! admitted, and every legitimate neighbouring combination still works.

use super::*;

fn arcane_damage() -> Value {
    json!({
        "family": "direct_damage",
        "potency": 3,
        "damage_kind": "arcane",
        "resistance": {
            "role": "incoming",
            "tag": "arcane",
            "mitigation": {"mode": "half_damage", "rounding": "down", "minimum_damage": 1}
        }
    })
}

fn resource_snapshot(engine: &Engine) -> (i32, i32, LogicalTime) {
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    (player.mp, player.stamina, player.timing.ready_at)
}

#[test]
fn self_targeted_direct_damage_is_refused_by_content_validation() {
    let mut parts = ContentParts::tracked("spell_effects", "profile/spell_effects");
    parts.push_selected(
        "spells",
        "spell/self_immolation/effect_family_test",
        spell(
            "self_immolation",
            "wizard_magic",
            arcane_damage(),
            json!({"kind": "self"}),
            "self",
        ),
    );
    let error = parts
        .validated_seed()
        .expect_err("a self-targeted direct-damage spell must not be admitted");
    assert!(
        error
            .to_string()
            .contains("target.kind cannot be self for direct_damage spells"),
        "{error}"
    );
}

#[test]
fn self_targeted_healing_buffs_and_conditions_remain_valid_and_execute() {
    let lane = "wizard_magic";
    let spells = vec![
        spell(
            "mend",
            lane,
            json!({"family": "healing", "potency": 4}),
            json!({"kind": "self"}),
            "self",
        ),
        spell(
            "strength",
            lane,
            json!({
                "family": "attribute_buff", "status_kind": "strength", "potency": 2,
                "stacking": "replace_same_kind", "duration": {"policy": "rounds", "rounds": 2}
            }),
            json!({"kind": "self"}),
            "self",
        ),
        spell(
            "self_poison",
            lane,
            json!({
                "family": "poison", "status_kind": "poison", "potency": 2,
                "start_delay_rounds": 1,
                "resistance": {"role": "incoming", "tag": "poison", "mitigation": {"mode": "negate"}},
                "duration": {"policy": "rounds", "rounds": 3}
            }),
            json!({"kind": "self"}),
            "self",
        ),
    ];
    let mut engine = family_engine("wizard", lane, spells, 7, |_| {});
    engine.world_mut().actors[0].hp = 4;

    for (spell_id, applied_tag) in [
        ("mend", None),
        ("strength", Some("strength")),
        ("self_poison", Some("poison")),
    ] {
        let before_mp = engine.world().actors[0].mp;
        let before_effects = engine.world().actors[0].active_effects.len();
        let events = cast(&mut engine, spell_id, Some(SpellTarget::SelfTarget));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::SpellCastStubbed { .. })),
            "{spell_id} must execute rather than stub"
        );
        assert!(
            engine.world().actors[0].mp < before_mp,
            "{spell_id} consumes its authored MP"
        );
        if let Some(tag) = applied_tag {
            let effects = &engine.world().actors[0].active_effects;
            assert!(
                effects
                    .iter()
                    .any(|effect| effect.tags.contains(&tag.to_string())),
                "{spell_id} applies the {tag} effect to the caster: {effects:?}"
            );
            assert!(
                effects.len() >= before_effects,
                "{spell_id} adds its own effect"
            );
        } else {
            assert_eq!(
                engine.world().actors[0].hp,
                8,
                "self healing restores the caster"
            );
        }
    }
}
#[test]
fn hostile_actor_targeted_direct_damage_still_executes_and_consumes_resources() {
    let lane = "wizard_magic";
    let spells = vec![spell(
        "spark",
        lane,
        arcane_damage(),
        json!({"kind": "actor", "range": 3, "requires_visible": true}),
        "character",
    )];
    let mut engine = family_engine("wizard", lane, spells, 7, |parts| {
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] = json!(0);
    });
    let target_hp = engine
        .world()
        .actor(&tme_rules::ActorId::from("target"))
        .expect("target")
        .hp;
    let before = resource_snapshot(&engine);

    let events = cast(
        &mut engine,
        "spark",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );

    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::SpellDamaged { .. })),
        "the actor-targeted direct-damage spell damages its target"
    );
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("target"))
            .expect("target")
            .hp
            < target_hp
    );
    let after = resource_snapshot(&engine);
    assert!(
        after.0 < before.0,
        "the cast spends MP from the caster: {before:?} -> {after:?}"
    );
    assert_eq!(after.1, before.1, "the cast does not invent a stamina cost");
    assert!(
        after.2 > before.2,
        "the cast schedules the caster's next action"
    );
}
