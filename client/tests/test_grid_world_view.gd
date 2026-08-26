extends RefCounted

## [GridWorldView] and the capture it can produce.
##
## The claim under test is not "the view draws something". It is that the view's
## geometry and its answers are the same fact: a square becomes a rectangle,
## that rectangle resolves a pointer back to the same target, and the identity
## raster a capture writes indexes exactly those rectangles. Anything that can
## drift between those three would put a wrong address in a selection packet.

const VIEW_SIZE: Vector2 = Vector2(800.0, 600.0)

var _support: TestSupport


func test_the_seam_default_never_invents_world_facts() -> void:
	var seam: WorldViewSeam = WorldViewSeam.new()
	seam.present_frame(_frame(), 1)
	_support.expect_equal(seam.observation_center(), Vector2i.ZERO, "the seam default presents no observation")
	_support.expect(seam.semantic_target_for_coordinate(Vector2i.ZERO).is_empty(), "the seam default resolves no square")
	_support.expect(seam.semantic_target_for_display_position(Vector2.ZERO).is_empty(), "the seam default resolves no pointer position")
	_support.expect_equal(seam.pointer_surface(), seam, "pointer positions default to the seam's own space")
	_support.expect(seam.present_feedback("physical_combat").is_empty(), "the seam default presents no feedback")
	seam.free()


func test_the_frame_is_laid_out_as_one_rectangle_per_square() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 3)
	var layout: Dictionary = view.layout()
	_support.expect(not layout.is_empty(), "a presented frame has a lattice layout")
	var pitch: int = int(layout["pitch"])
	_support.expect(pitch >= GridWorldView.MINIMUM_PITCH, "the lattice never draws below the minimum pitch")
	var bounds: Dictionary = layout["bounds"]
	_support.expect_equal(bounds["columns"], 3, "the lattice spans the frame's squares in x")
	_support.expect_equal(bounds["rows"], 2, "the lattice spans the frame's squares in y")

	var squares: Array[Dictionary] = _by_layer(view, GridWorldView.LAYER_SQUARES)
	_support.expect_equal(squares.size(), 6, "every square in the frame is drawn once")
	for record: Dictionary in squares:
		var rect: Rect2i = record["rect"]
		_support.expect_equal(rect.size, Vector2i(pitch, pitch), "a square's rectangle is one pitch square")
	_support.expect(
		view.status_text().contains("Observation centre 1,0"),
		"the status line names the observation centre",
	)


func test_pointer_resolution_round_trips_through_the_drawn_rectangles() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 5)
	for record: Dictionary in view.screen_targets():
		var expected: Dictionary = record["target"]
		var anchor: Vector2i = record["anchor"]
		var resolved: Dictionary = view.semantic_target_for_display_position(Vector2(anchor))
		_support.expect_equal(
			resolved.get("identity"),
			expected.get("identity"),
			"the anchor of %s resolves back to it" % str(expected.get("identity")),
		)
		_support.expect_equal(resolved.get("generation"), 5, "a resolved target carries the presented generation")
	_support.expect(
		view.semantic_target_for_display_position(Vector2(-10.0, -10.0)).is_empty(),
		"a pointer outside the lattice resolves to nothing rather than the nearest square",
	)
	var target: Dictionary = view.semantic_target_for_coordinate(Vector2i(1, 0))
	_support.expect_equal(target.get("identity"), "tile:1:0", "a square in the frame resolves to its tile target")
	_support.expect(view.semantic_target_for_coordinate(Vector2i(9, 9)).is_empty(), "a square outside the frame resolves to nothing")
	view.free()


func test_occupants_share_a_square_without_overlapping() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 1)
	var occupants: Array[Dictionary] = _by_layer(view, GridWorldView.LAYER_OCCUPANTS)
	_support.expect_equal(occupants.size(), 3, "each addressable occupant is drawn once")
	for outer: int in occupants.size():
		for inner: int in range(outer + 1, occupants.size()):
			var left: Rect2i = occupants[outer]["rect"]
			var right: Rect2i = occupants[inner]["rect"]
			_support.expect(
				not left.intersects(right),
				"no two addressable rectangles overlap, so no pixel has two owners",
			)
	var controlled: Array[Dictionary] = occupants.filter(func(row: Dictionary) -> bool:
		return bool(row["controlled"])
	)
	_support.expect_equal(controlled.size(), 1, "exactly one occupant is the controlled character")
	_support.expect_equal(
		(controlled[0]["target"] as Dictionary).get("source_identity"),
		"player",
		"the controlled occupant is the observer's own actor",
	)
	view.free()


