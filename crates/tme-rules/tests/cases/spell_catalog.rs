use crate::support::content_parts::ContentParts;
use tme_rules::{
    SpellCatalogState, SpellEffectFamily, SpellResistanceMitigationMode, SpellTargetKind,
};

fn catalog_parts() -> ContentParts {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    let marker = ["TME-", "PLACEHOLDER: focused internal catalog test."].concat();
    parts.catalog["clean_content"] = serde_json::json!(false);
    parts.catalog["research_boundary"] = serde_json::json!({
        "status": "internal_parity_fixture",
        "notes": marker,
        "review_refs": ["Slice DU spell catalog tests"]
    });
    parts.profile_value_mut()["spells"] = serde_json::json!([]);
    parts.push_selected(
        "spells",
        "spell/catalog_row_spark/test",
        serde_json::json!({
            "id": "catalog.row.spark",
            "name": "Catalog Spark",
            "status": "stub",
            "lane": "wizard_magic",
            "skill_requirement": 1,
            "mp_cost": 2,
            "social": {"hostile_act": true, "town_law": "permitted"},
            "acquisition": {"gold_cost": 0},
            "casting": {"method": "warm_then_cast", "cast_class": "character"},
            "catalog_entry": {
                "row_id": "catalog.row.spark",
                "topic_id": "spark",
                "acquisition_row_id": "catalog.acquire.spark",
                "variant_id": "base",
                "effect_family": "direct_damage",
                "target_kind": "actor",
                "state": "matched",
                "open_question_ids": ["catalog_formula_open"],
                "resistance_tags": ["arcane"],
                "resistance_mitigation_mode": "half_damage",
                "client_row_id": "client_spell.1",
                "client_spell_id": 1,
                "client_verb_type": 4,
                "client_powerable": false,
                "client_spell_poem_id": 53,
                "client_offensive": false
            }
        }),
    );
    parts.push_selected(
        "spells",
        "spell/catalog_row_client_only/test",
        serde_json::json!({
            "id": "catalog.row.client_only",
            "name": "Client-Only Catalog Row",
            "status": "stub",
            "social": {"hostile_act": false, "town_law": "permitted"},
            "catalog_entry": {
                "row_id": "catalog.row.client_only",
                "topic_id": "client_only",
                "acquisition_row_id": null,
                "variant_id": "base",
                "effect_family": "speed",
                "target_kind": null,
                "state": "open_evidence",
                "open_question_ids": ["catalog_details_open"],
                "resistance_tags": [],
                "resistance_mitigation_mode": null
            }
        }),
    );
    parts
}

fn validation_messages(value: &ContentParts) -> Vec<String> {
    match value.definition() {
        Ok(_) => panic!("mutated catalog must reject"),
        Err(error) => error.split("; ").map(str::to_string).collect(),
    }
}

#[test]
fn marked_catalog_loads_into_read_only_runtime_lookup() {
    let engine = catalog_parts()
        .engine(7)
        .expect("marked catalog should load");
    let entries = engine.spell_catalog_entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);

    let class_linked = engine
        .spell_catalog_entry("catalog.row.spark")
        .expect("class-linked row should load");
    assert_eq!(class_linked.spell_id, "catalog.row.spark");
    assert_eq!(class_linked.row_id, class_linked.spell_id);
    assert_eq!(class_linked.topic_id, "spark");
    assert_eq!(
        class_linked.acquisition_row_id.as_deref(),
        Some("catalog.acquire.spark")
    );
    assert_eq!(class_linked.effect_family, SpellEffectFamily::DirectDamage);
    assert_eq!(class_linked.target_kind, Some(SpellTargetKind::Actor));
    assert_eq!(class_linked.state, SpellCatalogState::Matched);
    assert_eq!(class_linked.resistance_tags, ["arcane"]);
    assert_eq!(
        class_linked.resistance_mitigation_mode,
        Some(SpellResistanceMitigationMode::HalfDamage)
    );
    assert_eq!(
        class_linked.client_row_id.as_deref(),
        Some("client_spell.1")
    );
    assert_eq!(class_linked.client_spell_id, Some(1));
    assert_eq!(class_linked.client_verb_type, Some(4));
    assert_eq!(class_linked.client_powerable, Some(false));
    assert_eq!(class_linked.client_spell_poem_id, Some(53));
    assert_eq!(class_linked.client_offensive, Some(false));

    let topic_only = engine
        .spell_catalog_entry("catalog.row.client_only")
        .expect("topic-only row should load");
    assert_eq!(topic_only.acquisition_row_id, None);
    assert_eq!(topic_only.target_kind, None);
    assert_eq!(topic_only.state, SpellCatalogState::OpenEvidence);
    assert!(topic_only.resistance_tags.is_empty());
    assert_eq!(topic_only.resistance_mitigation_mode, None);
    assert_eq!(topic_only.client_row_id, None);
}

#[test]
fn clean_catalog_rejects_internal_catalog_metadata() {
    let mut value = catalog_parts();
    value.catalog["clean_content"] = serde_json::json!(true);
    value.catalog["research_boundary"] = serde_json::json!({
        "status": "clean_original_fixture",
        "notes": "Original focused clean catalog rejection fixture.",
        "review_refs": ["Slice DU spell catalog tests"]
    });
    assert!(validation_messages(&value).iter().any(|message| {
        message == "spell catalog entries are only valid in a marked internal_parity_fixture"
    }));
}

