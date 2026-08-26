#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(crate) fn bt_warmed_spell_engine(known_spell_ids: &[&str]) -> Engine {
    let mut parts = ContentParts::tracked(
        "area_path_terrain_spells",
        "profile/area_path_terrain_spells",
    );
    parts.profile_value_mut()["spells"] = serde_json::json!([
        "spell/web_field/area_path_terrain_spells",
        "spell/ember_cloud/area_path_terrain_spells"
    ]);
    for index in 0..2 {
        let spell = parts.selected_mut("spells", index);
        spell["status"] = serde_json::json!("stub");
        spell["casting"]["method"] = serde_json::json!("warm_then_cast");
        spell["casting"]["cast_class"] = serde_json::json!("path");
        spell
            .as_object_mut()
            .expect("selected spell")
            .remove("effect");
    }
    parts.actors_mut()[0]["character"]["known_spells"] = serde_json::Value::Array(
        known_spell_ids
            .iter()
            .map(|spell_id| {
                serde_json::json!({
                    "spell_id": spell_id,
                    "lane": "wizard_magic",
                    "learned_at_level": 1
                })
            })
            .collect(),
    );
    parts.engine(7).expect("engine should start")
}
