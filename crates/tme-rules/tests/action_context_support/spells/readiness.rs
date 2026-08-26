#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(crate) fn spell_readiness_options_engine() -> Engine {
    let mut parts = ContentParts::tracked("spell_readiness", "profile/spell_readiness");
    parts.profile_value_mut()["spells"] = serde_json::json!([
        "spell/spark/spell_readiness",
        "spell/charged_spark/spell_readiness"
    ]);
    parts.actors_mut()[0]["character"]["known_spells"] = serde_json::json!([
        {
            "spell_id": "spark",
            "lane": "wizard_magic",
            "learned_at_level": 1
        },
        {
            "spell_id": "charged_spark",
            "lane": "wizard_magic",
            "learned_at_level": 1
        }
    ]);
    parts.engine(7).expect("engine should start")
}
