class_name CaptureEmitter
extends RefCounted

## Writes one nominated frame as a capture the Workbench can select over.
##
## Three files, and each one is meaningless without the other two:
##
## [br]- `capture.png` — what the presenter actually drew, straight off the
##   viewport.
## [br]- `capture.identity.pgm` — the **identity raster**: one 16-bit index per
##   pixel naming which entry of the sidecar's target list owns that pixel, or
##   zero for none. Written as a binary Netpbm greyscale so that both sides read
##   and write it with no library at all — Godot fills rectangles into an
##   [Image] and the Workbench slices bytes.
## [br]- `capture.sidecar.json` — the frame generation, the camera identity, the
##   viewport size, the digests of the other two files, and the full target list
##   with each target's identity, kind, coordinate, presentation layer, screen
##   anchor, and screen hit shape.
##
## **The raster is exact by construction, not by agreement.** [GridWorldView]
## draws every addressable thing as a rectangle, so the raster is those same
## rectangles filled in the same order — no sampling, no anti-aliased edges to
## guess at, and no possibility of the raster and the pointer resolution
## disagreeing, because both read one list.
##
## This is the presenter obligation of the Workbench spec §5.2, satisfied the
## cheap way a 2D presenter can satisfy it. A pixel-native presenter inherits
## it unchanged.

const SCHEMA_VERSION: int = 1
const SIDECAR_KIND: String = "capture_identity_sidecar"
const RASTER_FORMAT: String = "pgm_p5_u16_be_target_index"
const CAMERA_KIND: String = "orthographic_square_lattice"
const PRODUCER: String = "grid_world_view"

const IMAGE_NAME: String = "capture.png"
const RASTER_NAME: String = "capture.identity.pgm"
const SIDECAR_NAME: String = "capture.sidecar.json"

## A 16-bit index buffer addresses this many targets, and index zero is
## reserved for "no target". A frame with more addressable things than this is
## refused rather than silently truncated to a wrong address.
const MAXIMUM_TARGETS: int = 65535


## Emits the three files into `directory` (an absolute path) and returns a
## report. `ok` is false and `error` names the reason when anything refuses;
## nothing partial is left behind that a consumer could mistake for a capture.
static func emit(
	view: GridWorldView,
	viewport: Viewport,
	directory: String,
	route: String,
) -> Dictionary:
	if view == null:
		return _refuse("no view to capture")
	if viewport == null:
		return _refuse("no viewport to capture")
	var screen_targets: Array[Dictionary] = view.screen_targets()
	if screen_targets.is_empty():
		return _refuse("the view holds no authoritative frame to capture")
	if screen_targets.size() > MAXIMUM_TARGETS:
		return _refuse(
			"the frame carries %d addressable targets and the identity raster indexes %d"
			% [screen_targets.size(), MAXIMUM_TARGETS]
		)

	var transform: Transform2D = view.get_global_transform_with_canvas()
	if not transform.get_scale().is_equal_approx(Vector2.ONE) or not is_zero_approx(transform.get_rotation()):
		return _refuse("the view is scaled or rotated; capture rectangles would not be exact")
	var offset: Vector2i = Vector2i(roundi(transform.origin.x), roundi(transform.origin.y))

	# Asked before the texture is touched, because asking afterwards means asking
	# the dummy rendering driver for a picture it will refuse with an engine
	# error of its own. A capture that cannot show what was drawn is not a
	# capture, so this is a refusal rather than a blank image.
	if DisplayServer.get_name() == "headless":
		return _refuse(
			"this run is headless and Godot's headless display driver produces no "
			+ "viewport image; run the capture under a real or virtual display"
		)
	var texture: ViewportTexture = viewport.get_texture()
	if texture == null:
		return _refuse("the viewport has no texture to capture")
	var picture: Image = texture.get_image()
	if picture == null:
		return _refuse("the viewport texture produced no image to capture")

	if DirAccess.make_dir_recursive_absolute(directory) != OK and not DirAccess.dir_exists_absolute(directory):
		return _refuse("could not create the capture directory %s" % directory)

	var width: int = picture.get_width()
	var height: int = picture.get_height()
	var targets: Array[Dictionary] = target_records(screen_targets, offset)

	var image_path: String = directory.path_join(IMAGE_NAME)
	var save_error: Error = picture.save_png(image_path)
	if save_error != OK:
		return _refuse("could not write %s (%d)" % [image_path, save_error])

	var raster_path: String = directory.path_join(RASTER_NAME)
	var raster_error: String = _write_raster(raster_path, width, height, targets)
	if not raster_error.is_empty():
		return _refuse(raster_error)

	var sidecar: Dictionary = {
		"schema_version": SCHEMA_VERSION,
		"kind": SIDECAR_KIND,
		"producer": PRODUCER,
		"route": route,
		"frame_generation": view.frame_generation(),
		"scene": _scene(view),
		"camera": _camera(view, offset),
		"viewport": {"width": width, "height": height},
		"image": {"path": IMAGE_NAME, "sha256": FileAccess.get_sha256(image_path)},
		"identity_raster": {
			"path": RASTER_NAME,
			"format": RASTER_FORMAT,
			"width": width,
			"height": height,
			"sha256": FileAccess.get_sha256(raster_path),
		},
		"targets": targets,
	}
	var sidecar_path: String = directory.path_join(SIDECAR_NAME)
	var sidecar_file: FileAccess = FileAccess.open(sidecar_path, FileAccess.WRITE)
	if sidecar_file == null:
		return _refuse("could not write %s" % sidecar_path)
	sidecar_file.store_string(JSON.stringify(sidecar, "  ", true) + "\n")
	sidecar_file.close()

	return {
		"ok": true,
		"directory": directory,
		"image": image_path,
		"identity_raster": raster_path,
		"sidecar": sidecar_path,
		"targets": targets.size(),
		"viewport": {"width": width, "height": height},
	}


