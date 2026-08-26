#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(crate) fn wizard_spell_context_engine(known_spell_id: Option<&str>, skill_rank: i32) -> Engine {
    let mut parts = ContentParts::tracked("spell_readiness", "profile/spell_readiness");
    parts.profile_value_mut()["spells"] = serde_json::json!([
        "spell/spark/spell_readiness",
        "spell/mend/spell_readiness",
        "spell/charged_spark/spell_readiness"
    ]);
    for index in 0..3 {
        let spell = parts.selected_mut("spells", index);
        spell["status"] = serde_json::json!("stub");
        spell
            .as_object_mut()
            .expect("selected spell")
            .remove("effect");
    }
    let player = &mut parts.actors_mut()[0];
    player["character"]["skill_ledger"][0]["level"] = serde_json::json!(skill_rank);
    player["character"]["known_spells"] = known_spell_id.map_or_else(
        || serde_json::json!([]),
        |spell_id| {
            serde_json::json!([
                {
                    "spell_id": spell_id,
                    "lane": "wizard_magic",
                    "learned_at_level": 1
                }
            ])
        },
    );
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Target");
    let target = &mut parts.actors_mut()[1];
    target["id"] = serde_json::json!("target");
    target["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts.engine(7).expect("engine should start")
}
