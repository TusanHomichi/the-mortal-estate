class_name FeelAssetLoader
extends RefCounted

## Strict loader for the disposable in-engine feel-scene asset packet.
##
## `TME_FEEL_ASSETS` names the directory containing `feel-manifest.json`.
## Nothing in the project supplies a default path. The manifest schema is:
##
## {
##   "schema_version": 1,
##   "assets": {
##     "terrain": {<material>: {"file": <relative PNG>, "sha256": <hex>}},
##     "walls": {<member>: {"file": <relative PNG>, "sha256": <hex>}},
##     "props": {<prop>: {"file": <relative PNG>, "sha256": <hex>}}
##   },
##   "layout": {
##     "grid_extents": {"i": <positive int>, "j": <positive int>},
##     "cells": [{"i": <int>, "j": <int>, "material": <terrain key>}],
##     "wall_runs": [{"axis": "x"|"z", "start": [x,z], "cells": <int>,
##                    "door_interval": null|[run start, run end]}],
##     "props": [{"kind": <prop key>, "cell_anchor": [x,z],
##                "nominal_height": <positive number>, "sway": <bool>}],
##     "light_sources": {"lantern_glass": [x,y,z], "candles": [[x,y,z], ...]}
##   }
## }
##
## Required wall keys are `plinth`, `plaster`, `cap_front`, `cap_top`, `sill`,
## `post`, and `door`. Required prop keys are `caretaker`, `tree`,
## `lantern_post`, `shrine_table`, and `grave_marker`. Every referenced source
## is confined to the named directory, hashed before decoding, and refused on
## any mismatch. A refusal returns a reason; it never substitutes an asset.

const ENVIRONMENT_VARIABLE: String = "TME_FEEL_ASSETS"
const MANIFEST_NAME: String = "feel-manifest.json"
const REQUIRED_WALLS: Array[String] = [
	"plinth", "plaster", "cap_front", "cap_top", "sill", "post", "door",
]
const REQUIRED_PROPS: Array[String] = [
	"caretaker", "tree", "lantern_post", "shrine_table", "grave_marker",
]


func load_from_environment() -> Dictionary:
	return load_root(OS.get_environment(ENVIRONMENT_VARIABLE).strip_edges())


func load_root(root_path: String) -> Dictionary:
	if root_path.is_empty():
		return _reject("%s is unset; no candidate feel assets are available" % ENVIRONMENT_VARIABLE)
	var root: String = root_path.simplify_path()
	if not root.is_absolute_path() or not DirAccess.dir_exists_absolute(root):
		return _reject("%s does not name a readable absolute directory" % ENVIRONMENT_VARIABLE)
	var manifest_path: String = root.path_join(MANIFEST_NAME)
	var file: FileAccess = FileAccess.open(manifest_path, FileAccess.READ)
	if file == null:
		return _reject("the candidate feel manifest is unreadable")
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if parsed is not Dictionary:
		return _reject("the candidate feel manifest is not a JSON object")
	var manifest: Dictionary = parsed as Dictionary
	var schema_error: String = _validate_manifest(manifest)
	if not schema_error.is_empty():
		return _reject(schema_error)

	var textures: Dictionary = {}
	var assets: Dictionary = manifest["assets"] as Dictionary
	for group_name: String in ["terrain", "walls", "props"]:
		var group: Dictionary = assets[group_name] as Dictionary
		for asset_name_variant: Variant in group.keys():
			var asset_name: String = str(asset_name_variant)
			var row: Dictionary = group[asset_name] as Dictionary
			var loaded: Dictionary = _load_texture(root, row)
			if not bool(loaded.get("ok", false)):
				return _reject("%s/%s: %s" % [group_name, asset_name, str(loaded.get("error", "asset refused"))])
			textures["%s/%s" % [group_name, asset_name]] = loaded["texture"]
	return {"ok": true, "manifest": manifest, "textures": textures, "root": root, "error": ""}


