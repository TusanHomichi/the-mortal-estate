extends SceneTree

## Captures one named feel preset at the native review surface.
##
##     TME_FEEL_ASSETS=<candidate directory> TME_FEEL_PRESET=night \
##     TME_CAPTURE_OUTPUT=<output directory> xvfb-run -a <godot> \
##         --path client --resolution 1280x800 -s res://tests/feel_capture.gd

const FeelSceneResource: PackedScene = preload("res://presentation/feel/FeelScene.tscn")
const SUCCESS_SENTINEL: String = "TME_FEEL_CAPTURE_OK"
const SETTLE_FRAMES: int = 18
const SEQUENCE_FRAME_COUNT: int = 24
const SEQUENCE_SECONDS: float = 2.0


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	if DisplayServer.get_name() == "headless":
		_fail("a real or virtual display is required")
		return
	var output_root: String = OS.get_environment("TME_CAPTURE_OUTPUT").strip_edges()
	if output_root.is_empty() or not output_root.is_absolute_path():
		_fail("TME_CAPTURE_OUTPUT must name an absolute output directory")
		return
	root.size = Vector2i(1280, 800)
	var scene: FeelScene = FeelSceneResource.instantiate() as FeelScene
	root.add_child(scene)
	for _index: int in SETTLE_FRAMES:
		await process_frame
	if not scene.built_ok:
		_fail("feel scene refused its assets: " + scene.load_error)
		return
	DirAccess.make_dir_recursive_absolute(output_root)
	var preset: String = OS.get_environment("TME_FEEL_PRESET").strip_edges().to_lower()
	if preset.is_empty():
		preset = FeelScene.DEFAULT_PRESET
	var safe_name: String = preset.replace(",", "-").replace(" ", "")
	var requested_frames: String = OS.get_environment("TME_FEEL_FRAMES").strip_edges()
	if not requested_frames.is_empty():
		if requested_frames != str(SEQUENCE_FRAME_COUNT):
			_fail("TME_FEEL_FRAMES must be 24 when set")
			return
		await _capture_sequence(output_root, safe_name, preset)
		return
	var output_path: String = output_root.path_join("feel-" + safe_name + "-1280x800.png")
	var image: Image = _capture_image()
	if image.is_empty():
		return
	var save_error: Error = image.save_png(output_path)
	if save_error != OK:
		_fail("could not write the feel capture")
		return
	print(JSON.stringify({"ok": true, "preset": preset, "path": output_path, "size": [image.get_width(), image.get_height()]}, "", true))
	print(SUCCESS_SENTINEL)
	quit(0)


func _capture_sequence(output_root: String, safe_name: String, preset: String) -> void:
	var frames_root: String = output_root.path_join(safe_name + "-frames")
	DirAccess.make_dir_recursive_absolute(frames_root)
	for frame_index: int in SEQUENCE_FRAME_COUNT:
		await create_timer(SEQUENCE_SECONDS / float(SEQUENCE_FRAME_COUNT)).timeout
		var image: Image = _capture_image()
		if image.is_empty():
			return
		var output_path: String = frames_root.path_join("frame-%04d.png" % (frame_index + 1))
		if image.save_png(output_path) != OK:
			_fail("could not write feel sequence frame %d" % (frame_index + 1))
			return
	print(JSON.stringify({"frames": SEQUENCE_FRAME_COUNT, "ok": true, "preset": preset, "seconds": SEQUENCE_SECONDS, "path": frames_root}, "", true))
	print(SUCCESS_SENTINEL)
	quit(0)


func _capture_image() -> Image:
	var image: Image = root.get_texture().get_image()
	if image.is_empty() or image.get_width() != 1280 or image.get_height() != 800:
		_fail("the viewport did not yield a 1280x800 image")
		return Image.new()
	return image


func _fail(message: String) -> void:
	printerr("feel capture refused: " + message)
	quit(1)
