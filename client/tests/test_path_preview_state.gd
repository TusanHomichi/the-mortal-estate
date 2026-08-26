extends RefCounted

const PREVIEW_A: String = "11111111-1111-4111-8111-111111111111"
const PREVIEW_B: String = "22222222-2222-4222-8222-222222222222"

var _support: TestSupport


func test_request_is_strict_ephemeral_and_does_not_touch_command_or_authority_state() -> void:
	var control: ControlState = ControlState.new()
	var authority: AuthoritativeState = AuthoritativeState.new()
	control.accept_welcome("7")
	authority.accept_welcome(_welcome("4"))
	var generation_before: int = authority.frame_generation
	var state: PathPreviewState = PathPreviewState.new()
	var request: Dictionary = state.begin(["north", "northeast"], control.active_control_epoch, authority.world_revision(), "player", PREVIEW_A)
	_support.expect_equal(request, {
		"kind": "path_preview",
		"preview_id": PREVIEW_A,
		"control_epoch": "7",
		"observed_world_revision": "4",
		"actor_id": "player",
		"path": ["north", "northeast"],
	}, "preview request contains only current Protocol 1.8 wire facts")
	_support.expect(not request.has("client_sequence") and not request.has("command_id"), "preview request has no command correlation")
	_support.expect_equal(control.active_next_sequence(), "1", "preview creation consumes no sequence")
	_support.expect(not control.has_pending_command(), "preview creation installs no pending command")
	_support.expect_equal(authority.frame_generation, generation_before, "preview creation does not mutate authority")


func test_latest_request_wins_and_stale_correlations_are_discarded() -> void:
	var state: PathPreviewState = PathPreviewState.new()
	state.begin(["north"], "7", "4", "player", PREVIEW_A)
	state.begin(["east"], "7", "4", "player", PREVIEW_B)
	_support.expect_equal(state.accept_result(_result(PREVIEW_A, ["north"]), "7", "4", "player"), "discarded", "superseded correlation is discarded")
	_support.expect_equal(state.accept_result(_result(PREVIEW_B, ["east"]), "7", "4", "player"), "accepted", "latest correlation is accepted")
	_support.expect(state.has_preview(), "accepted latest result exposes a preview")
	_support.expect_equal(state.requested_path(), ["east"], "latest requested path remains immutable")


func test_epoch_actor_revision_and_path_mismatches_discard() -> void:
	var mismatches: Array[Dictionary] = [
		{"epoch": "8", "revision": "4", "actor": "player", "path": ["north"]},
		{"epoch": "7", "revision": "5", "actor": "player", "path": ["north"]},
		{"epoch": "7", "revision": "4", "actor": "other", "path": ["north"]},
		{"epoch": "7", "revision": "4", "actor": "player", "path": ["south"]},
	]
	for mismatch: Dictionary in mismatches:
		var state: PathPreviewState = PathPreviewState.new()
		state.begin(["north"], "7", "4", "player", PREVIEW_A)
		var envelope: Dictionary = _result(PREVIEW_A, mismatch["path"])
		envelope["control_epoch"] = mismatch["epoch"]
		envelope["world_revision"] = mismatch["revision"]
		envelope["actor_id"] = mismatch["actor"]
		_support.expect_equal(state.accept_result(envelope, mismatch["epoch"], mismatch["revision"], mismatch["actor"]), "discarded", "mismatched current authority or path is discarded")


func test_local_frame_replacement_does_not_invalidate_current_authority_request() -> void:
	var state: PathPreviewState = PathPreviewState.new()
	state.begin(["north"], "7", "4", "player", PREVIEW_A)
	_support.expect(
		state.request_matches_authority("7", "4", "player"),
		"a local frame-generation change leaves the authority-correlated request current",
	)
	_support.expect_equal(
		state.accept_result(_result(PREVIEW_A, ["north"]), "7", "4", "player"),
		"accepted",
		"a delayed result remains valid when epoch, revision, actor, path, and correlation still match",
	)


func test_rejection_is_current_feedback_and_clear_drops_all_ephemeral_state() -> void:
	var state: PathPreviewState = PathPreviewState.new()
	state.begin(["north"], "7", "4", "player", PREVIEW_A)
	var rejected: Dictionary = _result(PREVIEW_A, ["north"])
	rejected["disposition"] = {"kind": "rejected", "code": "rules_rejected"}
	rejected["preview"] = null
	_support.expect_equal(state.accept_result(rejected, "7", "4", "player"), "accepted", "current finite rejection is accepted as feedback")
	_support.expect(not state.has_preview(), "rejection never becomes a movement preview")
	_support.expect_equal(state.current_result()["disposition"]["code"], "rules_rejected", "finite rejection remains exact")
	state.clear()
	_support.expect_equal(state.debug_summary(), {"has_request": false, "has_preview": false, "preview_id": "", "requested_steps": 0}, "clear drops every ephemeral preview fact")


func _welcome(revision: String) -> Dictionary:
	return {"kind": "server_welcome", "connection_id": "55555555-5555-4555-8555-555555555555", "server_sequence": "0", "world_revision": revision, "static_scene_context": {"context": "fixture"}, "frame": {"observer_actor_id": "player"}}


func _result(preview_id: String, path: Array) -> Dictionary:
	return {
		"kind": "path_preview_result",
		"preview_id": preview_id,
		"disposition": {"kind": "previewed"},
		"control_epoch": "7",
		"actor_id": "player",
		"world_revision": "4",
		"preview": {"requested_path": path.duplicate()},
	}
