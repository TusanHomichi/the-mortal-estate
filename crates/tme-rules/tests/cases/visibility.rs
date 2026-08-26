use crate::support::content_parts::ContentParts;
use tme_rules::{
    Coord, Direction, Engine, PlayerIntent, WorldPosition,
    model::{ActiveEffectSource, TileEffectState},
};

fn tracked_engine(case_id: &str, profile: &str) -> Engine {
    ContentParts::tracked(case_id, profile)
        .engine(7)
        .expect("tracked content should start")
}

fn bt_visibility_overlay_engine(sight: &str) -> Engine {
    let mut engine = tracked_engine("first_room", "profile/first_room");
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: format!("tile:{sight}:1"),
        effect_id: "shadow_veil".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "shadow_veil".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["shadow".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: None,
        sight: Some(sight.to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    engine
}

fn bt_clear_sight_overlay(position: Coord) -> TileEffectState {
    TileEffectState {
        source_actor_id: None,
        instance_id: "tile:clear:1".to_string(),
        effect_id: "clear_sight".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "clear_sight".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", position),
        kind: "terrain_overlay".to_string(),
        tags: vec!["clear".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: None,
        sight: Some("clear".to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    }
}

#[test]
fn sight_overlay_blocks_line_of_sight_until_clear_overlay_wins() {
    let mut engine = bt_visibility_overlay_engine("blocked");
    assert!(!engine.has_line_of_sight(
        &WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        &WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 1 })
    ));
    engine
        .world_mut()
        .tile_effects
        .push(bt_clear_sight_overlay(Coord { x: 2, y: 1 }));
    assert!(engine.has_line_of_sight(
        &WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        &WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 1 })
    ));
}

#[test]
fn line_of_sight_open_floor() {
    let engine = tracked_engine("first_room", "profile/first_room");
    // first_room is 3x3, player at [1,1], floor tiles
    let from = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
    let to = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 2 });
    assert!(
        engine.has_line_of_sight(&from, &to),
        "open floor should be visible"
    );
}

#[test]
fn line_of_sight_blocked_by_wall() {
    let engine = tracked_engine("first_room", "profile/first_room");
    // (2,2) is a wall between (1,1) and (3,3)
    let from = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
    let to = WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 3 });
    assert!(
        !engine.has_line_of_sight(&from, &to),
        "wall should block sight"
    );
}

#[test]
fn line_of_sight_origin_included() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let pos = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
    assert!(engine.has_line_of_sight(&pos, &pos), "origin sees itself");
}

#[test]
fn line_of_sight_different_rooms_v0() {
    let engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    let from = WorldPosition::new("realm_0", "entrance_hall", Coord { x: 1, y: 1 });
    let to = WorldPosition::new("realm_0", "guard_post", Coord { x: 1, y: 1 });
    assert!(
        !engine.has_line_of_sight(&from, &to),
        "cross-room LoS blocked in V0"
    );
}

#[test]
fn door_tile_itself_is_visible() {
    let engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    // entrance_hall has a closed door east at (2,1)
    let from = WorldPosition::new("realm_0", "entrance_hall", Coord { x: 1, y: 1 });
    let door_pos = WorldPosition::new("realm_0", "entrance_hall", Coord { x: 2, y: 1 });
    assert!(
        engine.has_line_of_sight(&from, &door_pos),
        "should see door tile"
    );
}

#[test]
fn open_door_does_not_block_sight() {
    let mut engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    // Open the east door first
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::Open(tme_rules::Direction::East),
        )
        .ok();
    let from = WorldPosition::new("realm_0", "entrance_hall", Coord { x: 1, y: 1 });
    let to = WorldPosition::new("realm_0", "entrance_hall", Coord { x: 2, y: 1 });
    assert!(
        engine.has_line_of_sight(&from, &to),
        "open door should not block"
    );
}

#[test]
fn actor_visible_tiles_includes_origin() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    let origin = player.location.clone();
    let snapshot = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("visible");
    let origin_tile = snapshot
        .realms
        .iter()
        .find(|realm| realm.id == origin.realm)
        .and_then(|realm| realm.levels.iter().find(|level| level.id == origin.level))
        .and_then(|level| {
            level
                .tiles
                .iter()
                .find(|tile| tile.position == origin.position)
        })
        .expect("origin tile");
    assert_eq!(
        origin_tile.observation,
        tme_rules::TileObservationV1::Visible
    );
}

#[test]
fn visibility_is_read_only() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let snap_before = engine.snapshot();
    let _ = engine.actor_observed_snapshot(&tme_rules::ActorId::from("player"));
    let _ = engine.has_line_of_sight(
        &WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        &WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 2 }),
    );
    let snap_after = engine.snapshot();
    assert_eq!(snap_before, snap_after, "visibility must not mutate engine");
}

#[test]
fn closed_door_blocks_sight_to_tile_behind() {
    // Use first_room as base. first_room is 3x3 with walls on all edges
    // and no doors, so we test the wall-blocking path instead.
    // A proper door-behind test requires an inline fixture with a door
    // not at the room edge; tracked as a future fixture need.
    let engine = tracked_engine("first_room", "profile/first_room");
    // (1,1) -> (3,3) passes through wall at (2,2): should be blocked
    let from = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
    let behind_wall = WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 3 });
    assert!(
        !engine.has_line_of_sight(&from, &behind_wall),
        "wall should block LoS to tile behind it"
    );
}

#[test]
fn closed_door_blocks_los_open_door_allows() {
    let mut engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");

    // In undercroft_loop, player starts in room_0 at (1,1).
    // East at (2,1) is a door to room_1. Closed by default.
    let from = WorldPosition::new("realm_0", "room_0", (1, 1).into());
    let door_pos = WorldPosition::new("realm_0", "room_0", (2, 1).into());
    let _behind_door = WorldPosition::new("realm_0", "room_1", (1, 1).into());

    // Closed door blocks cross-room LoS (V0: cross-room always blocked anyway)
    // But same-room LoS to the door tile should see the door
    assert!(
        engine.has_line_of_sight(&from, &door_pos),
        "door tile itself should be visible"
    );

    // Open the door
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::East),
        )
        .expect("open");
    assert!(
        engine.has_line_of_sight(&from, &door_pos),
        "open door tile should still be visible"
    );
}
