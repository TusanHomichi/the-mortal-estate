class_name FeelScene
extends Node3D

## Standalone presentation experiment beside the renderer seam.
##
## It consumes no authoritative frame and emits no gameplay fact. Candidate
## art enters only through FeelAssetLoader and `TME_FEEL_ASSETS`; presets enter
## only through `TME_FEEL_PRESET`.

const AssetLoaderScript: Script = preload("res://presentation/feel/feel_asset_loader.gd")
const GroundShader: Shader = preload("res://presentation/feel/shaders/ground.gdshader")
const SwayShader: Shader = preload("res://presentation/feel/shaders/sway.gdshader")
const FogShader: Shader = preload("res://presentation/feel/shaders/fog_quad.gdshader")

const CAMERA_OFFSET: Vector3 = Vector3(8.0, 6.531973, 8.0)
const CAMERA_SIZE_1280X800: float = 5.050762722761054
const CAMERA_TARGET_HEIGHT: float = 1.22
const WALL_THICKNESS: float = 0.22
const PLINTH_TOP: float = 0.30
const SILL_TOP: float = 0.42
const CAP_BOTTOM: float = 1.98
const CAP_TOP: float = 2.20
const POST_WIDTH: float = 0.11
const CORNER_POST_WIDTH: float = POST_WIDTH * 1.3
const DOOR_HEIGHT: float = 1.60
const LINTEL_TOP: float = 1.74
const DOOR_LINTEL_INSET: float = 0.07
const DEFAULT_PRESET: String = "night"
const KNOWN_PRESETS: Array[String] = ["night", "dusk", "rain", "fog", "wind"]

var built_ok: bool = false
var absence_visible: bool = false
var load_error: String = ""
var geometry_summary: Dictionary = {
	"ground_cells": 0,
	"wall_boxes": 0,
	"wall_posts": 0,
	"wall_braces": 0,
	"props": 0,
	"lights": 0,
}
var active_presets: PackedStringArray = PackedStringArray()

var _camera: Camera3D
var _lantern_light: OmniLight3D
var _flicker_phase: float = 0.0
var _flicker_energy: float = 2.45


func _ready() -> void:
	active_presets = _resolve_presets(OS.get_environment("TME_FEEL_PRESET"))
	var loader: FeelAssetLoader = AssetLoaderScript.new() as FeelAssetLoader
	var result: Dictionary = loader.load_from_environment()
	if not bool(result.get("ok", false)):
		load_error = str(result.get("error", "candidate feel assets were refused"))
		_present_absence(load_error)
		return
	_build_scene(result["manifest"] as Dictionary, result["textures"] as Dictionary)
	built_ok = true


func _process(_delta: float) -> void:
	if _lantern_light == null:
		return
	var now: float = Time.get_ticks_msec() / 1000.0
	var noise: float = sin(now * 5.7 + _flicker_phase) * 0.055 + sin(now * 11.3 + _flicker_phase * 1.7) * 0.025
	_lantern_light.light_energy = _flicker_energy * (1.0 + noise)


func camera() -> Camera3D:
	return _camera


func projected_cell_diamond_width() -> float:
	if _camera == null:
		return 0.0
	var corners: Array[Vector3] = [
		Vector3(-0.5, 0.0, -0.5),
		Vector3(0.5, 0.0, -0.5),
		Vector3(0.5, 0.0, 0.5),
		Vector3(-0.5, 0.0, 0.5),
	]
	var minimum_x: float = INF
	var maximum_x: float = -INF
	for corner: Vector3 in corners:
		var screen: Vector2 = _camera.unproject_position(corner)
		minimum_x = minf(minimum_x, screen.x)
		maximum_x = maxf(maximum_x, screen.x)
	return maximum_x - minimum_x


func _build_scene(manifest: Dictionary, textures: Dictionary) -> void:
	var layout: Dictionary = manifest["layout"] as Dictionary
	_build_environment()
	_build_camera(layout["grid_extents"] as Dictionary)
	_build_ground(layout["cells"] as Array, textures)
	_build_walls(layout["wall_runs"] as Array, textures)
	_build_props(layout["props"] as Array, textures)
	_build_lights(layout["light_sources"] as Dictionary)
	if "rain" in active_presets:
		_build_rain()
	if "fog" in active_presets:
		_build_fog_veil()


