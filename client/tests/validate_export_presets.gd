extends SceneTree

const PRESET_PATH: String = "res://export_presets.cfg"


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: PackedStringArray = PackedStringArray()
	var presets: ConfigFile = ConfigFile.new()
	var load_error: Error = presets.load(PRESET_PATH)
	_expect(load_error == OK, "export presets must parse", failures)
	if load_error == OK:
		_validate_project_renderer(failures)
		_validate_sections(presets, failures)
		_validate_native_3d_presentation(failures)
		_validate_audio_projection(failures)
	for failure: String in failures:
		printerr("export_presets/" + failure)
	if failures.is_empty():
		print("export_presets/three_internal_desktop_targets")
		quit(0)
	else:
		quit(1)


func _validate_project_renderer(failures: PackedStringArray) -> void:
	var features: PackedStringArray = ProjectSettings.get_setting("application/config/features", PackedStringArray()) as PackedStringArray
	_expect(features.has("4.7"), "project feature must remain 4.7", failures)
	_expect(features.has("GL Compatibility"), "project renderer must remain GL Compatibility", failures)


func _validate_sections(presets: ConfigFile, failures: PackedStringArray) -> void:
	var expected: Array[Dictionary] = [
		{"name": "Linux x86_64 (internal)", "platform": "Linux", "path": "build/linux/tme-client.x86_64", "runnable": true},
		{"name": "Windows x86_64 (internal)", "platform": "Windows Desktop", "path": "build/windows/tme-client.exe", "runnable": false},
		{"name": "macOS universal (internal)", "platform": "macOS", "path": "build/macos/tme-client.zip", "runnable": false},
	]
	var expected_sections: PackedStringArray = PackedStringArray([
		"preset.0", "preset.0.options",
		"preset.1", "preset.1.options",
		"preset.2", "preset.2.options",
	])
	var actual_sections: PackedStringArray = presets.get_sections()
	actual_sections.sort()
	expected_sections.sort()
	_expect(actual_sections == expected_sections, "exactly three preset and three option sections are required", failures)
	for index: int in range(expected.size()):
		var section: String = "preset." + str(index)
		var options: String = section + ".options"
		var row: Dictionary = expected[index]
		_expect(presets.get_value(section, "name", "") == row["name"], section + " name differs", failures)
		_expect(presets.get_value(section, "platform", "") == row["platform"], section + " platform differs", failures)
		_expect(presets.get_value(section, "export_path", "") == row["path"], section + " path differs", failures)
		_expect(presets.get_value(section, "runnable", false) == row["runnable"], section + " runnable flag differs", failures)
		_expect(presets.get_value(section, "export_filter", "") == "all_resources", section + " must export project resources", failures)
		_expect(str(presets.get_value(section, "include_filter", "")).is_empty(), section + " must not narrow included resources", failures)
		_expect(presets.get_value(section, "exclude_filter", "") == "tests/*", section + " must exclude non-shippable tests", failures)
		_expect(not bool(presets.get_value(section, "encrypt_pck", true)), section + " must not encrypt the PCK", failures)
		_expect(not bool(presets.get_value(section, "encrypt_directory", true)), section + " must not encrypt directories", failures)
		_expect(str(presets.get_value(section, "encryption_include_filters", "")).is_empty(), section + " must not name encryption filters", failures)
		_expect(str(presets.get_value(section, "encryption_exclude_filters", "")).is_empty(), section + " must not name encryption exclusions", failures)
		_expect(str(presets.get_value(options, "custom_template/debug", "")).is_empty(), section + " must not require a custom debug template", failures)
		_expect(str(presets.get_value(options, "custom_template/release", "")).is_empty(), section + " must not require a custom release template", failures)
	_expect(presets.get_value("preset.0.options", "binary_format/architecture", "") == "x86_64", "Linux must target x86_64", failures)
	_expect(presets.get_value("preset.1.options", "binary_format/architecture", "") == "x86_64", "Windows must target x86_64", failures)
	_expect(not bool(presets.get_value("preset.1.options", "codesign/enable", true)), "Windows distribution signing must be disabled", failures)
	_expect(presets.get_value("preset.2.options", "binary_format/architecture", "") == "universal", "macOS must include arm64 and x86_64", failures)
	_expect(int(presets.get_value("preset.2.options", "codesign/codesign", -1)) == 1, "macOS must use built-in ad-hoc signing", failures)
	_expect(presets.get_value("preset.2.options", "codesign/identity", "") == "-", "macOS signing identity must remain ad-hoc", failures)
	_expect(str(presets.get_value("preset.2.options", "codesign/certificate_file", "")).is_empty(), "macOS must not name a signing certificate", failures)
	_expect(str(presets.get_value("preset.2.options", "codesign/certificate_password", "")).is_empty(), "macOS must not contain a signing credential", failures)
	_expect(int(presets.get_value("preset.2.options", "notarization/notarization", -1)) == 0, "macOS notarization must be disabled", failures)
	for section_name: String in presets.get_sections():
		for key: String in presets.get_section_keys(section_name):
			var lowered: String = (section_name + "/" + key).to_lower()
			_expect(not lowered.contains("web") and not lowered.contains("android") and not lowered.contains("ios"), "web/mobile export surface is forbidden", failures)


