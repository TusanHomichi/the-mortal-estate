use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorAiBehavior, ActorAwarenessDef, ActorDefinitionDef, CatalogV6, CharacterAlignment,
    PhysicalAttackMode, SocialAlignmentSourceDef, SocialBehaviorDef, SocialNatureDef,
    SocialOwnerRelationDef, ValidatedWorldSeed, WorldSeedDef,
};

fn first_room_value() -> ContentParts {
    ContentParts::tracked("first_room", "profile/first_room")
}

fn parse(value: &ContentParts) -> Result<ValidatedWorldSeed, String> {
    value.validated_seed()
}

fn assert_error(value: &ContentParts, fragment: &str) {
    let error = parse(value).expect_err("mutated fixture should fail");
    assert!(
        error.contains(fragment),
        "expected {fragment:?} in {error:?}"
    );
}

fn actor_definition<'a>(
    catalog: &'a CatalogV6,
    seed: &WorldSeedDef,
    actor_index: usize,
) -> &'a ActorDefinitionDef {
    let definition_id = &seed.actors[actor_index].actor_definition_id;
    catalog
        .actor_definitions
        .values()
        .find(|definition| &definition.id == definition_id)
        .expect("selected actor definition")
}

#[test]
fn content_parts_expose_typed_actor_ai_and_social_profiles() {
    let parts = first_room_value();
    parse(&parts).expect("explicit content parts validate");
    let (catalog, _, _, seed) = parts.decode().expect("content parts decode");
    let player = actor_definition(&catalog, &seed, 0);
    assert_eq!(
        player.social.alignment_source,
        SocialAlignmentSourceDef::Inherent {
            alignment: CharacterAlignment::Lawful,
        }
    );
    assert_eq!(player.social.nature, SocialNatureDef::Human);
    assert_eq!(player.social.behavior, SocialBehaviorDef::Adventurer);
    assert_eq!(player.social.owner_relation, SocialOwnerRelationDef::None);
    assert!(player.ai.is_none());

    let monsters = seed
        .actors
        .iter()
        .enumerate()
        .map(|(index, _)| actor_definition(&catalog, &seed, index))
        .filter(|definition| definition.kind == tme_rules::ActorKind::Monster)
        .collect::<Vec<_>>();
    assert!(!monsters.is_empty());
    for monster in monsters {
        assert_eq!(
            monster.social.alignment_source,
            SocialAlignmentSourceDef::Inherent {
                alignment: CharacterAlignment::Chaotic,
            }
        );
        assert_eq!(monster.social.nature, SocialNatureDef::Other);
        assert_eq!(
            monster.social.behavior,
            SocialBehaviorDef::AlignmentCreature
        );
        assert_eq!(monster.social.owner_relation, SocialOwnerRelationDef::None);
        let ai = monster.ai.as_ref().expect("monster AI is required");
        assert_eq!(ai.behavior, ActorAiBehavior::SimpleChase);
        assert_eq!(ai.cadence_units, 1);
        assert_eq!(ai.leash_range, 12);
        assert_eq!(ai.awareness, ActorAwarenessDef::Unrestricted {});
        assert_eq!(ai.physical_attack_modes, vec![PhysicalAttackMode::Fight]);
    }
}

#[test]
fn actor_kind_ai_matrix_has_no_default_or_alias() {
    let mut missing = first_room_value();
    missing
        .actor_definition_mut(1)
        .as_object_mut()
        .expect("monster object")
        .remove("ai");
    assert_error(&missing, "missing field `ai`");

    let mut player_ai = first_room_value();
    player_ai.actor_definition_mut(0)["ai"] =
        first_room_value().actor_definition_mut(1)["ai"].clone();
    assert_error(
        &player_ai,
        "actor_definitions[0].ai is forbidden for players",
    );

    let mut alias = first_room_value();
    let legacy_behavior = ["ally", "chase"].join("_");
    alias.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!(legacy_behavior);
    assert_error(&alias, "unknown variant");
}

#[test]
fn actor_social_profile_is_required_strict_and_typed() {
    let mut missing = first_room_value();
    missing
        .actor_definition_mut(1)
        .as_object_mut()
        .expect("monster object")
        .remove("social");
    assert_error(&missing, "missing field `social`");

    let mut unknown_alignment_source = first_room_value();
    unknown_alignment_source.actor_definition_mut(1)["social"]["alignment_source"] =
        serde_json::json!({"kind": "faction", "alignment": "chaotic"});
    assert_error(&unknown_alignment_source, "unknown variant `faction`");

    let mut unknown_nature = first_room_value();
    unknown_nature.actor_definition_mut(1)["social"]["nature"] = serde_json::json!("spectral");
    assert_error(&unknown_nature, "unknown variant `spectral`");
}

#[test]
fn cadence_leash_and_memory_must_be_positive_u32_values() {
    for field in ["cadence_units", "aggro_radius", "leash_range"] {
        let mut zero = first_room_value();
        zero.actor_definition_mut(1)["ai"][field] = serde_json::json!(0);
        assert_error(
            &zero,
            &format!("actor_definitions[1].ai.{field} must be positive"),
        );

        let mut boolean = first_room_value();
        boolean.actor_definition_mut(1)["ai"][field] = serde_json::json!(true);
        assert_error(&boolean, "expected u32");

        let mut overflow = first_room_value();
        overflow.actor_definition_mut(1)["ai"][field] = serde_json::json!(u64::from(u32::MAX) + 1);
        assert_error(&overflow, "expected u32");
    }

    let mut memory = first_room_value();
    memory.actor_definition_mut(1)["ai"]["awareness"] = serde_json::json!({
        "mode": "line_of_sight_memory",
        "memory_opportunities": 0
    });
    assert_error(
        &memory,
        "actor_definitions[1].ai.awareness.memory_opportunities must be positive",
    );
}

