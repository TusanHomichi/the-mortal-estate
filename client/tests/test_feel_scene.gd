extends RefCounted

const FeelSceneResource: PackedScene = preload("res://presentation/feel/FeelScene.tscn")
const FeelAssetLoaderScript: Script = preload("res://presentation/feel/feel_asset_loader.gd")
const ASSET_VARIABLE: String = "TME_FEEL_ASSETS"
const PRESET_VARIABLE: String = "TME_FEEL_PRESET"
const FIXTURE_MANIFEST: String = "res://tests/fixtures/feel/feel-manifest.json"

var _support: TestSupport


func test_unset_asset_root_presents_absence_without_failing_the_scene() -> void:
	var original_assets: String = OS.get_environment(ASSET_VARIABLE)
	OS.set_environment(ASSET_VARIABLE, "")
	var scene: FeelScene = FeelSceneResource.instantiate() as FeelScene
	(Engine.get_main_loop() as SceneTree).root.add_child(scene)
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(scene.absence_visible, "an unset asset root is stated inside the picture")
	_support.expect(not scene.built_ok, "absence never masquerades as a built feel scene")
	_support.expect(scene.load_error.contains(ASSET_VARIABLE), "the absence reason names the missing configuration")
	_support.expect(scene.get_node_or_null("AssetAbsence") != null, "the in-picture absence banner is mounted")
	scene.free()
	OS.set_environment(ASSET_VARIABLE, original_assets)


func test_synthetic_fixture_verifies_and_builds_real_geometry() -> void:
	var original_assets: String = OS.get_environment(ASSET_VARIABLE)
	var original_preset: String = OS.get_environment(PRESET_VARIABLE)
	var fixture_root: String = _write_fixture("valid")
	OS.set_environment(ASSET_VARIABLE, fixture_root)
	OS.set_environment(PRESET_VARIABLE, "night,rain,fog,wind")
	var scene: FeelScene = FeelSceneResource.instantiate() as FeelScene
	(Engine.get_main_loop() as SceneTree).root.add_child(scene)
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(scene.built_ok, "a digest-bound synthetic packet builds the experiment [" + scene.load_error + "]")
	_support.expect(not scene.absence_visible, "a valid packet does not show the absence banner")
	_support.expect_equal(scene.geometry_summary["ground_cells"], 4, "one ground quad is built per material-plan cell")
	_support.expect(int(scene.geometry_summary["wall_boxes"]) > 8, "the wall is member geometry rather than one billboard")
	_support.expect_equal(scene.geometry_summary["props"], 5, "every required billboard prop is placed")
	_support.expect(scene.get_node_or_null("CompatibilityRain") is CPUParticles3D, "rain uses Compatibility-safe CPU particles")
	_support.expect(scene.get_node_or_null("CompatibilityFogVeil") is CanvasLayer, "fog uses the Compatibility shader veil")
	scene.free()
	OS.set_environment(ASSET_VARIABLE, original_assets)
	OS.set_environment(PRESET_VARIABLE, original_preset)


func test_tampered_asset_is_refused_before_decode() -> void:
	var fixture_root: String = _write_fixture("tampered")
	var image_file: FileAccess = FileAccess.open(fixture_root.path_join("synthetic.png"), FileAccess.WRITE)
	_support.expect(image_file != null, "the synthetic asset is writable for the tamper mutant")
	if image_file != null:
		image_file.store_8(0x54)
		image_file.close()
	var loader: FeelAssetLoader = FeelAssetLoaderScript.new() as FeelAssetLoader
	var result: Dictionary = loader.load_root(fixture_root)
	_support.expect_reject(result, "a changed source byte invalidates the packet")
	_support.expect(str(result.get("error", "")).contains("digest"), "the refusal identifies digest evidence")


func test_ruled_camera_projects_one_cell_to_a_224_pixel_diamond() -> void:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	var original_assets: String = OS.get_environment(ASSET_VARIABLE)
	var original_size: Vector2i = tree.root.size
	OS.set_environment(ASSET_VARIABLE, _write_fixture("camera"))
	tree.root.size = Vector2i(1280, 800)
	var scene: FeelScene = FeelSceneResource.instantiate() as FeelScene
	tree.root.add_child(scene)
	await tree.process_frame
	await tree.process_frame
	var width: float = scene.projected_cell_diamond_width()
	_support.expect(absf(width - 224.0) <= 0.25, "the unprojected cell corners span exactly 224 px at 1280x800 (got %.3f; %s)" % [width, scene.load_error])
	if scene.camera() != null:
		_support.expect_equal(scene.camera().projection, Camera3D.PROJECTION_ORTHOGONAL, "the ruled camera is orthographic")
	else:
		_support.expect(false, "the valid fixture builds the ruled camera [" + scene.load_error + "]")
	scene.free()
	tree.root.size = original_size
	OS.set_environment(ASSET_VARIABLE, original_assets)


func _write_fixture(name: String) -> String:
	var root: String = "user://feel-scene-fixtures/" + name
	DirAccess.make_dir_recursive_absolute(root)
	var image: Image = Image.create(4, 4, false, Image.FORMAT_RGBA8)
	image.fill(Color(0.36, 0.47, 0.41, 1.0))
	image.set_pixel(0, 0, Color(0.68, 0.72, 0.76, 0.0))
	var image_path: String = root.path_join("synthetic.png")
	_support.expect_equal(image.save_png(image_path), OK, "the synthetic PNG fixture is written")
	_support.expect_equal(
		_sha256(image_path),
		"e38bacdc604e05eb793c1404ffa57fd6ec3e46ec1ce3e27f52900789dd1645ab",
		"the procedural 4px fixture stays byte-stable for the tracked manifest",
	)
	var source_manifest: FileAccess = FileAccess.open(FIXTURE_MANIFEST, FileAccess.READ)
	_support.expect(source_manifest != null, "the tracked synthetic manifest is readable")
	var manifest_file: FileAccess = FileAccess.open(root.path_join("feel-manifest.json"), FileAccess.WRITE)
	_support.expect(manifest_file != null, "the synthetic manifest fixture is writable")
	if manifest_file != null and source_manifest != null:
		manifest_file.store_string(source_manifest.get_as_text())
		manifest_file.close()
	if source_manifest != null:
		source_manifest.close()
	return ProjectSettings.globalize_path(root)


func _sha256(path: String) -> String:
	var hashing: HashingContext = HashingContext.new()
	hashing.start(HashingContext.HASH_SHA256)
	hashing.update(FileAccess.get_file_as_bytes(path))
	return hashing.finish().hex_encode()
