use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};

fn profile_for(case_id: &str) -> String {
    if case_id == "inspect_room" {
        "profile/combat_labels".to_string()
    } else {
        format!("profile/{case_id}")
    }
}

fn parts(case_id: &str) -> ContentParts {
    ContentParts::tracked(case_id, &profile_for(case_id))
}

fn definition_error(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("definition mutation must fail")
}

fn seed_error(parts: &ContentParts) -> String {
    parts.validated_seed().expect_err("seed mutation must fail")
}

fn decode_error(parts: &ContentParts) -> String {
    parts.decode().expect_err("strict decode must fail")
}

fn assert_has(error: &str, expected: &str) {
    assert!(
        error.contains(expected),
        "expected {expected:?} in diagnostic:\n{error}"
    );
}

#[test]
fn world_template_three_envelope_geometry_and_layers_are_strict() {
    let mut schema = parts("first_room");
    schema.world_template["schema_version"] = json!(1);
    assert_has(
        &definition_error(&schema),
        "world_template.schema_version must be 3",
    );

    let mut kind = parts("first_room");
    kind.world_template["kind"] = json!("map");
    assert_has(
        &definition_error(&kind),
        "world_template.kind must be \"world_template\"",
    );

    let mut empty = parts("first_room");
    empty.world_template["id"] = json!(" ");
    assert_has(
        &definition_error(&empty),
        "world_template.id must be non-empty",
    );

    let mut no_realms = parts("first_room");
    no_realms.world_template["realms"] = json!({});
    assert_has(
        &definition_error(&no_realms),
        "world_template.realms must be non-empty",
    );

    let mut bad_arrival = parts("first_room");
    bad_arrival.world_template["arrivals"]["bad"] = json!({
        "realm": "realm_0",
        "level": "missing",
        "position": {"x": 1, "y": 1}
    });
    assert_has(
        &definition_error(&bad_arrival),
        "references missing realm/level realm_0/missing",
    );

    let mut short_row = parts("first_room");
    short_row.template_levels_source_mut()["room_0"]["cells"][1] =
        json!([["stone_wall"], ["flagstone"], ["flagstone"], ["stone_wall"]]);
    assert_has(&definition_error(&short_row), "has 4 cells but width is 5");

    let mut missing_rows_with_arrival = parts("first_room");
    missing_rows_with_arrival.template_levels_source_mut()["room_0"]["cells"] = json!([]);
    missing_rows_with_arrival.world_template["arrivals"]["inside_declared_bounds"] = json!({
        "realm": "realm_0",
        "level": "room_0",
        "position": {"x": 1, "y": 1}
    });
    let error = definition_error(&missing_rows_with_arrival);
    assert_has(&error, "cells has 0 rows but height is 5");
    assert_has(
        &error,
        "arrivals[\"inside_declared_bounds\"] is out of bounds",
    );

    let mut empty_layers = parts("first_room");
    empty_layers.template_levels_source_mut()["room_0"]["cells"][1][1] = json!([]);
    assert_has(
        &definition_error(&empty_layers),
        "must be a non-empty layer list",
    );

    let mut all_null = parts("first_room");
    all_null.template_levels_source_mut()["room_0"]["cells"][1][1] = json!([null, null]);
    assert_has(
        &definition_error(&all_null),
        "must contain at least one non-null terrain layer",
    );

    let mut unknown_terrain = parts("first_room");
    unknown_terrain.template_levels_source_mut()["room_0"]["cells"][1][1] = json!(["missing"]);
    assert_has(
        &definition_error(&unknown_terrain),
        "references unselected terrain \"missing\"",
    );
}

#[test]
fn realm_level_dimensions_and_required_fields_are_exact() {
    parts("first_room")
        .definition()
        .expect("the canonical single-level terrain contract is valid");
    parts("undercroft_loop")
        .definition()
        .expect("the canonical multi-level terrain contract is valid");

    let mut bad_width = parts("first_room");
    bad_width.template_levels_source_mut()["room_0"]["width"] = json!(99);
    assert_has(
        &definition_error(&bad_width),
        "cells[0] has 5 cells but width is 99",
    );

    let mut bad_height = parts("first_room");
    bad_height.template_levels_source_mut()["room_0"]["height"] = json!(99);
    assert_has(
        &definition_error(&bad_height),
        "cells has 5 rows but height is 99",
    );

    let mut malformed_cells = parts("first_room");
    malformed_cells.template_levels_source_mut()["room_0"]["cells"][0] = json!("wall");
    assert_has(&decode_error(&malformed_cells), "invalid type");

    let mut obsolete_map = parts("first_room");
    obsolete_map.world_template["map"] = json!({});
    assert_has(&decode_error(&obsolete_map), "unknown field `map`");

    for required in ["realms", "arrivals", "topology"] {
        let mut missing = parts("first_room");
        missing
            .world_template
            .as_object_mut()
            .expect("world template object")
            .remove(required);
        let error = decode_error(&missing);
        assert_has(&error, "missing field");
        assert_has(&error, required);
    }

    let mut absent_level = parts("first_room");
    absent_level.world_template["realms"]["realm_0"]["levels"] = json!({});
    assert_has(&definition_error(&absent_level), "levels must be non-empty");
}