func _validate_manifest(manifest: Dictionary) -> String:
	if not _has_exact_keys(manifest, ["schema_version", "assets", "layout"]):
		return "the candidate feel manifest has unknown or missing top-level fields"
	if not _is_integer_number(manifest.get("schema_version")) or int(manifest["schema_version"]) != 1:
		return "the candidate feel manifest schema version is unsupported"
	if manifest.get("assets") is not Dictionary or manifest.get("layout") is not Dictionary:
		return "the candidate feel manifest assets or layout is not an object"
	var assets: Dictionary = manifest["assets"] as Dictionary
	if not _has_exact_keys(assets, ["terrain", "walls", "props"]):
		return "the candidate feel asset groups are incomplete"
	for group_name: String in ["terrain", "walls", "props"]:
		if assets.get(group_name) is not Dictionary or (assets[group_name] as Dictionary).is_empty():
			return "the candidate feel %s group is empty" % group_name
		for key: Variant in (assets[group_name] as Dictionary).keys():
			if typeof(key) != TYPE_STRING or not _valid_asset_row((assets[group_name] as Dictionary)[key]):
				return "the candidate feel %s asset row is invalid" % group_name
	var walls: Dictionary = assets["walls"] as Dictionary
	for wall_name: String in REQUIRED_WALLS:
		if not walls.has(wall_name):
			return "the candidate feel wall set is missing " + wall_name
	var props: Dictionary = assets["props"] as Dictionary
	for prop_name: String in REQUIRED_PROPS:
		if not props.has(prop_name):
			return "the candidate feel prop set is missing " + prop_name
	return _validate_layout(manifest["layout"] as Dictionary, assets)


func _validate_layout(layout: Dictionary, assets: Dictionary) -> String:
	if not _has_exact_keys(layout, ["grid_extents", "cells", "wall_runs", "props", "light_sources"]):
		return "the candidate feel layout has unknown or missing fields"
	if layout.get("grid_extents") is not Dictionary:
		return "the candidate feel grid extents are invalid"
	var extents: Dictionary = layout["grid_extents"] as Dictionary
	if not _has_exact_keys(extents, ["i", "j"]):
		return "the candidate feel grid extents are invalid"
	if not _is_integer_number(extents.get("i")) or not _is_integer_number(extents.get("j")):
		return "the candidate feel grid extents must be integers"
	var extent_i: int = int(extents["i"])
	var extent_j: int = int(extents["j"])
	if extent_i <= 0 or extent_j <= 0:
		return "the candidate feel grid extents must be positive"
	if layout.get("cells") is not Array or (layout["cells"] as Array).size() != extent_i * extent_j:
		return "the candidate feel material plan must name every cell exactly once"
	var seen_cells: Dictionary = {}
	var terrain: Dictionary = assets["terrain"] as Dictionary
	for cell_variant: Variant in layout["cells"] as Array:
		if cell_variant is not Dictionary:
			return "a candidate feel cell is not an object"
		var cell: Dictionary = cell_variant as Dictionary
		if not _has_exact_keys(cell, ["i", "j", "material"]):
			return "a candidate feel cell has invalid fields"
		if not _is_integer_number(cell.get("i")) or not _is_integer_number(cell.get("j")):
			return "a candidate feel cell coordinate is not an integer"
		var i: int = int(cell["i"])
		var j: int = int(cell["j"])
		var cell_key: String = "%d:%d" % [i, j]
		if i < 0 or i >= extent_i or j < 0 or j >= extent_j or seen_cells.has(cell_key):
			return "the candidate feel material plan has an invalid or duplicate cell"
		if typeof(cell.get("material")) != TYPE_STRING or not terrain.has(str(cell["material"])):
			return "a candidate feel cell names an unknown material"
		seen_cells[cell_key] = true
	if layout.get("wall_runs") is not Array or (layout["wall_runs"] as Array).is_empty():
		return "the candidate feel layout carries no wall runs"
	for run_variant: Variant in layout["wall_runs"] as Array:
		if run_variant is not Dictionary or not _valid_wall_run(run_variant as Dictionary):
			return "a candidate feel wall run is invalid"
	if layout.get("props") is not Array or (layout["props"] as Array).is_empty():
		return "the candidate feel layout carries no props"
	for prop_variant: Variant in layout["props"] as Array:
		if prop_variant is not Dictionary or not _valid_prop(prop_variant as Dictionary, assets["props"] as Dictionary):
			return "a candidate feel prop placement is invalid"
	if layout.get("light_sources") is not Dictionary:
		return "the candidate feel light sources are invalid"
	var lights: Dictionary = layout["light_sources"] as Dictionary
	if not _has_exact_keys(lights, ["lantern_glass", "candles"]):
		return "the candidate feel light sources are incomplete"
	if not _valid_vector3_array(lights.get("lantern_glass")) or lights.get("candles") is not Array:
		return "the candidate feel light positions are invalid"
	for candle: Variant in lights["candles"] as Array:
		if not _valid_vector3_array(candle):
			return "a candidate feel candle position is invalid"
	return ""