#[test]
fn ordered_physical_modes_are_nonempty_unique_and_typed() {
    let mut ordered = first_room_value();
    ordered.actor_definition_mut(1)["ai"]["physical_attack_modes"] =
        serde_json::json!(["jumpkick", "kick", "fight", "poke", "shoot", "throw"]);
    parse(&ordered).expect("all current modes parse");
    let (catalog, _, _, seed) = ordered.decode().expect("ordered content decode");
    assert_eq!(
        actor_definition(&catalog, &seed, 1)
            .ai
            .as_ref()
            .expect("monster AI")
            .physical_attack_modes,
        vec![
            PhysicalAttackMode::Jumpkick,
            PhysicalAttackMode::Kick,
            PhysicalAttackMode::Fight,
            PhysicalAttackMode::Poke,
            PhysicalAttackMode::Shoot,
            PhysicalAttackMode::Throw,
        ]
    );

    let mut empty = first_room_value();
    empty.actor_definition_mut(1)["ai"]["physical_attack_modes"] = serde_json::json!([]);
    assert_error(
        &empty,
        "actor_definitions[1].ai.physical_attack_modes must be non-empty",
    );

    let mut duplicate = first_room_value();
    duplicate.actor_definition_mut(1)["ai"]["physical_attack_modes"] =
        serde_json::json!(["fight", "fight"]);
    assert_error(&duplicate, "physical_attack_modes[1] duplicates");

    let mut unknown = first_room_value();
    unknown.actor_definition_mut(1)["ai"]["physical_attack_modes"] = serde_json::json!(["bite"]);
    assert_error(&unknown, "unknown variant `bite`");
}

#[test]
fn strict_ai_shapes_reject_unknown_and_removed_fields() {
    let mut unknown_ai = first_room_value();
    unknown_ai.actor_definition_mut(1)["ai"]["legacy"] = serde_json::json!(true);
    assert_error(&unknown_ai, "unknown field `legacy`");

    let mut unknown_awareness = first_room_value();
    unknown_awareness.actor_definition_mut(1)["ai"]["awareness"]["memory_opportunities"] =
        serde_json::json!(2);
    assert_error(&unknown_awareness, "unknown field `memory_opportunities`");

    let mut sibling_leash = first_room_value();
    sibling_leash.actor_definition_mut(1)["leash_range"] = serde_json::json!(12);
    assert_error(&sibling_leash, "unknown field `leash_range`");

    let mut top_level_perception = first_room_value();
    let legacy_awareness = ["line_of_sight", "awareness"].join("_");
    let legacy_memory = ["memory", "rounds"].join("_");
    top_level_perception.world_seed["perception"] = serde_json::json!({"mode": legacy_awareness});
    top_level_perception.world_seed["perception"]
        .as_object_mut()
        .expect("legacy perception object")
        .insert(legacy_memory, serde_json::json!(2));
    assert_error(&top_level_perception, "unknown field `perception`");
}

#[test]
fn summon_templates_require_complete_ai_and_summoner_social_rows() {
    let mut value = first_room_value();
    value.push_selected("actor_definitions", "actor/summon/guardian/automatic_actor_content", serde_json::json!({
        "id": "actor/summon/guardian",
        "name": "Guardian",
        "kind": "monster",
        "creature_traits": [],
        "social": {"alignment_source":{"kind":"inherent","alignment":"lawful"},"nature":"other","behavior":"alignment_creature","owner_relation":"summoner"},
        "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
        "death": {"remains": "none"},
        "stats": {"hp": 4, "attack": 1, "defense": 1},
        "ai": {
            "behavior": "simple_chase",
            "cadence_units": 2,
            "aggro_radius": 6,
            "leash_range": 6,
            "awareness": {"mode": "line_of_sight_memory", "memory_opportunities": 2},
            "physical_attack_modes": ["poke", "fight"]
        },
        "xp_value": 0,
        "physical_damage_affinity_profile_id": "ordinary",
        "monster_abilities": []
    }));
    value.push_selected(
        "summon_templates",
        "summon/guardian/automatic_actor_content",
        serde_json::json!({
            "id": "guardian",
            "actor_definition_id": "actor/summon/guardian",
            "item_instances": {},
            "carried": {"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}},
            "active_effects": []
        }),
    );
    parse(&value).expect("complete summon template parses");

    let mut missing_ai = value.clone();
    missing_ai
        .selected_by_runtime_id_mut("actor_definitions", "actor/summon/guardian")
        .as_object_mut()
        .expect("template object")
        .remove("ai");
    assert_error(&missing_ai, "missing field `ai`");

    let mut no_owner_relation = value.clone();
    no_owner_relation.selected_by_runtime_id_mut("actor_definitions", "actor/summon/guardian")["social"]
        ["owner_relation"] = serde_json::json!("none");
    assert_error(
        &no_owner_relation,
        ".social.owner_relation must be summoner for a summon template",
    );

    let mut sibling_leash = value;
    sibling_leash.selected_by_runtime_id_mut("actor_definitions", "actor/summon/guardian")["leash_range"] =
        serde_json::json!(6);
    assert_error(&sibling_leash, "unknown field `leash_range`");
}
