extends SceneTree

## Drives the real client scene against a real server.
##
## This is an on-demand proof tool, not part of the standing suite: it needs a
## live PostgreSQL-backed server, a TLS front, and credentials, all of which
## `tools/run_client_live_proof.py` provisions before invoking it. It mounts
## `ClientRoot.tscn` — the shipped scene, the real transport, the real codec —
## and walks sign-in, character selection, admission, authoritative play, and
## sign-out, printing one fact per step.
##
## Everything it asserts, it asserts against state the client actually holds.
## Nothing here reconstructs the flow: if a step is skipped or a response is
## invented, the following step has nothing to stand on and the run fails.
##
## The mount-and-admit sequence is shared with the capture proof through
## `res://tests/live_session.gd`; the claims below are this proof's own.

const SUCCESS_SENTINEL: String = "TME_CLIENT_LIVE_PROOF_OK"
const STEP_TIMEOUT_MSEC: int = 30000
const UPDATE_WAIT_MSEC: int = 25000

## Actions are sampled at different offsets after their previous cooldown.

const LiveSession: Script = preload("res://tests/live_session.gd")

var _session: RefCounted
var _client: ClientRoot
var _failures: Array[String] = []


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	await process_frame
	var credentials: Dictionary = LiveSession.credentials()
	if credentials.is_empty():
		_fail("the proof requires TME_EX_USERNAME, TME_EX_PASSWORD, and TME_EX_CHARACTER_ID")
		_finish()
		return

	_session = LiveSession.new(self)
	_client = await _session.mount()
	_report("endpoint", _client.facade.endpoint.https_base_url)

	if not await _session.sign_in(credentials["username"], credentials["password"]):
		_fail("sign-in left the client %s rather than bootstrapped" % _client.state_machine.current)
		_finish()
		return
	print("ok: sign-in leaves the client %s" % ConnectionStateMachine.BOOTSTRAPPED)
	var characters: Array = _client.control_state.bootstrap.get("characters", [])
	_expect(not characters.is_empty(), "the session bootstrap carries at least one character")
	_expect(
		not _client.control_state.bootstrap.has("facets"),
		"the session bootstrap carries no selectable world directory",
	)
	_report("account", str(_client.control_state.bootstrap.get("account", {}).get("display_name", "")))
	_report("characters", str(characters.size()))
	if not _failures.is_empty():
		_finish()
		return

	if not await _session.select_character(credentials["character_id"]):
		_fail("admission never reached online authority; last state %s" % _client.state_machine.current)
		_finish()
		return

	_report("lifecycle", _client.state_machine.current)
	_report("actor", _client.authoritative_state.actor_id())
	_report("connection", _client.authoritative_state.connection_id())
	_report("welcome_world_revision", _client.authoritative_state.world_revision())
	var view: GridWorldView = _client.world_screen.world_view as GridWorldView
	_expect(view != null, "the world shell presents through the neutral view seam")
	if view != null:
		var status: String = view.status_text()
		_expect(status.contains("Observation centre"), "the shell presents an observation centre")
		_expect(status.contains("You: "), "the shell presents the controlled character")
		_expect(not view.targets().is_empty(), "the presented frame carries addressable targets")
		_report("shell", status)
		_report("addressable_targets", str(view.targets().size()))
	_expect(
		_client.world_screen.connection_status.text.contains("ONLINE"),
		"the HUD reports the online lifecycle",
	)

	await _observe_cooldowns()

	var feedback_before: int = _client.presentation_state.feedback_presenter.feedback_entries.size()
	_client._on_intent_requested({"kind": "wait"})
	_expect(_client.control_state.has_pending_command(), "the command installs one pending record")
	var settled: bool = await _session.wait_until(func() -> bool:
		return not _client.control_state.has_pending_command()
	, STEP_TIMEOUT_MSEC)
	_expect(settled, "the authoritative command result settles the pending command")
	_report("command_feedback_entries", str(
		_client.presentation_state.feedback_presenter.feedback_entries.size() - feedback_before
	))
	_expect_lifecycle(ConnectionStateMachine.ONLINE, "the command round trip")

	_client._on_logout_requested()
	_expect_lifecycle(ConnectionStateMachine.SIGNED_OUT, "sign-out")
	_expect(_client.login_screen.visible, "sign-out returns to the sign-in screen")
	_expect(not _client.facade.socket_is_open(), "sign-out leaves no open socket")
	_finish()


## Observes consecutive authoritative beats while the client sits idle.
##
## Each observation is the frame's own `logical_time` — the authority's count of
## rounds — paired with the wall-clock millisecond the client installed it. The
## client holds no cadence of its own and infers nothing from elapsed time: it
## reports what arrived and when, and `tools/run_client_live_proof.py` judges the
## interval against the ruled pulse.
func _observe_cooldowns() -> void:
	for offset: int in [137, 1173, 2511]:
		await create_timer(float(offset) / 1000.0).timeout
		var began: int = Time.get_ticks_msec()
		_client._on_intent_requested({"kind": "wait"})
		var accepted: bool = await _session.wait_until(func() -> bool:
			return not _client.control_state.has_pending_command() and not bool(_client.authoritative_state.frame().get("can_act", true))
		, STEP_TIMEOUT_MSEC)
		_expect(accepted, "an offset action starts its own cooldown")
		if not accepted:
			return
		var frame: Dictionary = _client.authoritative_state.frame()
		var started: String = str(frame.get("logical_time", ""))
		var ready: String = str(frame.get("ready_at", ""))
		var finished: bool = await _session.wait_until(func() -> bool:
			return bool(_client.authoritative_state.frame().get("can_act", false))
		, STEP_TIMEOUT_MSEC)
		_expect(finished, "the server unlocks the completed action")
		_report("cooldown_observation", "start %s ready %s elapsed %d ms" % [started, ready, Time.get_ticks_msec() - began])


func _logical_time() -> String:
	return str(_client.authoritative_state.frame().get("logical_time", ""))


func _expect_lifecycle(expected: String, step: String) -> void:
	var current: String = _client.state_machine.current
	if current == expected:
		print("ok: %s leaves the client %s" % [step, expected])
	else:
		_fail("%s left the client %s rather than %s" % [step, current, expected])


func _expect(condition: bool, message: String) -> void:
	if condition:
		print("ok: " + message)
	else:
		_fail(message)


func _fail(message: String) -> void:
	_failures.append(message)
	printerr("FAIL: " + message)


func _report(name: String, value: String) -> void:
	print("%s = %s" % [name, value])


func _finish() -> void:
	if _session != null:
		_session.release()
	if _failures.is_empty():
		print(SUCCESS_SENTINEL)
		quit(0)
	else:
		printerr("%d proof step(s) failed" % _failures.size())
		quit(1)
