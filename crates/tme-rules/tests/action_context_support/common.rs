#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(crate) fn status_engine() -> Engine {
    ContentParts::tracked("status_effects", "profile/status_effects")
        .engine(7)
        .expect("status content should start")
}

pub(crate) fn ensure_current_class_growth_profile(parts: &mut ContentParts) {
    let class_id = parts.world_seed["actors"][0]["character"]["identity"]["current_class_id"]
        .as_str()
        .expect("test character current class")
        .to_string();
    if parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array()
        .expect("test progression profiles")
        .iter()
        .any(|profile| profile["class_id"] == class_id)
    {
        return;
    }
    let profile =
        parts.catalog["rules_profiles"]["rules/first_room"]["progression"]["growth_profiles"]
            .as_array()
            .expect("canonical growth profiles")
            .iter()
            .find(|profile| profile["class_id"] == class_id)
            .unwrap_or_else(|| panic!("missing canonical growth profile for {class_id:?}"))
            .clone();
    parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("test progression profiles")
        .push(profile);
}

pub(crate) fn option_by_id<'a>(
    options: &'a [tme_rules::ActionOptionV1],
    id: &str,
) -> &'a tme_rules::ActionOptionV1 {
    options
        .iter()
        .find(|option| option.id == id)
        .unwrap_or_else(|| panic!("missing option {id}"))
}

pub(crate) fn make_command(intent: PlayerIntentPayloadV1) -> PlayerCommandV1 {
    PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent,
    }
}

pub(crate) fn first_room_engine() -> Engine {
    ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("first-room content should start")
}