func test_the_identity_raster_indexes_the_same_targets_the_pointer_resolves() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 2)
	var targets: Array[Dictionary] = CaptureEmitter.target_records(view.screen_targets(), Vector2i.ZERO)
	var width: int = int(VIEW_SIZE.x)
	var height: int = int(VIEW_SIZE.y)
	var samples: PackedByteArray = CaptureEmitter.raster_samples(width, height, targets)
	_support.expect_equal(samples.size(), width * height * 2, "the raster carries one 16-bit sample per pixel")

	for record: Dictionary in targets:
		var anchor: Dictionary = record["anchor"]
		var index: int = CaptureEmitter.raster_index_at(samples, width, int(anchor["x"]), int(anchor["y"]))
		_support.expect_equal(
			index,
			record["index"],
			"the raster names %s at its own anchor" % str(record["identity"]),
		)
		var resolved: Dictionary = view.semantic_target_for_display_position(
			Vector2(float(anchor["x"]), float(anchor["y"]))
		)
		_support.expect_equal(
			resolved.get("identity"),
			record["identity"],
			"the raster and the pointer agree at %s" % str(record["identity"]),
		)
	_support.expect_equal(
		CaptureEmitter.raster_index_at(samples, width, width - 1, height - 1),
		0,
		"a pixel no target occupies indexes nothing",
	)
	view.free()


func test_an_empty_frame_presents_as_absence_not_as_stale_world() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 1)
	view.show_pending(Vector2i(1, 0), ["north", "east"])
	_support.expect("Movement draft from 1,0 · north → east" in view.interaction_label.text, "a drafted path is presented")
	view.show_preview({"pace": "run", "accepted_steps": "1", "requested_path": ["north", "east"], "stop_reason": "blocked"})
	_support.expect("run pace · 1 of 2 steps accepted · blocked" in view.interaction_label.text, "an authoritative preview replaces the draft text")
	view.clear_interaction()
	_support.expect_equal(view.interaction_label.text, "Movement: no draft", "clearing the interaction clears its text")

	view.present_frame({}, 2)
	_support.expect("No authoritative frame" in view.status_label.text, "an empty frame presents absence")
	_support.expect(view.targets().is_empty(), "an empty frame addresses nothing")
	_support.expect(view.screen_targets().is_empty(), "an empty frame draws nothing addressable")
	_support.expect(view.layout().is_empty(), "an empty frame has no lattice")
	_support.expect(
		view.semantic_target_for_display_position(Vector2(400.0, 300.0)).is_empty(),
		"a view with no authority resolves no pointer position",
	)
	view.free()


func test_the_reach_grid_is_a_real_preference() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 1)
	_support.expect_equal(view.grid_control_state(), GridWorldView.GRID_STATE_ON, "the reach grid starts presented")
	_support.expect(view.reach_grid_visible(), "the reach grid is visible while preferred")
	view.toggle_grid()
	_support.expect_equal(view.grid_control_state(), GridWorldView.GRID_STATE_OFF, "toggling turns the preference off")
	_support.expect(not view.reach_grid_visible(), "an unpreferred reach grid is not drawn")
	view.set_reach_grid_transient_active(true)
	_support.expect(view.reach_grid_visible(), "a press shows the reach grid transiently")
	view.set_reach_grid_transient_active(false)
	_support.expect(not view.reach_grid_visible(), "releasing the press hides it again")
	view.toggle_grid()
	_support.expect_equal(view.grid_control_state(), GridWorldView.GRID_STATE_ON, "toggling again restores the preference")
	view.free()


func test_feedback_cues_return_a_truthful_receipt() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 1)
	_support.expect_equal(
		view.present_feedback("physical_combat"),
		{"presented": "lattice_flash", "kind": "physical_combat"},
		"a presented cue reports how it was presented",
	)
	_support.expect(view.present_feedback("").is_empty(), "an empty cue presents nothing")
	_support.expect_equal(view.presented_feedback(), ["physical_combat"], "presented cues are recorded once each")
	view.clear()
	_support.expect(view.presented_feedback().is_empty(), "clearing drops the cue record")
	view.free()


func test_a_capture_refuses_rather_than_writing_a_blank_picture() -> void:
	## The standing suite runs headless, where Godot's display driver produces
	## no viewport image at all. The emitter must say so rather than write a
	## capture whose pixels show nothing and whose sidecar claims they do.
	var view: GridWorldView = _add_view()
	view.present_frame(_frame(), 1)
	var report: Dictionary = CaptureEmitter.emit(
		view, (Engine.get_main_loop() as SceneTree).root, _scratch_directory(), "fixture_frame"
	)
	_support.expect(not bool(report.get("ok", true)), "a headless capture refuses")
	_support.expect(
		str(report.get("error", "")).contains("headless"),
		"the refusal names the reason rather than a generic failure",
	)
	view.free()


