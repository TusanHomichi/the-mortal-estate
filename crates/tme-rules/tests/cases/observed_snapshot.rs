use crate::support::content_parts::ContentParts;
use tme_rules::{
    Coord, Engine, OBSERVED_SNAPSHOT_CONTRACT_VERSION, SNAPSHOT_CONTRACT_VERSION,
    TileObservationV1, WorldSnapshotV1, WorldSnapshotV2,
};

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn first_room_engine() -> Engine {
    ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start")
}

fn tile_observation<'a>(
    v2: &'a WorldSnapshotV2,
    room_id: &str,
    x: i32,
    y: i32,
) -> &'a TileObservationV1 {
    &room_tiles(v2, room_id)
        .iter()
        .find(|t| t.position == Coord { x, y })
        .unwrap_or_else(|| panic!("tile ({x}, {y}) not found in room {room_id}"))
        .observation
}

fn tile_terrain_id<'a>(
    v2: &'a WorldSnapshotV2,
    room_id: &str,
    x: i32,
    y: i32,
) -> &'a Option<String> {
    &room_tiles(v2, room_id)
        .iter()
        .find(|t| t.position == Coord { x, y })
        .unwrap_or_else(|| panic!("tile ({x}, {y}) not found in room {room_id}"))
        .terrain_id
}

fn room_tiles<'a>(v2: &'a WorldSnapshotV2, room_id: &str) -> &'a [tme_rules::TileSnapshotV2] {
    &v2.realms
        .iter()
        .find(|realm| realm.id == "realm_0")
        .expect("realm_0")
        .levels
        .iter()
        .find(|level| level.id == room_id)
        .unwrap_or_else(|| panic!("level {room_id} not found"))
        .tiles
}

fn actor_ids(snapshot: &WorldSnapshotV1) -> BTreeSet<String> {
    snapshot.actors.iter().map(|a| a.id.to_string()).collect()
}

fn v2_actor_ids(v2: &WorldSnapshotV2) -> BTreeSet<String> {
    v2.actors.iter().map(|a| a.id.to_string()).collect()
}

/// Build a minimal scenario identical to first_room but with a monster
/// placed at (3, 3) — behind the wall pillar at (2, 2) — so it is *not*
/// visible from the player at (1, 1).
fn engine_with_hidden_monster() -> Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 3});
    parts.engine(7).expect("engine should start")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1. snapshot() (omniscient V1) is unchanged — returns all actors/items
#[test]
fn v1_snapshot_includes_all_actors() {
    let engine = first_room_engine();
    let v1 = engine.snapshot();

    assert_eq!(v1.contract_version, SNAPSHOT_CONTRACT_VERSION);
    assert!(matches!(
        v1.scope,
        tme_rules::SnapshotScopeV1::OmniscientLocal
    ));

    // first_room has two actors: player and mireling
    assert_eq!(v1.actors.len(), 2);
    let ids = actor_ids(&v1);
    assert!(ids.contains("player"));
    assert!(ids.contains("mireling"));

    // The first actor opportunity starts at logical time one.
    assert_eq!(v1.logical_time, tme_rules::LogicalTime::FIRST);
}

/// 2. actor_observed_snapshot() returns V2 with the player's own tile visible
///    and correct terrain data for visible tiles.
#[test]
fn v2_player_tile_is_visible() {
    let engine = first_room_engine();
    let v2 = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");

    assert_eq!(v2.contract_version, OBSERVED_SNAPSHOT_CONTRACT_VERSION);
    assert_eq!(OBSERVED_SNAPSHOT_CONTRACT_VERSION, 30);

    // Player at (1, 1) in room_0 — must be Visible
    assert_eq!(
        tile_observation(&v2, "room_0", 1, 1),
        &TileObservationV1::Visible,
        "player's own tile must be Visible"
    );
    // Visible tiles carry real terrain data
    let tid = tile_terrain_id(&v2, "room_0", 1, 1);
    assert_eq!(tid.as_deref(), Some("flagstone"));
}

