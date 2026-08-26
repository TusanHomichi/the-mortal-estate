use crate::support::content_parts::ContentParts;
use tme_rules::{ActorKind, CharacterAlignment, Engine};

fn parts(case_id: &str, profile: &str) -> ContentParts {
    ContentParts::tracked(case_id, profile)
}

fn engine(case_id: &str, profile: &str) -> Engine {
    parts(case_id, profile)
        .engine(7)
        .expect("content should start")
}

fn validation_error(parts: &ContentParts) -> String {
    match parts.validated_seed() {
        Ok(_) => panic!("mutated content must fail validation"),
        Err(error) => error,
    }
}

#[test]
fn character_sheet_fixture_loads() {
    let engine = engine("character_sheet", "profile/character_sheet");
    let player = &engine.world().actors[0];
    assert_eq!(player.kind, ActorKind::Player);
    assert_eq!(player.id, "player");
    let character_id = player
        .character_id
        .as_ref()
        .expect("character-backed player should have a stable character id");
    assert_eq!(character_id.as_str(), "character:character_sheet:primary");
    assert_ne!(character_id.as_str(), player.id);
    let cs = player
        .character
        .as_ref()
        .expect("player should have character sheet");
    assert_eq!(cs.identity.base_class_id, "fighter");
    assert_eq!(cs.identity.display_class, "Fighter");
    assert_eq!(cs.identity.nationality_id, "aldland");
    assert_eq!(cs.attributes.strength, 14);
    assert_eq!(cs.attributes.dexterity, 12);
    assert_eq!(cs.progression.level, 1);
    assert_eq!(cs.progression.experience, 0);
    assert_eq!(cs.resources.hp, 12);
    assert_eq!(cs.resources.max_hp, 12);
    assert_eq!(cs.physical_attribute_adds.strength_adds, 1);
    assert_eq!(cs.physical_attribute_adds.dexterity_adds, 0);
    assert!(cs.promotion_history.is_empty());
    assert_eq!(cs.alignment_state.alignment, CharacterAlignment::Lawful);
    assert_eq!(cs.alignment_state.karma_points, 0);
}

#[test]
fn non_character_fixture_has_no_character() {
    let engine = engine("first_room", "profile/first_room");
    let player = &engine.world().actors[0];
    assert!(
        player.character.is_none(),
        "non-character fixture should have no character"
    );
    assert!(
        player.character_id.is_none(),
        "non-character actor should have no stable character id"
    );
}

#[test]
fn monster_with_character_fails_validation() {
    let mut value = parts("character_sheet", "profile/character_sheet");
    // Clone the player's character block onto the monster
    let character = value.actors_mut()[0]["character"].clone();
    value.actors_mut()[1]["character"] = character;
    value.actors_mut()[1]["character_id"] =
        serde_json::json!("character:character_sheet:invalid_monster");
    let msg = validation_error(&value);
    assert!(
        msg.contains("character is only valid for players"),
        "expected character-on-monster error, got: {msg}"
    );
}

#[test]
fn attribute_out_of_range_fails_validation() {
    let mut value = parts("character_sheet", "profile/character_sheet");
    value.actors_mut()[0]["character"]["attributes"]["strength"] = serde_json::Value::from(99);
    let msg = validation_error(&value);
    assert!(
        msg.contains("strength must be between 3 and 18"),
        "expected attribute range error, got: {msg}"
    );
}

#[test]
fn level_zero_fails_validation() {
    let mut value = parts("character_sheet", "profile/character_sheet");
    value.actors_mut()[0]["character"]["progression"]["level"] = serde_json::Value::from(0);
    assert!(!validation_error(&value).is_empty());
}

#[test]
fn resources_hp_exceeds_max_fails_validation() {
    let mut value = parts("character_sheet", "profile/character_sheet");
    value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::Value::from(999);
    let msg = validation_error(&value);
    assert!(
        msg.contains("hp must not exceed max_hp"),
        "expected hp constraint error, got: {msg}"
    );
}

#[test]
fn snapshot_includes_character_sheet() {
    let engine = engine("character_sheet", "profile/character_sheet");
    let snapshot = engine.snapshot();
    let player_view = snapshot
        .actors
        .iter()
        .find(|a| a.id == "player")
        .expect("player should be in snapshot");
    let csv = player_view
        .character
        .as_ref()
        .expect("player view should have character sheet");
    assert_eq!(csv.identity.display_class, "Fighter");
    assert_eq!(csv.identity.nationality_id, "aldland");
    assert_eq!(csv.alignment_state.alignment, CharacterAlignment::Lawful);
    assert_eq!(csv.alignment_state.karma_points, 0);
    assert_eq!(csv.attributes.strength, 14);
    assert_eq!(csv.progression.level, 1);
    assert_eq!(csv.physical_attribute_adds.strength_adds, 1);
}

#[test]
fn snapshot_determinism_with_character() {
    let engine_a = engine("character_sheet", "profile/character_sheet");
    let engine_b = engine("character_sheet", "profile/character_sheet");
    let snap_a = engine_a.snapshot();
    let snap_b = engine_b.snapshot();
    let json_a = serde_json::to_string(&snap_a).expect("snapshot should serialize");
    let json_b = serde_json::to_string(&snap_b).expect("snapshot should serialize");
    assert_eq!(
        json_a, json_b,
        "snapshots with character sheet must be deterministic"
    );
}