## The target list, in draw order, with every rectangle moved into the
## viewport's own pixel space so a consumer never has to know where inside the
## window the view was mounted.
static func target_records(screen_targets: Array[Dictionary], offset: Vector2i) -> Array[Dictionary]:
	var records: Array[Dictionary] = []
	for record: Dictionary in screen_targets:
		var target: Dictionary = record["target"]
		var rect: Rect2i = record["rect"]
		var anchor: Vector2i = record["anchor"]
		var square: Vector2i = target["coordinate"]
		records.append({
			"index": records.size() + 1,
			"identity": str(target["identity"]),
			"kind": str(target["kind"]),
			"source_identity": str(target["source_identity"]),
			"coordinate": {"x": square.x, "y": square.y},
			"presentation_layer": str(record["layer"]),
			"anchor": {"x": anchor.x + offset.x, "y": anchor.y + offset.y},
			"hit_shape": {
				"kind": "rect",
				"x": rect.position.x + offset.x,
				"y": rect.position.y + offset.y,
				"width": rect.size.x,
				"height": rect.size.y,
			},
		})
	return records


## The identity raster's samples: each target's rectangle filled with its own
## index, in draw order, so a later marker overwrites the square beneath it
## exactly as it did in the picture — and exactly as
## [method GridWorldView.semantic_target_for_display_position] resolves it.
##
## Two bytes per sample, most significant first. An [Image] in [constant
## Image.FORMAT_RG8] already holds that byte order and fills rectangles in
## engine code, so this is the fast path as well as the simple one.
static func raster_samples(
	width: int, height: int, targets: Array[Dictionary]
) -> PackedByteArray:
	var raster: Image = Image.create_empty(width, height, false, Image.FORMAT_RG8)
	raster.fill(Color8(0, 0, 0))
	for record: Dictionary in targets:
		var index: int = int(record["index"])
		var shape: Dictionary = record["hit_shape"]
		raster.fill_rect(
			Rect2i(int(shape["x"]), int(shape["y"]), int(shape["width"]), int(shape["height"])),
			Color8(index >> 8, index & 0xFF, 0),
		)
	return raster.get_data()


## The index the raster records at one pixel, or zero for no target.
static func raster_index_at(
	samples: PackedByteArray, width: int, x: int, y: int
) -> int:
	var offset: int = (y * width + x) * 2
	if offset < 0 or offset + 1 >= samples.size():
		return 0
	return (samples[offset] << 8) | samples[offset + 1]


static func _write_raster(
	path: String, width: int, height: int, targets: Array[Dictionary]
) -> String:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return "could not write %s" % path
	file.store_string("P5\n# workbench capture identity raster\n# index 0 is no target\n")
	file.store_string("%d %d\n65535\n" % [width, height])
	file.store_buffer(raster_samples(width, height, targets))
	file.close()
	return ""


static func _scene(view: GridWorldView) -> Dictionary:
	## Only what the client actually holds. The land the capture belongs to is
	## the Workbench's fact to bind, not the client's to assert.
	var frame: Dictionary = view.frame()
	var centre: Variant = frame.get("observation_center")
	var realm: String = ""
	var level: String = ""
	if centre is Dictionary:
		realm = str((centre as Dictionary).get("realm", ""))
		level = str((centre as Dictionary).get("level", ""))
	var square: Vector2i = view.observation_center()
	return {
		"realm": realm,
		"level": level,
		"observation_center": {"x": square.x, "y": square.y},
		"logical_time": str(frame.get("logical_time", "")),
	}


static func _camera(view: GridWorldView, offset: Vector2i) -> Dictionary:
	var layout: Dictionary = view.layout()
	var origin: Vector2i = layout["origin"]
	var bounds: Dictionary = layout["bounds"]
	return {
		"kind": CAMERA_KIND,
		"square_pitch_px": int(layout["pitch"]),
		# Where square (0,0) of the world would be drawn, in viewport pixels.
		# A square's rectangle is this origin plus the square times the pitch;
		# there is no other framing constant in force.
		"square_origin_px": {"x": origin.x + offset.x, "y": origin.y + offset.y},
		"square_bounds": bounds,
		"view_origin_px": {"x": offset.x, "y": offset.y},
		"view_size_px": {"x": int(view.size.x), "y": int(view.size.y)},
	}


static func _refuse(reason: String) -> Dictionary:
	return {"ok": false, "error": reason}
