extends SceneTree

## Photographs the beat.
##
## Charter item 3 asks for the authoritative pulse to be visible, and a claim
## that something is visible is settled by looking at it. This mounts the
## shipped `ClientRoot.tscn` against a real server, waits until the client has
## actually observed a beat, and then captures the same window a player would be
## looking at at several known points inside **one** beat — so the meter's
## advance is in the pictures rather than in a description of them.
##
## Every sample records the frame it was taken under alongside the picture: the
## frame's own logical time and readiness time, whether the frame said the
## observer could act, the span the client measured, and the fill and words the
## meter was showing. `tools/run_pulse_capture.py` judges those, and refuses a
## run where the meter did not move, where the samples straddled a beat, or
## where the fill and the frame disagreed.
##
## Provisioned and driven by `tools/run_pulse_capture.py`. A real display is
## required; the driver supplies one.

const SUCCESS_SENTINEL: String = "TME_PULSE_CAPTURE_OK"
const MANIFEST_NAME: String = "pulse.json"
const ROUTE: String = "pulse"
const SETTLE_FRAMES: int = 4
const STEP_TIMEOUT_MSEC: int = 30000
const SCHEMA_VERSION: int = 1

## Where inside the beat the samples are taken. Three points, spread far enough
## apart that a meter which had frozen could not produce them by accident and a
## reader can see the difference without measuring it.
const SAMPLE_FILLS: Array[float] = [0.15, 0.50, 0.85]

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
		_fail("the pulse capture requires TME_EX_USERNAME, TME_EX_PASSWORD, and TME_EX_CHARACTER_ID")
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

	var shell: WorldShellScreen = client.world_screen
	var view: GridWorldView = shell.world_view as GridWorldView
	if view == null:
		_fail("the world shell is not presenting through the grid view")
		return
	for _index: int in SETTLE_FRAMES:
		await process_frame

	var clock: PulseClock = shell.pulse_clock
	# Two consecutive beats have to arrive before the client has observed an
	# interval at all. Until then it draws no fill, on purpose, and there is
	# nothing here worth photographing.
	if not await _session.wait_until(func() -> bool: return clock.has_measured_span(), STEP_TIMEOUT_MSEC):
		_fail("the client never observed a beat interval to draw")
		return
	print("measured_span_msec = %d" % clock.span_msec())

	# Start the samples at the top of a fresh beat so all three belong to one
	# round of logical time rather than to whichever ones happened to pass.
	var before: String = clock.logical_time()
	if not await _session.wait_until(func() -> bool: return clock.logical_time() != before, STEP_TIMEOUT_MSEC):
		_fail("logical time stalled at T%s" % before)
		return

	var samples: Array = []
	for target: float in SAMPLE_FILLS:
		var sample: Dictionary = await _sample(client, shell, view, output, target, samples.size() + 1)
		if sample.is_empty():
			return
		samples.append(sample)
		print("pulse_sample = %d at fill %.2f, T%s" % [
			samples.size(), float(sample["fill"]), str(sample["logical_time"])
		])

	var manifest: Dictionary = {
		"schema_version": SCHEMA_VERSION,
		"kind": "pulse_capture_manifest",
		"produced_by": "client/tests/pulse_capture.gd",
		"driver": "tools/run_pulse_capture.py",
		"measured_span_msec": clock.span_msec(),
		"requested_fills": SAMPLE_FILLS,
		"samples": samples,
	}
	var manifest_path: String = output.path_join(MANIFEST_NAME)
	var file: FileAccess = FileAccess.open(manifest_path, FileAccess.WRITE)
	if file == null:
		_fail("could not write %s" % manifest_path)
		return
	file.store_string(JSON.stringify(manifest, "  ", true) + "\n")
	file.close()
	print("pulse_manifest = %s" % manifest_path)

	client._on_logout_requested()
	_session.release()
	print(SUCCESS_SENTINEL)
	quit(0)


## Waits until the beat has filled to `target` and captures the window then,
## recording what the client held at that instant rather than what it was asked
## for. The fill written down is read back out of the meter after the capture,
## so a sample can never claim a fill the picture does not show.
func _sample(
	client: ClientRoot,
	shell: WorldShellScreen,
	view: GridWorldView,
	output: String,
	target: float,
	index: int,
) -> Dictionary:
	var clock: PulseClock = shell.pulse_clock
	var reached: bool = await _session.wait_until(func() -> bool:
		return clock.beat_fill() >= target
	, STEP_TIMEOUT_MSEC)
	if not reached:
		_fail("the beat never filled to %.2f" % target)
		return {}

	var directory: String = output.path_join("beat-%d" % index)
	var report: Dictionary = CaptureEmitter.emit(view, root, directory, ROUTE)
	if not bool(report.get("ok", false)):
		_fail(str(report.get("error", "the capture refused without a reason")))
		return {}

	var meter: PulseMeter = shell.pulse_meter
	var fills: Array = []
	for segment: Dictionary in meter.segments():
		fills.append(float(segment["fill"]))
	return {
		"index": index,
		"requested_fill": target,
		"directory": directory,
		"image": report["image"],
		"sidecar": report["sidecar"],
		"logical_time": clock.logical_time(),
		"ready_at": clock.ready_at(),
		"can_act": clock.is_ready(),
		"beats_until_ready": clock.beats_until_ready(),
		"measured": clock.has_measured_span(),
		"span_msec": clock.span_msec(),
		"fill": clock.beat_fill(),
		"segment_fills": fills,
		"meter_text": meter.meter_text(),
		"world_revision": client.authoritative_state.world_revision(),
	}


func _fail(message: String) -> void:
	printerr("pulse capture refused: " + message)
	if _session != null:
		_session.release()
	quit(1)