func test_a_capture_of_nothing_refuses_before_it_touches_the_filesystem() -> void:
	var view: GridWorldView = _add_view()
	var report: Dictionary = CaptureEmitter.emit(
		view, (Engine.get_main_loop() as SceneTree).root, _scratch_directory(), "fixture_frame"
	)
	_support.expect(not bool(report.get("ok", true)), "a view with no frame cannot be captured")
	_support.expect(
		str(report.get("error", "")).contains("no authoritative frame"),
		"the refusal names the missing frame",
	)
	view.free()


func test_a_step_is_presented_across_its_beat_rather_than_snapped() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(0, 1)), 1)
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(1, 1)), 2)
	var landing: Rect2i = _occupant_rect(view, "actor:other")

	var steps: Dictionary = view.animated_steps()
	_support.expect_equal(steps.size(), 1, "only the marker that moved is animated")
	_support.expect_equal(steps.get("actor:other"), Vector2i(-1, 0), "its travel is the square it came from")

	view.present_pulse(_pulse(0.0))
	var pitch: int = int(view.layout()["pitch"])
	_support.expect_equal(
		_occupant_rect(view, "actor:other").position,
		landing.position - Vector2i(pitch, 0),
		"at the start of the beat the marker is still drawn where it was",
	)
	view.present_pulse(_pulse(0.5))
	_support.expect_equal(
		_occupant_rect(view, "actor:other").position,
		landing.position - Vector2i(pitch / 2, 0),
		"half a beat is half the step",
	)
	view.present_pulse(_pulse(1.0))
	_support.expect_equal(
		_occupant_rect(view, "actor:other").position,
		landing.position,
		"the step completes within one beat and lands exactly on the authoritative square",
	)
	view.free()


func test_a_marker_mid_step_resolves_a_pointer_where_it_is_drawn() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(0, 1)), 1)
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(1, 1)), 2)
	view.present_pulse(_pulse(0.25))
	for record: Dictionary in view.screen_targets():
		var resolved: Dictionary = view.semantic_target_for_display_position(Vector2(record["anchor"] as Vector2i))
		_support.expect_equal(
			resolved.get("identity"),
			(record["target"] as Dictionary).get("identity"),
			"mid-step, %s still resolves at the pixel it occupies" % str((record["target"] as Dictionary).get("identity")),
		)
	view.free()


func test_the_lattice_holds_still_inside_a_beat_and_the_controlled_marker_with_it() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(1, 1)), 1)
	# The observer takes a step: its own square and the observation centre move
	# together, so the character keeps its place in the picture and the world
	# moves past it. The stranger stands inside both observation windows, so it
	# is the same marker before and after rather than one that came into view.
	view.present_frame(_walking_frame(Vector2i(3, 1), Vector2i(1, 1)), 2)
	_support.expect(
		view.animated_steps().has("actor:other"),
		"the marker that stood still is the one that slides, because the observer moved past it",
	)
	_support.expect(
		not view.animated_steps().has("actor:player"),
		"the observer's own marker has nowhere to travel: the lattice re-centres under it",
	)

	var origin: Vector2i = view.layout()["origin"]
	var tile: Rect2i = _target_rect(view, "tile:2:1")
	var controlled: Rect2i = _occupant_rect(view, "actor:player")
	for fill: float in [0.0, 0.5, 1.0]:
		view.present_pulse(_pulse(fill))
		_support.expect_equal(view.layout()["origin"], origin, "the lattice placement a capture reads never moves inside a beat")
		_support.expect_equal(_target_rect(view, "tile:2:1"), tile, "and neither does a square's rectangle")
		_support.expect_equal(
			_occupant_rect(view, "actor:player"),
			controlled,
			"nor the controlled marker, which is what makes the world look like the thing that moved",
		)
	view.free()


func test_a_frame_that_is_not_the_next_one_snaps_instead_of_stepping() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(0, 1)), 4)
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(1, 1)), 9)
	_support.expect(
		view.animated_steps().is_empty(),
		"two frames either side of a discard are not a beat apart and are not animated across one",
	)
	view.present_frame({}, 10)
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(0, 1)), 11)
	_support.expect(view.animated_steps().is_empty(), "the frame after absence of authority has nowhere to come from")
	view.free()


func test_a_displacement_wider_than_a_step_snaps() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(0, 1)), 1)
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(4, 1)), 2)
	_support.expect(
		view.animated_steps().is_empty(),
		"a transition is not a walk and is not drawn as one",
	)
	view.free()


