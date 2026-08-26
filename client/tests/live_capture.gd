extends SceneTree

## The accuracy reference: a capture of a real frame from a real server.
##
## It mounts the shipped client, signs in, is admitted, waits for the world view
## to be presenting an authoritative frame, and then writes the capture, the
## identity raster, and the sidecar through [CaptureEmitter] — from the same
## [GridWorldView] the player would be looking at, mounted in the same shell.
##
## It also records the frame it received, verbatim, so the ordinary capture
## route can replay a real server frame instead of a synthesised one. That
## recording is the only way the tracked frame fixture is ever produced.
##
## Provisioned and driven by `tools/run_fixture_land_capture.py`. A real display
## is required; the driver supplies one.

const SUCCESS_SENTINEL: String = "TME_CAPTURE_OK"
const FRAME_FIXTURE_KIND: String = "capture_frame_fixture"
const ROUTE: String = "live"
const SETTLE_FRAMES: int = 4

const LiveSession: Script = preload("res://tests/live_session.gd")

var _session: RefCounted


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	await process_frame
	var output: String = OS.get_environment("TME_CAPTURE_OUTPUT").strip_edges()
	if output.is_empty():
		_fail("TME_CAPTURE_OUTPUT is required")
		return
	var credentials: Dictionary = LiveSession.credentials()
	if credentials.is_empty():
		_fail("the capture requires TME_EX_USERNAME, TME_EX_PASSWORD, and TME_EX_CHARACTER_ID")
		return

	_session = LiveSession.new(self)
	var client: ClientRoot = await _session.mount()
	print("endpoint = %s" % client.facade.endpoint.https_base_url)
	if not await _session.sign_in(credentials["username"], credentials["password"]):
		_fail("sign-in never reached the session bootstrap")
		return
	if not await _session.select_character(credentials["character_id"]):
		_fail("admission never reached online authority; last state %s" % client.state_machine.current)
		return
	print("lifecycle = %s" % client.state_machine.current)
	print("actor = %s" % client.authoritative_state.actor_id())

	var view: GridWorldView = client.world_screen.world_view as GridWorldView
	if view == null:
		_fail("the world shell is not presenting through the grid view")
		return
	# The shell installs the frame on the same signal that brought it in, but
	# the lattice is laid out against the view's real size, which is only known
	# after the shell has been through a layout pass.
	for _index: int in SETTLE_FRAMES:
		await process_frame
	if view.screen_targets().is_empty():
		_fail("the view holds no addressable targets to capture")
		return
	print("targets = %d" % view.screen_targets().size())
	print("observation_center = %s" % str(view.observation_center()))

	var recorded: String = OS.get_environment("TME_CAPTURE_FRAME_OUT").strip_edges()
	if not recorded.is_empty() and not _record_frame(view, client, recorded):
		_fail("could not record the authoritative frame to %s" % recorded)
		return

	var report: Dictionary = CaptureEmitter.emit(view, root, output, ROUTE)
	if not bool(report.get("ok", false)):
		_fail(str(report.get("error", "the capture refused without a reason")))
		return
	print(JSON.stringify(report, "", true))
	client._on_logout_requested()
	_session.release()
	print(SUCCESS_SENTINEL)
	quit(0)


## Writes the frame exactly as the client holds it, with the provenance a reader
## needs to know what produced it. Nothing here reshapes the frame: a fixture
## that was tidied on the way out would not be a real server frame any more.
func _record_frame(view: GridWorldView, client: ClientRoot, path: String) -> bool:
	var document: Dictionary = {
		"schema_version": 1,
		"kind": FRAME_FIXTURE_KIND,
		"provenance": {
			"route": ROUTE,
			"recorded_by": "client/tests/live_capture.gd",
			"driver": "tools/run_fixture_land_capture.py",
			"world_revision": client.authoritative_state.world_revision(),
			"server_sequence": client.authoritative_state.server_sequence(),
		},
		"frame_generation": view.frame_generation(),
		"frame": view.frame(),
	}
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(document, "  ", true) + "\n")
	file.close()
	return true


func _fail(message: String) -> void:
	printerr("live capture refused: " + message)
	if _session != null:
		_session.release()
	quit(1)