func _build_environment() -> void:
	var is_dusk: bool = "dusk" in active_presets
	var world_environment: WorldEnvironment = WorldEnvironment.new()
	world_environment.name = "DuskEnvironment" if is_dusk else "NightEnvironment"
	var environment: Environment = Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color("4b394d") if is_dusk else Color("091426")
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color("b39aa1") if is_dusk else Color("4a6d9e")
	environment.ambient_light_energy = 0.40 if is_dusk else 0.27
	environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	world_environment.environment = environment
	add_child(world_environment)


func _build_camera(extents: Dictionary) -> void:
	_camera = Camera3D.new()
	_camera.name = "DimetricCamera"
	_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	_camera.size = CAMERA_SIZE_1280X800
	_camera.current = true
	var extent_i: float = float(extents["i"])
	var extent_j: float = float(extents["j"])
	var target: Vector3 = Vector3(minf(extent_i * 0.38, 4.5), CAMERA_TARGET_HEIGHT, minf(extent_j * 0.38, 3.5))
	_camera.position = target + CAMERA_OFFSET
	add_child(_camera)
	_camera.look_at(target, Vector3.UP)


func _build_ground(cells: Array, textures: Dictionary) -> void:
	var parent: Node3D = Node3D.new()
	parent.name = "GroundCells"
	add_child(parent)
	for cell_variant: Variant in cells:
		var cell: Dictionary = cell_variant as Dictionary
		var i: int = int(cell["i"])
		var j: int = int(cell["j"])
		var mesh_instance: MeshInstance3D = MeshInstance3D.new()
		mesh_instance.name = "Cell_%d_%d" % [i, j]
		var mesh: QuadMesh = QuadMesh.new()
		mesh.size = Vector2.ONE
		mesh_instance.mesh = mesh
		mesh_instance.rotation.x = -PI / 2.0
		mesh_instance.position = Vector3(float(i), -0.006, float(j))
		var material: ShaderMaterial = ShaderMaterial.new()
		material.shader = GroundShader
		material.set_shader_parameter("swatch", textures["terrain/" + str(cell["material"])])
		material.set_shader_parameter("cell_origin", Vector2(float(i), float(j)))
		material.set_shader_parameter("wetness", 1.0 if "rain" in active_presets else 0.0)
		material.set_shader_parameter("time_tint", _ground_tint(str(cell["material"])))
		mesh_instance.material_override = material
		parent.add_child(mesh_instance)
		geometry_summary["ground_cells"] = int(geometry_summary["ground_cells"]) + 1


func _build_walls(runs: Array, textures: Dictionary) -> void:
	var parent: Node3D = Node3D.new()
	parent.name = "ConstructedWalls"
	add_child(parent)
	for run_variant: Variant in runs:
		_build_wall_run(parent, run_variant as Dictionary, textures)
	var first_run: Dictionary = runs[0] as Dictionary
	var corner: Array = first_run["start"] as Array
	_add_wall_box(
		parent,
		"CornerPost",
		Vector3(CORNER_POST_WIDTH, CAP_BOTTOM - SILL_TOP, CORNER_POST_WIDTH),
		Vector3(float(corner[0]), (SILL_TOP + CAP_BOTTOM) * 0.5, float(corner[1])),
		_wall_material(textures["walls/post"] as Texture2D, 1.0, 0.0),
	)
	geometry_summary["wall_posts"] = int(geometry_summary["wall_posts"]) + 1


