use crate::support::content_parts::ContentParts;
use tme_rules::model::{ActiveEffectSource, TileEffectState};
use tme_rules::{
    AttackSafety, CharacterAlignment, Coord, CorpseId, Engine, OBSERVED_SNAPSHOT_CONTRACT_VERSION,
    PlayerIntent, SNAPSHOT_CONTRACT_VERSION, SpellTarget, WorldPosition, WorldSnapshotV1,
    view::{SocialBehaviorViewV1, SocialNatureViewV1, SocialOwnerRelationViewV1},
};

fn portal_snapshot_engine() -> Engine {
    ContentParts::tracked(
        "utility_door_secret_item_spells",
        "profile/utility_door_secret_item_spells",
    )
    .engine(7)
    .expect("engine should start")
}

fn hidden_open_door_engine() -> Engine {
    let mut parts = ContentParts::tracked(
        "utility_door_secret_item_spells",
        "profile/utility_door_secret_item_spells",
    );
    let navigation = &mut parts.world_template["topology"]["edge/workroom/1/4"];
    navigation["kind"]["initial_state"] = serde_json::json!("open");
    navigation["hidden"] = serde_json::json!(true);
    parts.engine(7).expect("engine should start")
}

fn tile_transition_in_snapshot(
    snapshot: &tme_rules::WorldSnapshotV1,
    room_id: &str,
    position: Coord,
) -> Option<tme_rules::TransitionViewV1> {
    snapshot
        .realms
        .iter()
        .flat_map(|realm| realm.levels.iter())
        .find(|level| level.id == room_id)
        .and_then(|level| level.tiles.iter().find(|tile| tile.position == position))
        .and_then(|tile| tile.transition.clone())
}

fn tile_transition_in_observed_snapshot(
    snapshot: &tme_rules::WorldSnapshotV2,
    room_id: &str,
    position: Coord,
) -> Option<tme_rules::TransitionViewV1> {
    snapshot
        .realms
        .iter()
        .flat_map(|realm| realm.levels.iter())
        .find(|level| level.id == room_id)
        .and_then(|level| level.tiles.iter().find(|tile| tile.position == position))
        .and_then(|tile| tile.transition.clone())
}

fn bw_summon_snapshot_engine(alignment: &str) -> Engine {
    let mut parts = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    );
    parts.summon_actor_definition_mut(0)["social"]["alignment_source"]["alignment"] =
        serde_json::json!(alignment);
    parts.engine(7).expect("engine should start")
}

#[path = "snapshot_contract/snapshot_contract_version_matches_constant.rs"]
mod snapshot_contract_version_matches_constant;

#[path = "snapshot_contract/snapshot_includes_active_effects_and_resistance_boosts.rs"]
mod snapshot_includes_active_effects_and_resistance_boosts;
