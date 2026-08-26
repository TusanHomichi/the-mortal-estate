extends SceneTree

## The ordinary capture route: one authoritative frame, drawn and captured.
##
## No server, no database, no credentials, no account. It mounts [GridWorldView]
## alone in the window, presents one recorded authoritative frame, and writes
## the capture, the identity raster, and the sidecar through [CaptureEmitter].
## That is the whole cost of a capture on the Workbench's ordinary path, and it
## is seconds rather than minutes.
##
## The frame it replays is a **real server frame**, recorded once by the live
## route (`client/tests/live_capture.gd`) and tracked as a fixture. Replaying a
## recorded frame is honest in a way that synthesising one would not be: nothing
## here invents a world, and a fixture that drifted from the compiled land is
## caught by `tests/test_capture_correspondence.py`.
##
## A real window is required. Godot's headless display driver produces no
## viewport image at all, so the driver runs this under a virtual display; see
## `tools/workbench/capture.py`.
##
##     TME_CAPTURE_FRAME=<frame fixture> TME_CAPTURE_OUTPUT=<directory> \
##         xvfb-run -a <godot> --path client --resolution 1024x768 \
##         -s res://tests/capture_fixture_frame.gd

const SUCCESS_SENTINEL: String = "TME_CAPTURE_OK"
const FRAME_FIXTURE_KIND: String = "capture_frame_fixture"
const ROUTE: String = "fixture_frame"
const SETTLE_FRAMES: int = 3


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	await process_frame
	var frame_path: String = OS.get_environment("TME_CAPTURE_FRAME").strip_edges()
	var output: String = OS.get_environment("TME_CAPTURE_OUTPUT").strip_edges()
	if frame_path.is_empty() or output.is_empty():
		_fail("TME_CAPTURE_FRAME and TME_CAPTURE_OUTPUT are both required")
		return

	var document: Variant = _read_frame_document(frame_path)
	if document is not Dictionary:
		_fail("%s is not a %s document" % [frame_path, FRAME_FIXTURE_KIND])
		return
	var fixture: Dictionary = document as Dictionary
	var frame: Variant = fixture.get("frame")
	if frame is not Dictionary:
		_fail("%s carries no frame" % frame_path)
		return

	var view: GridWorldView = (
		load("res://presentation/GridWorldView.tscn") as PackedScene
	).instantiate() as GridWorldView
	root.add_child(view)
	view.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.position = Vector2.ZERO
	view.size = Vector2(root.size)
	view.present_frame(frame as Dictionary, int(fixture.get("frame_generation", 0)))
	for _index: int in SETTLE_FRAMES:
		await process_frame

	var report: Dictionary = CaptureEmitter.emit(view, root, output, ROUTE)
	view.free()
	if not bool(report.get("ok", false)):
		_fail(str(report.get("error", "the capture refused without a reason")))
		return
	print(JSON.stringify(report, "", true))
	print(SUCCESS_SENTINEL)
	quit(0)


func _read_frame_document(path: String) -> Variant:
	var file: FileAccess = FileAccess.open(path, FileAccess.READ)
	if file == null:
		return null
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if parsed is not Dictionary:
		return null
	if str((parsed as Dictionary).get("kind", "")) != FRAME_FIXTURE_KIND:
		return null
	return parsed


func _fail(message: String) -> void:
	printerr("capture refused: " + message)
	quit(1)