func _build_wall_run(parent: Node3D, run: Dictionary, textures: Dictionary) -> void:
	var axis: String = str(run["axis"])
	var start_array: Array = run["start"] as Array
	var start: Vector2 = Vector2(float(start_array[0]), float(start_array[1]))
	var cells: float = float(run["cells"])
	var door: Variant = run["door_interval"]
	var lower_segments: Array[Vector2] = [Vector2(0.0, cells)]
	if door is Array:
		var door_array: Array = door as Array
		lower_segments = [Vector2(0.0, float(door_array[0])), Vector2(float(door_array[1]), cells)]
	_add_wall_layer(parent, axis, start, cells, lower_segments, 0.0, PLINTH_TOP, textures["walls/plinth"] as Texture2D, "Plinth")
	_add_wall_layer(parent, axis, start, cells, lower_segments, PLINTH_TOP, CAP_BOTTOM, textures["walls/plaster"] as Texture2D, "Plaster")
	_add_wall_layer(parent, axis, start, cells, lower_segments, PLINTH_TOP, SILL_TOP, textures["walls/sill"] as Texture2D, "Sill", 0.014)
	if door is Array:
		var interval: Array = door as Array
		var door_segment: Array[Vector2] = [Vector2(float(interval[0]), float(interval[1]))]
		_add_wall_layer(parent, axis, start, cells, door_segment, DOOR_HEIGHT, CAP_BOTTOM, textures["walls/plaster"] as Texture2D, "PlasterAboveDoor")
		_add_door(parent, axis, start, interval, textures)
	_add_wall_layer(parent, axis, start, cells, [Vector2(0.0, cells)], CAP_BOTTOM, CAP_TOP, textures["walls/cap_front"] as Texture2D, "CapFront")
	_add_cap_top(parent, axis, start, cells, textures["walls/cap_top"] as Texture2D)
	for boundary: int in range(1, int(cells) + 1):
		var position: Vector3 = _run_position(axis, start, float(boundary), (SILL_TOP + CAP_BOTTOM) * 0.5)
		var size: Vector3 = Vector3(POST_WIDTH, CAP_BOTTOM - SILL_TOP, WALL_THICKNESS + 0.018)
		if axis == "z":
			size = Vector3(WALL_THICKNESS + 0.018, CAP_BOTTOM - SILL_TOP, POST_WIDTH)
		_add_wall_box(parent, "Post_%s_%d" % [axis, boundary], size, position, _wall_material(textures["walls/post"] as Texture2D, 1.0, 0.0))
		geometry_summary["wall_posts"] = int(geometry_summary["wall_posts"]) + 1
	for panel: int in int(cells):
		if panel % 3 == 1 and (door is not Array or not _interval_contains_panel(door as Array, panel)):
			_add_brace(parent, axis, start, panel, textures["walls/post"] as Texture2D)


func _add_wall_layer(
	parent: Node3D,
	axis: String,
	start: Vector2,
	_run_length: float,
	segments: Array[Vector2],
	height_start: float,
	height_end: float,
	texture: Texture2D,
	label: String,
	thickness_extra: float = 0.0,
) -> void:
	for segment: Vector2 in segments:
		var length: float = segment.y - segment.x
		if length <= 0.0:
			continue
		var along: float = (segment.x + segment.y) * 0.5
		var position: Vector3 = _run_position(axis, start, along, (height_start + height_end) * 0.5)
		var size: Vector3 = Vector3(length, height_end - height_start, WALL_THICKNESS + thickness_extra)
		if axis == "z":
			size = Vector3(WALL_THICKNESS + thickness_extra, height_end - height_start, length)
		_add_wall_box(parent, "%s_%s_%.2f" % [label, axis, segment.x], size, position, _wall_material(texture, length, segment.x))


func _add_cap_top(parent: Node3D, axis: String, start: Vector2, cells: float, texture: Texture2D) -> void:
	var mesh_instance: MeshInstance3D = MeshInstance3D.new()
	mesh_instance.name = "CapTop_" + axis
	var mesh: QuadMesh = QuadMesh.new()
	mesh.size = Vector2(cells, WALL_THICKNESS)
	mesh_instance.mesh = mesh
	mesh_instance.rotation.x = -PI / 2.0
	if axis == "z":
		mesh_instance.rotation.y = PI / 2.0
	mesh_instance.position = _run_position(axis, start, cells * 0.5, CAP_TOP + 0.001)
	mesh_instance.material_override = _wall_material(texture, cells, 0.0)
	parent.add_child(mesh_instance)
	geometry_summary["wall_boxes"] = int(geometry_summary["wall_boxes"]) + 1


