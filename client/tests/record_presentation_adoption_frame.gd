extends SceneTree

## Records one representative frame from the shipped client and real server.
##
## The fixture is proof input, not gameplay authority. This recorder does not
## reshape the frame: it waits until the server-supplied facts satisfy the
## fixture's semantic barrier, then writes the exact frame held by GridWorldView.

const SUCCESS_SENTINEL: String = "TME_PRESENTATION_ADOPTION_FRAME_OK"
const FRAME_FIXTURE_KIND: String = "capture_frame_fixture"
const TIMEOUT_MSEC: int = 20_000
const LiveSession: Script = preload("res://tests/live_session.gd")

var _session: RefCounted


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	await process_frame
	var output: String = OS.get_environment("TME_PRESENTATION_FRAME_OUT").strip_edges()
	var fixture_path: String = OS.get_environment("TME_PRESENTATION_FIXTURE").strip_edges()
	if output.is_empty() or fixture_path.is_empty():
		_fail("TME_PRESENTATION_FRAME_OUT and TME_PRESENTATION_FIXTURE are required")
		return
	var fixture: Variant = _read_json(fixture_path)
	if fixture is not Dictionary:
		_fail("the presentation recording fixture is not a JSON object")
		return
	var barrier: Variant = (fixture as Dictionary).get("capture_barrier")
	if barrier is not Dictionary:
		_fail("the presentation recording fixture carries no capture barrier")
		return
	var credentials: Dictionary = LiveSession.credentials()
	if credentials.is_empty():
		_fail("the recorder requires the live proof credentials")
		return

	_session = LiveSession.new(self)
	var client: ClientRoot = await _session.mount()
	if not await _session.sign_in(credentials["username"], credentials["password"]):
		_fail("sign-in never reached the session bootstrap")
		return
	if not await _session.select_character(credentials["character_id"]):
		_fail("admission never reached online authority")
		return

	var view: GridWorldView = client.world_screen.world_view as GridWorldView
	if view == null:
		_fail("the shipped world shell is not presenting through GridWorldView")
		return
	var started: int = Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < TIMEOUT_MSEC:
		var frame: Dictionary = view.frame()
		if _matches_barrier(frame, barrier as Dictionary):
			if not _record(frame, view.frame_generation(), client, barrier as Dictionary, output):
				_fail("could not write the authoritative frame")
				return
			print("logical_time = %s" % str(frame.get("logical_time", "")))
			print("actors = %d" % (frame.get("actors", []) as Array).size())
			client._on_logout_requested()
			_session.release()
			print(SUCCESS_SENTINEL)
			quit(0)
			return
		await process_frame
	_fail("no authoritative frame satisfied the semantic barrier within %d ms" % TIMEOUT_MSEC)


func _matches_barrier(frame: Dictionary, barrier: Dictionary) -> bool:
	if int(frame.get("logical_time", -1)) < int(barrier.get("minimum_logical_time", -1)):
		return false
	if str(frame.get("observer_actor_id", "")) != str(barrier.get("observer_actor_id", "")):
		return false
	var actors: Array = frame.get("actors", []) as Array
	if actors.size() != int(barrier.get("actor_count", -1)):
		return false
	for expected_value: Variant in barrier.get("actors", []) as Array:
		var expected: Dictionary = expected_value as Dictionary
		var actor: Dictionary = _find(actors, "actor_id", str(expected.get("actor_id", "")))
		if actor.is_empty():
			return false
		for field: String in ["attack_safety", "kind", "life_state", "position"]:
			if actor.get(field) != expected.get(field):
				return false

	var expected_action: Dictionary = barrier.get("action_option", {}) as Dictionary
	var action: Dictionary = _find(
		frame.get("action_options", []) as Array,
		"id",
		str(expected_action.get("id", "")),
	)
	if action.is_empty():
		return false
	for field: String in ["blocked_reason", "enabled", "label"]:
		if action.get(field) != expected_action.get(field):
			return false

	var expected_npc: Dictionary = barrier.get("npc_interaction", {}) as Dictionary
	var npc: Dictionary = _find(
		frame.get("npcs_here", []) as Array,
		"actor_id",
		str(expected_npc.get("actor_id", "")),
	)
	if npc.is_empty() or _find(
		npc.get("interactions", []) as Array,
		"interaction_id",
		str(expected_npc.get("interaction_id", "")),
	).is_empty():
		return false

	var expected_service: Dictionary = barrier.get("service", {}) as Dictionary
	var service: Dictionary = _find(
		frame.get("services_here", []) as Array,
		"service_id",
		str(expected_service.get("service_id", "")),
	)
	if service.is_empty() or service.get("position") != expected_service.get("position"):
		return false
	if _find(
		service.get("capabilities", []) as Array,
		"capability_id",
		str(expected_service.get("capability_id", "")),
	).is_empty():
		return false

	var terrain_ids: Dictionary = {}
	var context: Dictionary = frame.get("static_scene_context", {}) as Dictionary
	for tile_value: Variant in context.get("tiles", []) as Array:
		var tile: Dictionary = tile_value as Dictionary
		for terrain_id: Variant in tile.get("terrain_ids", []) as Array:
			terrain_ids[str(terrain_id)] = true
	for required: Variant in barrier.get("required_terrain_ids", []) as Array:
		if not terrain_ids.has(str(required)):
			return false
	return true


func _find(rows: Array, field: String, value: String) -> Dictionary:
	for row_value: Variant in rows:
		if row_value is Dictionary:
			var row: Dictionary = row_value as Dictionary
			if str(row.get(field, "")) == value:
				return row
	return {}


func _record(
		frame: Dictionary,
		frame_generation: int,
		client: ClientRoot,
		barrier: Dictionary,
		path: String,
) -> bool:
	var document: Dictionary = {
		"schema_version": 1,
		"kind": FRAME_FIXTURE_KIND,
		"provenance": {
			"route": "headless_live_server",
			"recorded_by": "client/tests/record_presentation_adoption_frame.gd",
			"driver": "tools/run_presentation_adoption_recording.py",
			"source_commit": OS.get_environment("TME_PRESENTATION_SOURCE_COMMIT"),
			"source_tree": OS.get_environment("TME_PRESENTATION_SOURCE_TREE"),
			"world_revision": client.authoritative_state.world_revision(),
			"server_sequence": client.authoritative_state.server_sequence(),
			"semantic_barrier": barrier,
		},
		"frame_generation": frame_generation,
		"frame": frame,
	}
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(document, "  ", true) + "\n")
	file.close()
	return true


func _read_json(path: String) -> Variant:
	var file: FileAccess = FileAccess.open(path, FileAccess.READ)
	if file == null:
		return null
	return JSON.parse_string(file.get_as_text())


func _fail(message: String) -> void:
	printerr("presentation-adoption recording refused: " + message)
	if _session != null:
		_session.release()
	quit(1)
