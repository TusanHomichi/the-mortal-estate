use super::*;
use crate::model::ActorId;

#[test]
fn creation_is_complete_isolated_and_checkpoint_stable() {
    let engine = crate::engine::setup::test_engine("town_adventure_loop_gallery");
    let before = engine.export_checkpoint().unwrap();
    let profile = &engine.definition.creation_profiles()[0];
    let id = CharacterId::new("new-character");
    let created = engine
        .prepare_character_creation(
            &profile.id,
            id.clone(),
            "New Arrival",
            profile.character.attributes.clone(),
        )
        .unwrap();
    assert_eq!(engine.export_checkpoint().unwrap(), before);
    let actor = created
        .world
        .actor(&ActorId::new("created/new-character"))
        .unwrap();
    assert_eq!(actor.name, "New Arrival");
    assert_eq!(actor.character.as_ref().unwrap(), &profile.character);
    assert_eq!(actor.carried.gold, profile.carried.gold);
    assert_eq!(actor.carried.items.len(), profile.carried.items.len());
    assert!(!created.world.character_presence[&id].connected);
    for item in actor.carried.items.values() {
        assert!(item.starts_with("created/new-character/"));
        assert!(created.world.item_instances.contains_key(item));
    }
    let checkpoint = created.export_checkpoint().unwrap();
    let hydrated = Engine::hydrate_checkpoint(engine.definition.clone(), &checkpoint).unwrap();
    assert_eq!(hydrated.export_checkpoint().unwrap(), checkpoint);
    assert!(
        created
            .prepare_character_creation(
                &profile.id,
                id,
                "Again",
                profile.character.attributes.clone()
            )
            .is_err()
    );
}

#[test]
fn invalid_allocations_and_profile_selection_leave_everything_unchanged() {
    let engine = crate::engine::setup::test_engine("town_adventure_loop_gallery");
    let profile = &engine.definition.creation_profiles()[0];
    let before = engine.export_checkpoint().unwrap();
    for strength in [
        i32::MIN,
        7,
        19,
        i32::MAX,
        profile.character.attributes.strength + 1,
    ] {
        let mut attributes = profile.character.attributes.clone();
        attributes.strength = strength;
        assert!(
            engine
                .prepare_character_creation(&profile.id, CharacterId::new("new"), "New", attributes)
                .is_err()
        );
        assert_eq!(engine.export_checkpoint().unwrap(), before);
    }
    assert!(
        engine
            .prepare_character_creation(
                "knight",
                CharacterId::new("new"),
                "New",
                profile.character.attributes.clone()
            )
            .is_err()
    );
}

#[test]
fn invalid_creation_content_is_refused_before_world_start() {
    let (catalog, key, template, _) =
        crate::engine::setup::test_parts("town_adventure_loop_gallery");
    for fault in 0..4 {
        let mut invalid = catalog.clone();
        let profile = invalid.creation_profiles.values_mut().next().unwrap();
        match fault {
            0 => profile.arrival_id = "absent".into(),
            1 => profile.character.identity.current_class_id = "knight".into(),
            2 => profile.item_instances.clear(),
            _ => profile.attribute_points += 1,
        }
        assert!(
            invalid
                .select(&key)
                .unwrap()
                .validate_with_template(&template)
                .is_err()
        );
        assert!(GameDefinition::from_content(invalid, key.clone(), template.clone()).is_err());
    }
    let mut missing = serde_json::to_value(&catalog).unwrap();
    missing.as_object_mut().unwrap().remove("creation_profiles");
    assert!(serde_json::from_value::<crate::content::CatalogV6>(missing).is_err());
}