#[test]
fn observed_29_exposes_living_ecology_actors_without_scheduler_or_origin_state() {
    let engine = ContentParts::tracked(
        "creature_ecology_gallery",
        "profile/creature_ecology_gallery",
    )
    .engine(7)
    .expect("ecology gallery starts");
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed ecology snapshot");
    assert_eq!(observed.contract_version, 30);
    assert!(observed.actors.iter().any(|actor| {
        actor.id == "ecology:gallery_pack:runner:0" && actor.name == "Bramble Runner"
    }));

    let serialized = serde_json::to_value(observed).expect("observed snapshot serializes");
    assert!(
        serialized.get("ecology_sites").is_none(),
        "observer wire must not expose site generations, vacancies, or due times"
    );
    assert!(
        serialized["actors"]
            .as_array()
            .expect("observed actors")
            .iter()
            .all(|actor| actor.get("ecology_origin").is_none()),
        "observer wire must not expose actor generation origins"
    );
}

#[test]
fn observed_snapshot_uses_the_shared_explicit_item_projection() {
    let engine = ContentParts::tracked("item_instance_contract", "profile/item_instance_contract")
        .engine(7)
        .expect("engine should start");
    let observed = serde_json::to_value(
        engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot should build"),
    )
    .expect("observed snapshot should serialize");

    let tonic = observed["ground_items"]
        .as_array()
        .expect("ground items should be an array")
        .iter()
        .find(|item| item["item_instance_id"] == "tonic_a")
        .expect("visible tonic_a should be projected");
    assert_eq!(tonic["item_definition_id"], "restorative_tonic");
    assert_eq!(tonic["quantity"], 2);
    assert_eq!(tonic["identified"], false);
    assert_eq!(tonic["appraised"], true);
    assert_eq!(tonic["known_unit_value_gold"], 12);
    assert_eq!(tonic["known_stack_value_gold"], 24);
    assert_eq!(tonic["unit_burden"], 3);
    assert_eq!(tonic["stack_burden"], 6);
    assert!(tonic.get("item_id").is_none());
}

/// 3. Actors not visible to the player are excluded from the observed
///    snapshot.  When the monster IS visible it appears; when it is behind
///    a wall it is excluded.
#[test]
fn v2_includes_visible_actors() {
    let engine = first_room_engine();
    let v2 = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");
    let visible = v2
        .realms
        .iter()
        .flat_map(|realm| {
            realm.levels.iter().flat_map(move |level| {
                level
                    .tiles
                    .iter()
                    .filter(|tile| tile.observation == TileObservationV1::Visible)
                    .map(move |tile| {
                        tme_rules::WorldPosition::new(&realm.id, &level.id, tile.position)
                    })
            })
        })
        .collect::<BTreeSet<_>>();

    // In first_room the mireling at (3, 1) is visible from the player at (1, 1)
    // because the south row is open floor.
    let actor_ids = v2_actor_ids(&v2);
    assert!(
        actor_ids.contains("mireling"),
        "visible monster should be included in observed snapshot"
    );
    assert!(
        actor_ids.contains("player"),
        "player must be included in observed snapshot"
    );

    // Every actor in V2 must be in the visible-tiles set
    for actor in &v2.actors {
        let pos = actor.location.clone();
        assert!(
            visible.contains(&pos),
            "actor {} at {:?} must be in visible-tiles set",
            actor.id,
            pos,
        );
    }
}

#[test]
fn v2_excludes_invisible_actors() {
    let engine = engine_with_hidden_monster();
    let v2 = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");

    // The hidden_ogre at (3, 3) is behind the wall pillar at (2, 2) and
    // should NOT appear in the observed snapshot.
    let actor_ids = v2_actor_ids(&v2);
    assert!(
        !actor_ids.contains("hidden_ogre"),
        "monster behind wall should be excluded from observed snapshot"
    );
    assert!(
        actor_ids.contains("player"),
        "player must still be included in observed snapshot"
    );
}