func _validate_native_3d_presentation(failures: PackedStringArray) -> void:
	const THEME_PATH: String = "res://presentation/TmeTheme.tres"
	_expect(ResourceLoader.exists(THEME_PATH, "Theme"), "presentation theme must be exportable", failures)
	var theme := load(THEME_PATH) as Theme
	_expect(theme != null, "presentation theme must load", failures)
	_expect(not DirAccess.dir_exists_absolute("res://presentation/painterly"), "retired painterly runtime resources must not ship", failures)


func _validate_audio_projection(failures: PackedStringArray) -> void:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	var manifest: Dictionary = loader.load_manifest()
	_expect(not manifest.is_empty(), "audio projection schema 1 must load for export proof: " + loader.last_error, failures)
	var cues: Array = manifest.get("cues", [])
	_expect(cues.size() == 10, "export must reach exactly ten approved audio resources", failures)
	var paths: Dictionary = {}
	for cue_value: Variant in cues:
		if cue_value is not Dictionary:
			_expect(false, "projected audio cue must be an object", failures)
			continue
		var cue: Dictionary = cue_value as Dictionary
		var path: String = str(cue.get("path", ""))
		_expect(
			path.begins_with(AudioManifestLoader.GENERATED_ROOT)
			and (path.ends_with(".ogg") or path.ends_with(".wav"))
			and ".." not in path
			and "/addons/" not in path,
			"projected audio path must remain safe and client-local",
			failures,
		)
		_expect(not paths.has(path), "projected audio paths must be unique", failures)
		paths[path] = true
		_expect(FileAccess.file_exists(path), "projected audio bytes must exist", failures)
		_expect(ResourceLoader.exists(path, "AudioStream") and (load(path) as AudioStream) != null, "projected audio must import as AudioStream", failures)
		for dependency: String in ResourceLoader.get_dependencies(path):
			var lowered: String = dependency.to_lower()
			_expect(
				"content/" not in lowered
				and "research/" not in lowered
				and ("place" + "holders") + "/" not in lowered
				and "/addons/" not in lowered,
				"audio import dependency crosses an authoring/private boundary",
				failures,
			)
	var directory_files: PackedStringArray = DirAccess.get_files_at(AudioManifestLoader.GENERATED_ROOT)
	var audio_count: int = 0
	for file_name: String in directory_files:
		if file_name.ends_with(".ogg") or file_name.ends_with(".wav"):
			audio_count += 1
	_expect(audio_count == 10, "generated audio directory must contain only the ten projected sound files", failures)


func _expect(condition: bool, message: String, failures: PackedStringArray) -> void:
	if not condition:
		failures.append(message)