func _valid_wall_run(run: Dictionary) -> bool:
	if not _has_exact_keys(run, ["axis", "start", "cells", "door_interval"]):
		return false
	if typeof(run.get("axis")) != TYPE_STRING or str(run["axis"]) not in ["x", "z"]:
		return false
	if not _valid_vector2_array(run.get("start")) or not _is_integer_number(run.get("cells")) or int(run["cells"]) <= 0:
		return false
	var door: Variant = run.get("door_interval")
	return door == null or (_valid_vector2_array(door) and float((door as Array)[0]) >= 0.0 and float((door as Array)[1]) <= float(run["cells"]) and float((door as Array)[0]) < float((door as Array)[1]))


func _valid_prop(prop: Dictionary, assets: Dictionary) -> bool:
	return (
		_has_exact_keys(prop, ["kind", "cell_anchor", "nominal_height", "sway"])
		and typeof(prop.get("kind")) == TYPE_STRING
		and assets.has(str(prop["kind"]))
		and _valid_vector2_array(prop.get("cell_anchor"))
		and typeof(prop.get("nominal_height")) in [TYPE_INT, TYPE_FLOAT]
		and float(prop["nominal_height"]) > 0.0
		and typeof(prop.get("sway")) == TYPE_BOOL
	)


func _valid_asset_row(value: Variant) -> bool:
	if value is not Dictionary:
		return false
	var row: Dictionary = value as Dictionary
	return (
		_has_exact_keys(row, ["file", "sha256"])
		and typeof(row.get("file")) == TYPE_STRING
		and _safe_relative_file(str(row["file"]))
		and _valid_sha(str(row.get("sha256", "")))
	)


func _load_texture(root: String, row: Dictionary) -> Dictionary:
	var relative_path: String = str(row["file"])
	if not _safe_relative_file(relative_path):
		return _reject("asset path escapes the candidate directory")
	var path: String = root.path_join(relative_path).simplify_path()
	if not path.begins_with(root.rstrip("/") + "/") or not FileAccess.file_exists(path):
		return _reject("asset file is missing")
	var bytes: PackedByteArray = FileAccess.get_file_as_bytes(path)
	var hashing: HashingContext = HashingContext.new()
	hashing.start(HashingContext.HASH_SHA256)
	hashing.update(bytes)
	if hashing.finish().hex_encode() != str(row["sha256"]):
		return _reject("asset digest does not match the manifest")
	var image: Image = Image.new()
	if image.load(path) != OK or image.is_empty():
		return _reject("asset is not a decodable image")
	return {"ok": true, "texture": ImageTexture.create_from_image(image), "error": ""}


func _safe_relative_file(path: String) -> bool:
	return (
		not path.is_empty()
		and not path.is_absolute_path()
		and not path.contains("\\")
		and ".." not in path.split("/", false)
		and path.simplify_path() == path
		and path.to_lower().ends_with(".png")
	)


func _valid_sha(value: String) -> bool:
	if value.length() != 64:
		return false
	for index: int in value.length():
		if value.substr(index, 1) not in "0123456789abcdef":
			return false
	return true


func _valid_vector2_array(value: Variant) -> bool:
	return value is Array and (value as Array).size() == 2 and _is_number((value as Array)[0]) and _is_number((value as Array)[1])


func _valid_vector3_array(value: Variant) -> bool:
	return value is Array and (value as Array).size() == 3 and _is_number((value as Array)[0]) and _is_number((value as Array)[1]) and _is_number((value as Array)[2])


func _is_number(value: Variant) -> bool:
	return typeof(value) in [TYPE_INT, TYPE_FLOAT] and is_finite(float(value))


func _is_integer_number(value: Variant) -> bool:
	return _is_number(value) and float(value) == floorf(float(value))


func _has_exact_keys(value: Dictionary, expected: Array[String]) -> bool:
	if value.size() != expected.size():
		return false
	for key: Variant in value.keys():
		if typeof(key) != TYPE_STRING or str(key) not in expected:
			return false
	return true


func _reject(message: String) -> Dictionary:
	return {"ok": false, "error": message}