func _add_door(parent: Node3D, axis: String, start: Vector2, interval: Array, textures: Dictionary) -> void:
	var u0: float = float(interval[0])
	var u1: float = float(interval[1])
	var width: float = u1 - u0
	var centre: float = (u0 + u1) * 0.5
	var door: MeshInstance3D = MeshInstance3D.new()
	door.name = "DoorLeaf"
	var mesh: QuadMesh = QuadMesh.new()
	mesh.size = Vector2(width, DOOR_HEIGHT)
	door.mesh = mesh
	door.position = _run_position(axis, start, centre, DOOR_HEIGHT * 0.5)
	if axis == "x":
		door.position.z += WALL_THICKNESS * 0.5 + 0.002
	else:
		door.position.x += WALL_THICKNESS * 0.5 + 0.002
		door.rotation.y = PI / 2.0
	var material: StandardMaterial3D = _wall_material(textures["walls/door"] as Texture2D, 1.0, 0.0)
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	material.alpha_scissor_threshold = 0.12
	door.material_override = material
	parent.add_child(door)
	geometry_summary["wall_boxes"] = int(geometry_summary["wall_boxes"]) + 1
	var lintel_u0: float = floorf(centre) + DOOR_LINTEL_INSET
	var lintel_u1: float = floorf(centre) + 1.0 - DOOR_LINTEL_INSET
	var lintel_position: Vector3 = _run_position(axis, start, (lintel_u0 + lintel_u1) * 0.5, (DOOR_HEIGHT + LINTEL_TOP) * 0.5)
	var lintel_size: Vector3 = Vector3(lintel_u1 - lintel_u0, LINTEL_TOP - DOOR_HEIGHT, WALL_THICKNESS + 0.025)
	if axis == "z":
		lintel_size = Vector3(WALL_THICKNESS + 0.025, LINTEL_TOP - DOOR_HEIGHT, lintel_u1 - lintel_u0)
	_add_wall_box(parent, "DoorLintel", lintel_size, lintel_position, _wall_material(textures["walls/cap_front"] as Texture2D, 1.0, lintel_u0))


func _add_brace(parent: Node3D, axis: String, start: Vector2, panel: int, texture: Texture2D) -> void:
	var delta_along: float = 0.76
	var delta_height: float = CAP_BOTTOM - SILL_TOP - 0.12
	var length: float = Vector2(delta_along, delta_height).length()
	var position: Vector3 = _run_position(axis, start, float(panel) + 0.5, (SILL_TOP + CAP_BOTTOM) * 0.5)
	var brace: MeshInstance3D = MeshInstance3D.new()
	brace.name = "Brace_%s_%d" % [axis, panel]
	var mesh: BoxMesh = BoxMesh.new()
	mesh.size = Vector3(0.13, length, 0.032) if axis == "x" else Vector3(0.032, length, 0.13)
	brace.mesh = mesh
	brace.position = position
	if axis == "x":
		brace.position.z += WALL_THICKNESS * 0.5 + 0.018
		brace.rotation.z = -atan2(delta_along, delta_height)
	else:
		brace.position.x += WALL_THICKNESS * 0.5 + 0.018
		brace.rotation.x = atan2(delta_along, delta_height)
	brace.material_override = _wall_material(texture, 1.0, 0.0)
	parent.add_child(brace)
	geometry_summary["wall_braces"] = int(geometry_summary["wall_braces"]) + 1


func _add_wall_box(parent: Node3D, node_name: String, size: Vector3, position: Vector3, material: Material) -> void:
	var mesh_instance: MeshInstance3D = MeshInstance3D.new()
	mesh_instance.name = node_name
	var mesh: BoxMesh = BoxMesh.new()
	mesh.size = size
	mesh_instance.mesh = mesh
	mesh_instance.position = position
	mesh_instance.material_override = material
	parent.add_child(mesh_instance)
	geometry_summary["wall_boxes"] = int(geometry_summary["wall_boxes"]) + 1


func _wall_material(texture: Texture2D, scale_u: float, offset_u: float) -> StandardMaterial3D:
	var material: StandardMaterial3D = StandardMaterial3D.new()
	material.albedo_texture = texture
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST_WITH_MIPMAPS_ANISOTROPIC
	material.uv1_scale = Vector3(maxf(scale_u / 4.0, 0.25), 1.0, 1.0)
	material.uv1_offset = Vector3(offset_u / 4.0, 0.0, 0.0)
	material.roughness = 0.86
	return material


func _run_position(axis: String, start: Vector2, along: float, height: float) -> Vector3:
	return Vector3(start.x + (along if axis == "x" else 0.0), height, start.y + (along if axis == "z" else 0.0))


func _interval_contains_panel(interval: Array, panel: int) -> bool:
	var centre: float = float(panel) + 0.5
	return centre >= float(interval[0]) and centre <= float(interval[1])


