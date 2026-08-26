#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(crate) fn profession_action_engine_with(
    class_id: &str,
    hide_class_ids: &[&str],
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let mut parts = ContentParts::tracked(
        "profession_specific_actions",
        "profile/profession_specific_actions",
    );
    let mut cells = vec![vec![vec!["flagstone"]; 7]; 7];
    for row in cells.iter_mut().take(7) {
        row[0] = vec!["stone_wall"];
        row[6] = vec!["stone_wall"];
    }
    for cell in cells[0].iter_mut().take(7) {
        *cell = vec!["stone_wall"];
    }
    for cell in cells[6].iter_mut().take(7) {
        *cell = vec!["stone_wall"];
    }
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 7,
            "height": 7,
            "cells": cells
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 3, "y": 3});
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 5, "y": 3});
    parts.actors_mut()[0]["character"]["identity"]["base_class_id"] = serde_json::json!(class_id);
    parts.actors_mut()[0]["character"]["identity"]["current_class_id"] =
        serde_json::json!(class_id);
    parts.actors_mut()[0]["character"]["identity"]["display_class"] = serde_json::json!(class_id);
    parts.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([]);
    let has_growth_profile = parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array()
        .expect("growth profiles")
        .iter()
        .any(|profile| profile["class_id"] == class_id);
    if !has_growth_profile {
        let source =
            parts.catalog["rules_profiles"]["rules/first_room"]["progression"]["growth_profiles"]
                .as_array()
                .expect("source growth profiles")
                .iter()
                .find(|profile| profile["class_id"] == class_id)
                .unwrap_or_else(|| panic!("missing growth profile for {class_id}"))
                .clone();
        parts.rules_source_mut()["progression"]["growth_profiles"]
            .as_array_mut()
            .expect("growth profiles")
            .push(source);
    }
    parts.selected_mut("profession_actions", 0)["class_ids"] = serde_json::json!(hide_class_ids);
    parts.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
        serde_json::json!(false);
    mutate(&mut parts);
    parts.engine(7).expect("profession content should start")
}

pub(crate) fn profession_action_engine(class_id: &str, hide_class_ids: &[&str]) -> Engine {
    profession_action_engine_with(class_id, hide_class_ids, |_| {})
}
