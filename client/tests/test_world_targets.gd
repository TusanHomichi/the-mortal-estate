extends RefCounted

var _support: TestSupport


func test_every_frame_collection_produces_identified_targets_in_stable_order() -> void:
	var targets: Array[Dictionary] = WorldTargets.build(_frame(), 4)
	var identities: Array[String] = []
	for target: Dictionary in targets:
		identities.append(str(target["identity"]))
		_support.expect_equal(target["generation"], 4, str(target["identity"]) + " carries the building generation")
	_support.expect_equal(identities, [
		"actor:other",
		"actor:player",
		"corpse:corpse:1",
		"gold_pile:gold:1",
		"ground_item:item:1",
		"tile:0:0",
		"tile:1:0",
	], "every collection contributes exactly one identified target, ordered by identity")
	_support.expect_equal(WorldTargets.build(_frame(), 4), targets, "two builds of one frame are identical")


func test_target_identity_priority_and_source_rows_are_exact() -> void:
	var targets: Array[Dictionary] = WorldTargets.build(_frame(), 1)
	_support.expect_equal(WorldTargets.semantic_identity("tile", "tile:2:3"), "tile:2:3", "an already-qualified tile identity is not requalified")
	_support.expect_equal(WorldTargets.semantic_identity("actor", "player"), "actor:player", "other kinds are qualified by kind")
	_support.expect_equal(WorldTargets.tile_identity(Vector2i(-2, 5)), "tile:-2:5", "tile identity is its square")
	_support.expect_equal(WorldTargets.topmost_at(targets, Vector2i.ZERO)["kind"], "actor", "an actor outranks the ground state it stands on")
	_support.expect_equal(WorldTargets.tile_at(targets, Vector2i(1, 0))["kind"], "tile", "a square resolves to its own tile regardless of priority")
	_support.expect(WorldTargets.tile_at(targets, Vector2i(9, 9)).is_empty(), "a square outside the frame resolves to nothing")
	_support.expect_equal(WorldTargets.topmost_at(targets, Vector2i.ZERO)["source"]["name"], "Wayfarer", "a target carries its authoritative source row")


func test_ground_reach_is_classified_by_ring_distance_and_fails_closed_without_a_player() -> void:
	var targets: Array[Dictionary] = WorldTargets.build(_frame(), 1)
	var corpse: Dictionary = WorldTargets.topmost_at(targets, Vector2i.ZERO)
	for target: Dictionary in targets:
		if target["identity"] == "corpse:corpse:1":
			corpse = target
	_support.expect_equal(corpse["ground_reach"], WorldTargets.GROUND_REACH_MANIPULATE, "ground state underfoot is manipulable")
	_support.expect(corpse["manipulation_reachable"], "manipulable ground state says so")

	var adjacent: Dictionary = _frame()
	adjacent["corpses"][0]["location"] = _position(1, 0)
	var adjacent_corpse: Dictionary = _target(WorldTargets.build(adjacent, 1), "corpse:corpse:1")
	_support.expect_equal(adjacent_corpse["ground_reach"], WorldTargets.GROUND_REACH_EXAMINE, "adjacent ground state is examinable only")
	_support.expect(not adjacent_corpse["manipulation_reachable"], "adjacent ground state is not manipulable")

	var distant: Dictionary = _frame()
	distant["corpses"][0]["location"] = _position(2, 0)
	_support.expect(_target(WorldTargets.build(distant, 1), "corpse:corpse:1").is_empty(), "ground state beyond one square is not addressable at all")

	var no_player: Dictionary = _frame()
	no_player["observer_actor_id"] = "absent"
	var no_player_targets: Array[Dictionary] = WorldTargets.build(no_player, 1)
	_support.expect(_target(no_player_targets, "corpse:corpse:1").is_empty(), "no known player square means no reachable ground state")
	_support.expect(not _target(no_player_targets, "tile:0:0").is_empty(), "squares remain addressable without a player square")
	_support.expect_equal(WorldTargets.controlled_player_coordinate(no_player), null, "an absent observer has no coordinate")


func test_malformed_rows_are_refused_rather_than_rounded() -> void:
	var frame: Dictionary = _frame()
	frame["tiles"].append({"position": {"realm": "synthetic", "level": "surface", "position": {"x": 0.5, "y": 0}}})
	frame["tiles"].append({"position": {"realm": "synthetic", "level": "surface", "position": {"x": "2", "y": 0}}})
	frame["actors"].append({"actor_id": "", "position": _position(3, 0)})
	frame["ground_items"].append({"item_instance_id": "item:1", "name": "Duplicate", "location": _position(0, 0)})
	var identities: Array[String] = []
	for target: Dictionary in WorldTargets.build(frame, 1):
		identities.append(str(target["identity"]))
	_support.expect_equal(identities.count("ground_item:item:1"), 1, "a duplicate identity is admitted once")
	_support.expect(not identities.has("actor:"), "a row without an identity is refused")
	_support.expect_equal(identities.count("tile:0:0"), 1, "a fractional square is refused rather than rounded onto a real one")
	_support.expect(not identities.has("tile:2:0"), "a string coordinate is refused")
	_support.expect_equal(WorldTargets.coordinate_or_null({"x": 1.0, "y": -2.0}), Vector2i(1, -2), "an integral float pair is a square")
	_support.expect_equal(WorldTargets.coordinate_or_null(Vector2(1.5, 0.0)), null, "a fractional vector is refused")
	_support.expect_equal(WorldTargets.coordinate_or_null("north"), null, "a non-position value is refused")
	_support.expect_equal(WorldTargets.coordinate("north"), Vector2i.ZERO, "the lenient form yields the origin for an unusable value")
	_support.expect_equal(WorldTargets.observation_center(_frame()), Vector2i.ZERO, "the observation centre reads through its nested position")


func _target(targets: Array[Dictionary], identity: String) -> Dictionary:
	for target: Dictionary in targets:
		if target["identity"] == identity:
			return target
	return {}


func _frame() -> Dictionary:
	return {
		"observer_actor_id": "player",
		"observation_center": _position(0, 0),
		"tiles": [
			{"position": _position(0, 0), "terrain_id": "town_floor"},
			{"position": _position(1, 0), "terrain_id": "town_floor"},
		],
		"actors": [
			{"actor_id": "player", "name": "Wayfarer", "position": _position(0, 0)},
			{"actor_id": "other", "name": "Stranger", "position": _position(1, 0)},
		],
		"corpses": [{"corpse_id": "corpse:1", "origin_name": "Kobold", "searched": false, "location": _position(0, 0)}],
		"ground_items": [{"item_instance_id": "item:1", "name": "Torch", "quantity": 1, "location": _position(0, 0)}],
		"gold_piles": [{"gold_pile_id": "gold:1", "amount": "12", "location": _position(0, 0)}],
	}


func _position(x: int, y: int) -> Dictionary:
	return {"realm": "synthetic", "level": "surface", "position": {"x": x, "y": y}}