func _build_props(props: Array, textures: Dictionary) -> void:
	var parent: Node3D = Node3D.new()
	parent.name = "BillboardProps"
	add_child(parent)
	for prop_variant: Variant in props:
		var row: Dictionary = prop_variant as Dictionary
		var kind: String = str(row["kind"])
		var anchor_array: Array = row["cell_anchor"] as Array
		var anchor: Vector2 = Vector2(float(anchor_array[0]), float(anchor_array[1]))
		var height: float = float(row["nominal_height"])
		_add_contact_shadow(parent, anchor, height)
		var sprite: Sprite3D = Sprite3D.new()
		sprite.name = "Prop_" + kind
		var texture: Texture2D = textures["props/" + kind] as Texture2D
		sprite.texture = texture
		sprite.billboard = BaseMaterial3D.BILLBOARD_FIXED_Y
		sprite.alpha_cut = SpriteBase3D.ALPHA_CUT_DISCARD
		sprite.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST_WITH_MIPMAPS_ANISOTROPIC
		sprite.pixel_size = height / float(texture.get_height())
		sprite.position = Vector3(anchor.x, height * 0.5, anchor.y)
		if bool(row["sway"]):
			var sway_material: ShaderMaterial = ShaderMaterial.new()
			sway_material.shader = SwayShader
			sway_material.set_shader_parameter("albedo_texture", texture)
			sway_material.set_shader_parameter("wind_strength", 1.0 if "wind" in active_presets else 0.12)
			sway_material.set_shader_parameter("time_offset", anchor.x * 0.73 + anchor.y * 1.13)
			sprite.material_override = sway_material
		parent.add_child(sprite)
		geometry_summary["props"] = int(geometry_summary["props"]) + 1


func _add_contact_shadow(parent: Node3D, anchor: Vector2, height: float) -> void:
	var shadow: MeshInstance3D = MeshInstance3D.new()
	shadow.name = "ContactShadow"
	var mesh: QuadMesh = QuadMesh.new()
	mesh.size = Vector2(clampf(height * 0.34, 0.24, 0.72), clampf(height * 0.13, 0.10, 0.28))
	shadow.mesh = mesh
	shadow.rotation.x = -PI / 2.0
	shadow.position = Vector3(anchor.x, 0.004, anchor.y)
	var material: StandardMaterial3D = StandardMaterial3D.new()
	material.albedo_color = Color(0.018, 0.025, 0.055, 0.42)
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	shadow.material_override = material
	parent.add_child(shadow)


func _build_lights(light_sources: Dictionary) -> void:
	var is_dusk: bool = "dusk" in active_presets
	var moon: DirectionalLight3D = DirectionalLight3D.new()
	moon.name = "WarmHorizonKey" if is_dusk else "CoolMoonlight"
	moon.light_color = Color("ffd6bb") if is_dusk else Color("a9caff")
	moon.light_energy = 0.44 if is_dusk else 0.86
	moon.shadow_enabled = true
	moon.rotation_degrees = Vector3(-24.0, 58.0, 0.0) if is_dusk else Vector3(-52.0, -38.0, 0.0)
	add_child(moon)
	geometry_summary["lights"] = int(geometry_summary["lights"]) + 1

	var lantern_position: Vector3 = _vector3_from_array(light_sources["lantern_glass"] as Array)
	_lantern_light = _warm_omni("LanternGlow", lantern_position, 1.35 if is_dusk else 2.45, 4.2)
	add_child(_lantern_light)
	geometry_summary["lights"] = int(geometry_summary["lights"]) + 1
	var random: RandomNumberGenerator = RandomNumberGenerator.new()
	random.seed = 0x544d455f4645454c
	_flicker_phase = random.randf_range(0.0, TAU)
	_flicker_energy = _lantern_light.light_energy
	var candle_index: int = 0
	for candle_variant: Variant in light_sources["candles"] as Array:
		var candle: OmniLight3D = _warm_omni(
			"Candle_%d" % candle_index,
			_vector3_from_array(candle_variant as Array),
			0.36 if is_dusk else 0.48,
			1.15,
		)
		add_child(candle)
		geometry_summary["lights"] = int(geometry_summary["lights"]) + 1
		candle_index += 1


func _warm_omni(node_name: String, position: Vector3, energy: float, range_value: float) -> OmniLight3D:
	var light: OmniLight3D = OmniLight3D.new()
	light.name = node_name
	light.position = position
	light.light_color = Color("ffb457")
	light.light_energy = energy
	light.omni_range = range_value
	light.omni_attenuation = 1.35
	light.shadow_enabled = true
	return light