#[test]
fn catalog_mode_requires_every_spell_and_matching_row_ids() {
    let mut missing = catalog_parts();
    missing.push_selected(
        "spells",
        "spell/plain_stub/test",
        serde_json::json!({
            "id": "plain_stub",
            "name": "Plain Stub",
            "status": "stub",
            "social": {"hostile_act": false, "town_law": "permitted"}
        }),
    );
    assert!(validation_messages(&missing).iter().any(|message| {
        message == "spells[2].catalog_entry is required when any spell has catalog metadata"
    }));

    let mut mismatch = catalog_parts();
    mismatch.selected_mut("spells", 0)["catalog_entry"]["row_id"] =
        serde_json::json!("catalog.row.wrong");
    assert!(
        validation_messages(&mismatch)
            .iter()
            .any(|message| { message == "spells[0].catalog_entry.row_id must equal spells[0].id" })
    );
}

#[test]
fn catalog_metadata_rejects_duplicate_questions_and_invalid_open_target_state() {
    let mut value = catalog_parts();
    value.selected_mut("spells", 1)["catalog_entry"]["state"] = serde_json::json!("matched");
    value.selected_mut("spells", 1)["catalog_entry"]["open_question_ids"] =
        serde_json::json!(["catalog_details_open", "catalog_details_open"]);
    let messages = validation_messages(&value);
    assert!(messages.iter().any(|message| {
        message == "spells[1].catalog_entry.target_kind may be absent only for open_evidence"
    }));
    assert!(messages.iter().any(|message| {
        message.contains("spells[1].catalog_entry.open_question_ids[1] duplicates")
    }));
}

#[test]
fn catalog_metadata_rejects_invalid_resistance_metadata() {
    let mut value = catalog_parts();
    value.selected_mut("spells", 0)["catalog_entry"]["resistance_tags"] =
        serde_json::json!(["arcane", "", "arcane"]);
    value.selected_mut("spells", 1)["catalog_entry"]["resistance_mitigation_mode"] =
        serde_json::json!("negate");
    let messages = validation_messages(&value);
    assert!(messages.iter().any(|message| {
        message == "spells[0].catalog_entry.resistance_tags[1] must be non-empty"
    }));
    assert!(messages.iter().any(|message| {
        message.contains("spells[0].catalog_entry.resistance_tags[2] duplicates")
    }));
    assert!(messages.iter().any(|message| {
        message
            == "spells[1].catalog_entry.resistance_mitigation_mode requires at least one resistance tag"
    }));
}

#[test]
fn catalog_client_metadata_is_all_or_none_and_has_positive_ids() {
    let mut incomplete = catalog_parts();
    incomplete.selected_mut("spells", 0)["catalog_entry"]["client_offensive"] =
        serde_json::Value::Null;
    assert!(validation_messages(&incomplete).iter().any(|message| {
        message == "spells[0].catalog_entry client metadata must be complete when linked"
    }));

    let mut invalid = catalog_parts();
    invalid.selected_mut("spells", 0)["catalog_entry"]["client_row_id"] = serde_json::json!(" ");
    invalid.selected_mut("spells", 0)["catalog_entry"]["client_spell_id"] = serde_json::json!(0);
    invalid.selected_mut("spells", 0)["catalog_entry"]["client_spell_poem_id"] =
        serde_json::json!(0);
    let messages = validation_messages(&invalid);
    assert!(messages.iter().any(|message| {
        message == "spells[0].catalog_entry.client_row_id must be non-empty when present"
    }));
    assert!(messages.iter().any(|message| {
        message == "spells[0].catalog_entry.client_spell_id must be positive when present"
    }));
    assert!(messages.iter().any(|message| {
        message == "spells[0].catalog_entry.client_spell_poem_id must be positive when present"
    }));
}

#[test]
fn catalog_metadata_rejects_operational_and_topic_only_mismatches() {
    let mut value = catalog_parts();
    value.selected_mut("spells", 0)["effect"] =
        serde_json::json!({"family": "healing", "potency": 1});
    value.selected_mut("spells", 0)["target"] = serde_json::json!({"kind": "self"});
    value.selected_mut("spells", 1)["lane"] = serde_json::json!("wizard_magic");
    let messages = validation_messages(&value);
    assert!(messages.iter().any(|message| {
        message == "spells[0].effect.family must match spells[0].catalog_entry.effect_family"
    }));
    assert!(messages.iter().any(|message| {
        message == "spells[0].target.kind must match spells[0].catalog_entry.target_kind"
    }));
    assert!(messages.iter().any(|message| {
        message == "spells[1].lane must be absent for a topic-only catalog entry"
    }));
}

#[test]
fn ordinary_clean_content_loads_an_empty_runtime_catalog() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("clean fixture should load");
    assert_eq!(engine.spell_catalog_entries().len(), 0);
    assert!(engine.spell_catalog_entry("spark").is_none());
}