#[test]
fn typed_topology_targets_directions_and_conflicts_are_strict() {
    parts("utility_door_secret_item_spells")
        .definition()
        .expect("current door and stair topology is valid");

    let mut impassable_target = parts("utility_door_secret_item_spells");
    impassable_target.world_template["topology"]["edge/workroom/1/4"]["target"]["location"]["position"] =
        json!({"x": 0, "y": 0});
    assert_has(
        &definition_error(&impassable_target),
        "target is not traversable",
    );

    let mut cross_realm_position = parts("utility_door_secret_item_spells");
    cross_realm_position.world_template["topology"]["edge/workroom/1/4"]["target"]["location"]["realm"] =
        json!("missing_realm");
    assert_has(
        &definition_error(&cross_realm_position),
        "must use an arrival for a cross-realm destination",
    );

    let mut missing_arrival = parts("utility_door_secret_item_spells");
    missing_arrival.world_template["topology"]["edge/workroom/1/4"]["target"] =
        json!({"kind": "arrival", "arrival_id": "missing"});
    assert_has(
        &definition_error(&missing_arrival),
        "target.arrival_id \"missing\" does not exist",
    );

    let mut missing_direction = parts("utility_door_secret_item_spells");
    missing_direction.world_template["topology"]["edge/workroom/1/3"]["kind"]
        .as_object_mut()
        .expect("stairs kind")
        .remove("direction");
    assert_has(
        &decode_error(&missing_direction),
        "missing field `direction`",
    );

    for (invalid, expected) in [
        (json!("sideways"), "unknown variant `sideways`"),
        (json!(1), "invalid type: integer"),
    ] {
        let mut bad_direction = parts("utility_door_secret_item_spells");
        bad_direction.world_template["topology"]["edge/workroom/1/3"]["kind"]["direction"] =
            invalid;
        assert_has(&decode_error(&bad_direction), expected);
    }

    let mut direction_on_door = parts("utility_door_secret_item_spells");
    direction_on_door.world_template["topology"]["edge/workroom/1/4"]["kind"]["direction"] =
        json!("down");
    assert_has(
        &decode_error(&direction_on_door),
        "unknown field `direction`",
    );

    let mut conflicting = parts("utility_door_secret_item_spells");
    conflicting.world_template["topology"]["edge/duplicate"] =
        conflicting.world_template["topology"]["edge/workroom/1/4"].clone();
    assert_has(
        &definition_error(&conflicting),
        "conflicts with automatic topology edge",
    );
}

#[test]
fn terrain_registry_navigation_and_layer_composition_are_strict() {
    let mut passable_zero = parts("first_room");
    passable_zero.selected_by_runtime_id_mut("terrains", "flagstone")["navigation"]["move_cost"] =
        json!(0);
    assert_has(
        &definition_error(&passable_zero),
        "navigation.move_cost must be positive",
    );

    let mut boolean_cost = parts("first_room");
    boolean_cost.selected_by_runtime_id_mut("terrains", "flagstone")["navigation"]["move_cost"] =
        json!(true);
    assert_has(&decode_error(&boolean_cost), "invalid type: boolean");

    let mut blocked_extra = parts("first_room");
    blocked_extra.selected_by_runtime_id_mut("terrains", "stone_wall")["navigation"]["move_cost"] =
        json!(1);
    assert_has(&decode_error(&blocked_extra), "unknown field `move_cost`");

    let mut nullable_layer = parts("first_room");
    nullable_layer.template_levels_source_mut()["room_0"]["cells"][1][1] =
        json!([null, "flagstone", null]);
    nullable_layer
        .validated_seed()
        .expect("null layers are allowed when composition remains traversable");

    let mut blocked_composition = parts("first_room");
    blocked_composition.template_levels_source_mut()["room_0"]["cells"][1][1] =
        json!(["flagstone", "stone_wall"]);
    assert_has(
        &seed_error(&blocked_composition),
        "actors[0].location is not traversable",
    );
}

#[test]
fn actor_world_positions_are_checked_against_selected_layers() {
    let mut actor_on_wall = parts("first_room");
    actor_on_wall.actors_mut()[0]["location"]["position"] = json!({"x": 0, "y": 0});
    assert_has(
        &seed_error(&actor_on_wall),
        "actors[0].location is not traversable at realm_0/room_0:0,0",
    );

    let mut actor_in_named_level = parts("undercroft_loop");
    actor_in_named_level.actors_mut()[0]["location"]["level"] = json!("guard_post");
    actor_in_named_level.actors_mut()[0]["location"]["position"] = json!({"x": 0, "y": 0});
    assert_has(
        &seed_error(&actor_in_named_level),
        "actors[0].location is not traversable at realm_0/guard_post:0,0",
    );
}

#[test]
fn terrain_overlay_value_matrix_is_preserved() {
    let invalid_enums: [(&str, Value); 3] = [
        ("passability", json!("sticky")),
        ("sight", json!("foggy")),
        ("hazard", json!("acid")),
    ];
    for (field, value) in invalid_enums {
        let mut invalid = parts("area_path_terrain_spells");
        invalid.selected_by_runtime_id_mut("spells", "web_field")["effect"]["terrain_overlay"]
            [field] = value;
        let error = definition_error(&invalid);
        assert_has(&error, field);
    }

    let mut zero_move_cost = parts("area_path_terrain_spells");
    zero_move_cost.selected_by_runtime_id_mut("spells", "web_field")["effect"]["terrain_overlay"]
        ["move_cost"] = json!(0);
    assert_has(
        &definition_error(&zero_move_cost),
        "terrain_overlay.move_cost must be positive",
    );
}