func _build_rain() -> void:
	var rain: CPUParticles3D = CPUParticles3D.new()
	rain.name = "CompatibilityRain"
	rain.amount = 1080
	rain.lifetime = 1.15
	rain.preprocess = 1.15
	rain.emission_shape = CPUParticles3D.EMISSION_SHAPE_BOX
	rain.emission_box_extents = Vector3(7.0, 0.1, 7.0)
	rain.position = Vector3(4.0, 5.8, 3.5)
	rain.direction = Vector3(0.18, -1.0, 0.09)
	rain.spread = 3.0
	rain.gravity = Vector3(0.0, -5.0, 0.0)
	rain.initial_velocity_min = 7.5
	rain.initial_velocity_max = 9.2
	rain.scale_amount_min = 0.7
	rain.scale_amount_max = 1.2
	var streak: ArrayMesh = _rain_streak_mesh()
	var material: StandardMaterial3D = StandardMaterial3D.new()
	material.albedo_color = Color(0.65, 0.77, 0.94, 0.42)
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	streak.surface_set_material(0, material)
	rain.mesh = streak
	add_child(rain)


func _rain_streak_mesh() -> ArrayMesh:
	var half_width: float = 0.006
	var half_height: float = 0.04
	var wind_slant: float = 0.014
	var vertices: PackedVector3Array = PackedVector3Array([
		Vector3(-half_width, -half_height, 0.0),
		Vector3(half_width, -half_height, 0.0),
		Vector3(half_width - wind_slant, half_height, 0.0),
		Vector3(-half_width - wind_slant, half_height, 0.0),
	])
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_TEX_UV] = PackedVector2Array([
		Vector2(0.0, 1.0),
		Vector2(1.0, 1.0),
		Vector2(1.0, 0.0),
		Vector2(0.0, 0.0),
	])
	arrays[Mesh.ARRAY_INDEX] = PackedInt32Array([0, 1, 2, 0, 2, 3])
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _build_fog_veil() -> void:
	# Volumetric Environment fog is unavailable under Compatibility. A subtle
	# canvas shader veil keeps this preset on the contracted renderer without
	# asking the engine to silently ignore a Forward+ feature.
	var layer: CanvasLayer = CanvasLayer.new()
	layer.name = "CompatibilityFogVeil"
	layer.layer = 20
	var veil: ColorRect = ColorRect.new()
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	veil.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var material: ShaderMaterial = ShaderMaterial.new()
	material.shader = FogShader
	veil.material = material
	layer.add_child(veil)
	add_child(layer)


func _present_absence(reason: String) -> void:
	absence_visible = true
	_build_environment()
	var layer: CanvasLayer = CanvasLayer.new()
	layer.name = "AssetAbsence"
	var wash: ColorRect = ColorRect.new()
	wash.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	wash.color = Color("11182b")
	wash.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(wash)
	var panel: ColorRect = ColorRect.new()
	panel.set_anchors_preset(Control.PRESET_CENTER_TOP)
	panel.position = Vector2(-390.0, 54.0)
	panel.size = Vector2(780.0, 112.0)
	panel.color = Color(0.055, 0.075, 0.13, 0.96)
	panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(panel)
	var label: Label = Label.new()
	label.position = Vector2(24.0, 18.0)
	label.size = Vector2(732.0, 76.0)
	label.text = "FEEL SCENE — CANDIDATE ASSETS ABSENT\n" + reason
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.add_theme_color_override("font_color", Color("d8e1f2"))
	panel.add_child(label)
	add_child(layer)


func _resolve_presets(raw: String) -> PackedStringArray:
	var result: PackedStringArray = PackedStringArray()
	var source: String = raw.strip_edges().to_lower()
	if source.is_empty():
		source = DEFAULT_PRESET
	for part: String in source.split(",", false):
		var preset: String = part.strip_edges()
		if preset in KNOWN_PRESETS and preset not in result:
			result.append(preset)
	if result.is_empty():
		result.append(DEFAULT_PRESET)
	result.sort()
	return result


func _vector3_from_array(value: Array) -> Vector3:
	return Vector3(float(value[0]), float(value[1]), float(value[2]))


func _ground_tint(material_name: String) -> Color:
	if "dusk" in active_presets:
		return Color(1.0, 0.96, 0.90)
	if material_name == "grass":
		return Color(0.60, 0.72, 0.90)
	return Color(0.74, 0.82, 0.96)