func test_an_unmeasured_beat_animates_nothing() -> void:
	var view: GridWorldView = _add_view()
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(0, 1)), 1)
	view.present_frame(_walking_frame(Vector2i(2, 1), Vector2i(1, 1)), 2)
	var landing: Rect2i = _occupant_rect(view, "actor:other")
	view.present_pulse({"measured": false, "fill": 0.0})
	_support.expect_equal(view.step_motion(), 0.0, "with no measured beat there is no interval to spread a step over")
	_support.expect_equal(_occupant_rect(view, "actor:other").position, landing.position, "so the marker sits on its authoritative square")
	view.free()


func _pulse(fill: float) -> Dictionary:
	return {"measured": true, "fill": fill}


func _target_rect(view: GridWorldView, identity: String) -> Rect2i:
	for record: Dictionary in view.screen_targets():
		if str((record["target"] as Dictionary)["identity"]) == identity:
			return record["rect"]
	return Rect2i()


func _occupant_rect(view: GridWorldView, identity: String) -> Rect2i:
	return _target_rect(view, identity)


## A five-by-three frame carrying the controlled character and one stranger,
## both placed by the caller, so a step can be made by presenting two of them.
func _walking_frame(player: Vector2i, other: Vector2i) -> Dictionary:
	# The observation window travels with the observer, exactly as a real frame's
	# does, so a step actually changes which squares the lattice is laid out
	# around rather than leaving a fixed board behind.
	var tiles: Array = []
	for dy: int in range(-1, 2):
		for dx: int in range(-2, 3):
			tiles.append(_tile(player.x + dx, player.y + dy, "testland_grass", true))
	return {
		"observer_actor_id": "player",
		"observation_center": _position(player.x, player.y),
		"logical_time": "12",
		"ready_at": "12",
		"can_act": true,
		"tiles": tiles,
		"actors": [
			{"actor_id": "player", "name": "Wayfarer", "position": _position(player.x, player.y), "life_state": "alive", "hp": 7, "max_hp": 10},
			{"actor_id": "other", "name": "Stranger", "position": _position(other.x, other.y), "life_state": "alive", "hp": 4, "max_hp": 4},
		],
		"corpses": [],
		"ground_items": [],
		"gold_piles": [],
	}


func _scratch_directory() -> String:
	return ProjectSettings.globalize_path("user://capture-suite-scratch")


func _by_layer(view: GridWorldView, layer: String) -> Array[Dictionary]:
	return view.screen_targets().filter(func(record: Dictionary) -> bool:
		return str(record["layer"]) == layer
	)


func _add_view() -> GridWorldView:
	var view: GridWorldView = (
		load("res://presentation/GridWorldView.tscn") as PackedScene
	).instantiate() as GridWorldView
	(Engine.get_main_loop() as SceneTree).root.add_child(view)
	view.set_anchors_preset(Control.PRESET_TOP_LEFT)
	view.position = Vector2.ZERO
	view.size = VIEW_SIZE
	return view


## A three-by-two frame with the controlled character, a stranger, and a corpse
## sharing one square, so occupant packing and reach both have something to say.
func _frame() -> Dictionary:
	return {
		"observer_actor_id": "player",
		"observation_center": _position(1, 0),
		"logical_time": "12",
		"tiles": [
			_tile(0, 0, "testland_grass", true),
			_tile(1, 0, "testland_grass", true),
			_tile(2, 0, "testland_wall", false),
			_tile(0, 1, "testland_path", true),
			_tile(1, 1, "testland_path", true),
			_tile(2, 1, "testland_deep_water", false),
		],
		"actors": [
			{"actor_id": "player", "name": "Wayfarer", "position": _position(1, 0), "life_state": "alive", "hp": 7, "max_hp": 10},
			{"actor_id": "other", "name": "Stranger", "position": _position(0, 1), "life_state": "alive", "hp": 4, "max_hp": 4},
		],
		"corpses": [{"corpse_id": "corpse:1", "origin_name": "Kobold", "searched": false, "location": _position(1, 0)}],
		"ground_items": [],
		"gold_piles": [],
	}


func _tile(x: int, y: int, terrain_id: String, passable: bool) -> Dictionary:
	return {
		"position": {"x": x, "y": y},
		"terrain_id": terrain_id,
		"terrain_name": terrain_id.capitalize(),
		"passable": passable,
		"move_cost": 1,
		"transition": null,
	}


func _position(x: int, y: int) -> Dictionary:
	return {"realm": "testland", "level": "surface", "position": {"x": x, "y": y}}
