extends RefCounted

var _support: TestSupport


func test_authoritative_reducer_replaces_complete_frame_atomically() -> void:
	var state: AuthoritativeState = AuthoritativeState.new()
	_support.expect(state.accept_welcome(_welcome("1", "1", {"actors": ["old"], "tiles": ["old"]})), "welcome must install authority")
	_support.expect_equal(state.frame(), {"actors": ["old"], "tiles": ["old"]}, "welcome frame is complete")
	var update: Dictionary = _update("2", "2", {"actors": ["new"], "corpses": []})
	_support.expect_equal(state.apply_state_update(update), "replaced", "newer update replaces")
	_support.expect_equal(state.frame(), {"actors": ["new"], "corpses": []}, "replacement removes stale fields")
	_support.expect(not state.frame().has("tiles"), "result and update logic never patches old fields")
	_support.expect_equal(
		state.presentation_frame().get("static_scene_context"),
		state.static_scene_context(),
		"presentation frame carries the entity-free static scene context",
	)


func test_update_sequence_duplicate_lower_and_conflict_rules() -> void:
	var state: AuthoritativeState = AuthoritativeState.new()
	state.accept_welcome(_welcome("5", "9", {"value": "current"}))
	_support.expect_equal(state.apply_state_update(_update("5", "9", {"value": "current"})), "duplicate", "exact duplicate is a no-op")
	_support.expect_equal(state.apply_state_update(_update("4", "8", {"value": "old"})), "rejected", "lower sequence rejects")
	_support.expect_equal(state.apply_state_update(_update("5", "10", {"value": "conflict"})), "rejected", "equal sequence conflict rejects")
	_support.expect_equal(state.apply_state_update(_update("8", "10", {"value": "new"})), "replaced", "noncontiguous newer sequence replaces")


func test_result_update_overtaking_never_patches_frame() -> void:
	var state: AuthoritativeState = AuthoritativeState.new()
	state.accept_welcome(_welcome("1", "1", {"value": "before"}))
	_support.expect_equal(state.apply_state_update(_update("3", "2", {"value": "after"})), "replaced", "update may overtake result")
	_support.expect_equal(state.frame()["value"], "after", "only the accepted complete update changes frame authority")


func test_reconnect_discards_authority_until_fresh_welcome() -> void:
	var state: AuthoritativeState = AuthoritativeState.new()
	state.accept_welcome(_welcome("1", "1", {"value": "before"}))
	_support.expect_equal(state.actor_id(), "player", "welcome retains accepted actor identity")
	_support.expect(state.ordinary_input_enabled(), "fresh welcome enables input")
	state.discard_for_reconnect()
	_support.expect(not state.has_authority(), "reconnect clears cached authority")
	_support.expect_equal(state.actor_id(), "", "reconnect clears accepted actor identity")
	_support.expect(not state.ordinary_input_enabled(), "ordinary input remains disabled")
	_support.expect_equal(state.apply_state_update(_update("2", "2", {"value": "not_welcome"})), "rejected", "update cannot replace the required welcome")
	state.accept_welcome(_welcome("1", "1", {"value": "fresh"}))
	_support.expect(state.ordinary_input_enabled(), "fresh full welcome resumes input")


func test_revision_coverage_uses_checked_decimal_ordering() -> void:
	var state: AuthoritativeState = AuthoritativeState.new()
	state.accept_welcome(_welcome("1", "9007199254740993", {"value": "wide"}))
	_support.expect(state.world_revision_at_least("9007199254740992"), "wide revision compares without float coercion")
	_support.expect(state.world_revision_at_least("9007199254740993"), "equal revision is covered")
	_support.expect(not state.world_revision_at_least("9007199254740994"), "newer revision is not covered")
	_support.expect(not state.world_revision_at_least("09"), "noncanonical required revision is rejected")
	_support.expect(not state.world_revision_at_least("18446744073709551616"), "overflowing required revision is rejected")


func _welcome(sequence: String, revision: String, frame: Dictionary) -> Dictionary:
	return {"kind": "server_welcome", "connection_id": "connection", "actor_id": "player", "server_sequence": sequence, "world_revision": revision, "static_scene_context": _context(), "frame": frame}


func _update(sequence: String, revision: String, frame: Dictionary) -> Dictionary:
	return {"kind": "state_update", "server_sequence": sequence, "world_revision": revision, "static_scene_context": _context(), "frame": frame}


func _context() -> Dictionary:
	return {"contract_version": 1, "site": {"realm": "realm", "level": "level"}, "bounds": {"min": {"x": 0, "y": 0}, "max": {"x": 0, "y": 0}}, "content_digest": "a".repeat(64), "visual_manifest_digest": "b".repeat(64), "scene_role": "combat_space", "presentation_mode": "combat_space", "world_zoom": [156, 104], "tiles": [{"position": {"x": 0, "y": 0}, "terrain_ids": ["stone"], "walkable": true}], "walkable_mask": [{"x": 0, "y": 0}], "static_props": [], "transition_apertures": []}