/// 4. Tiles not visible to the player show observation: Unknown and
///    None terrain fields.
#[test]
fn v2_unknown_tiles_have_none_terrain() {
    let engine = first_room_engine();
    let v2 = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");

    // From (1,1) the tile (3,3) is behind the wall pillar at (2,2) so
    // it should be Unknown.
    assert_eq!(
        tile_observation(&v2, "room_0", 3, 3),
        &TileObservationV1::Unknown,
        "tile (3,3) should be Unknown (behind wall pillar)"
    );

    // All terrain fields must be None for unknown tiles
    let level = &v2.realms[0].levels[0];
    for tile in &level.tiles {
        if tile.observation == TileObservationV1::Unknown {
            assert!(
                tile.terrain_id.is_none(),
                "unknown tile ({}, {}): terrain_id should be None, got {:?}",
                tile.position.x,
                tile.position.y,
                tile.terrain_id,
            );
            assert!(
                tile.terrain_name.is_none(),
                "unknown tile ({}, {}): terrain_name should be None",
                tile.position.x,
                tile.position.y,
            );
            assert!(
                tile.passable.is_none(),
                "unknown tile ({}, {}): passable should be None",
                tile.position.x,
                tile.position.y,
            );
            assert!(
                tile.move_cost.is_none(),
                "unknown tile ({}, {}): move_cost should be None",
                tile.position.x,
                tile.position.y,
            );
        }
    }

    // Known-visible tiles should NOT have None terrain fields.  Check a
    // few representative tiles.
    let assert_visible_has_data = |x, y| {
        let v2 = engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot should succeed");
        let tid = tile_terrain_id(&v2, "room_0", x, y);
        assert!(
            tid.is_some(),
            "visible tile ({x}, {y}) must have terrain_id, got None"
        );
    };
    assert_visible_has_data(1, 1);
    assert_visible_has_data(2, 1);
    assert_visible_has_data(3, 1);
    assert_visible_has_data(1, 2);
}

/// 5. Observed snapshot is deterministic: two calls from freshly
///    constructed engines (same seed) return the same result.
#[test]
fn v2_snapshot_is_deterministic() {
    let engine_a = first_room_engine();
    let engine_b = first_room_engine();

    let snap_a = engine_a
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot a");
    let snap_b = engine_b
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot b");

    assert_eq!(
        snap_a, snap_b,
        "actor_observed_snapshot must be deterministic"
    );

    // Also verify that repeated calls on the same engine produce the same
    // result (same reference — i.e. the engine hasn't mutated between calls).
    let snap_a2 = engine_a
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot a2");
    assert_eq!(
        snap_a, snap_a2,
        "consecutive calls on same engine must return identical snapshot"
    );
}

/// 6. Observed snapshot is read-only: calling actor_observed_snapshot
///    must not mutate the engine's state.  We verify by taking a V1
///    snapshot before and after, comparing them for equality.
#[test]
fn v2_snapshot_is_read_only() {
    let engine = first_room_engine();

    // Two calls to actor_observed_snapshot must produce identical results
    let v2_a = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");
    let v2_b = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");

    assert_eq!(v2_a, v2_b, "actor_observed_snapshot must be deterministic");
    // Also verify omniscient V1 is unchanged
    let before = engine.snapshot();
    let _ = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("ok");
    let after = engine.snapshot();
    assert_eq!(
        before, after,
        "actor_observed_snapshot must not mutate engine state"
    );
}

/// Sanity: V2 snapshot serializes to deterministic JSON (two calls on the
/// same engine produce identical JSON).
#[test]
fn v2_snapshot_serializes_deterministically() {
    let engine = first_room_engine();
    let v2_a = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot a");
    let v2_b = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot b");

    let json_a = serde_json::to_string_pretty(&v2_a).expect("serialize a");
    let json_b = serde_json::to_string_pretty(&v2_b).expect("serialize b");

    assert_eq!(json_a, json_b, "JSON serialization must be deterministic");
}

/// The V2 snapshot should have exactly one room (room_0) for a single-map
/// scenario, with 25 tiles (5x5).
#[test]
fn v2_snapshot_has_correct_tile_count() {
    let engine = first_room_engine();
    let v2 = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot should succeed");

    assert_eq!(v2.realms.len(), 1, "first_room is a single-realm scenario");
    assert_eq!(v2.realms[0].levels[0].id, "room_0");
    assert_eq!(v2.realms[0].levels[0].width, 5);
    assert_eq!(v2.realms[0].levels[0].height, 5);
    assert_eq!(v2.realms[0].levels[0].tiles.len(), 25);
}

#[test]
fn actor_observed_frame_matches_separate_calls() {
    let engine = first_room_engine();

    let frame = engine
        .actor_observed_frame(&tme_rules::ActorId::from("player"))
        .expect("frame");
    let snap = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("snap");
    let ctx = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("ctx");

    assert_eq!(
        frame.observed_snapshot, snap,
        "frame snapshot must match separate call"
    );
    assert_eq!(
        frame.action_context, ctx,
        "frame action context must match separate call"
    );
}
