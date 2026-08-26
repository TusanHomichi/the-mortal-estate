extends RefCounted

var _support: TestSupport


func test_generated_manifest_loads_exact_roles_resources_and_chant_original() -> void:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	var manifest: Dictionary = loader.load_manifest()
	_support.expect(not manifest.is_empty(), "the generated audio manifest passes strict runtime validation: " + loader.last_error)
	_support.expect_equal(manifest.get("schema_version"), 1, "the projected client manifest is schema 1")
	_support.expect_equal((manifest.get("cues", []) as Array).size(), 10, "the projected client manifest contains the exact ten approved cue files")
	_support.expect_equal(loader.cues_for_role("combat_body_impact").size(), 2, "body impact is the only two-variant role")
	for role: String in AudioManifestLoader.EXPECTED_ROLES:
		var cues: Array[Dictionary] = loader.cues_for_role(role)
		_support.expect_equal(cues.size(), 2 if role == "combat_body_impact" else 1, role + " has its exact approved variant count")
		for cue: Dictionary in cues:
			_support.expect(cue.get("stream") is AudioStream, str(cue.get("id", "")) + " resolves to a decodable imported AudioStream")
	var chant: Dictionary = loader.cue("spell_chant_01")
	_support.expect_equal(chant.get("path"), "res://presentation/audio/assets/generated/spell_chant_01.wav", "the chant route uses the owner-provided original projection")
	_support.expect_equal(chant.get("byte_length"), 417180, "the chant byte length matches the verified original")
	_support.expect_equal(chant.get("sha256"), "2042752db95720f22ba66a20977266f732b6213c609a9986d08c5e420185e87c", "the chant hash matches the verified original")


func test_role_variants_and_pitch_are_deterministic_by_event_identity() -> void:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	_support.expect(not loader.load_manifest().is_empty(), "the manifest loads before deterministic selection")
	var identity: String = "event:connection-a:42:3"
	_support.expect_equal(loader.cue_for_role("combat_body_impact", identity).get("id"), loader.cue_for_role("combat_body_impact", identity).get("id"), "the same event identity selects the same body-impact variant")
	var selected_ids: Dictionary = {}
	for index: int in range(64):
		selected_ids[loader.cue_for_role("combat_body_impact", "event-%d" % index).get("id")] = true
	_support.expect_equal(selected_ids.size(), 2, "bounded deterministic selection reaches both approved impact variants")

	var player: AudioCuePlayer = _player(loader)
	var first: Dictionary = player.play_role("combat_swing", identity)
	var second: Dictionary = player.play_role("combat_swing", identity)
	_support.expect_equal(first.get("pitch_scale"), second.get("pitch_scale"), "the same event identity produces the same bounded pitch")
	_support.expect(float(first.get("pitch_scale", 0.0)) >= 0.96 and float(first.get("pitch_scale", 0.0)) <= 1.04, "swing pitch remains inside its approved range")
	player.free()


func test_voice_caps_mix_controls_and_discard_are_bounded() -> void:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	var player: AudioCuePlayer = _player(loader)
	for index: int in range(3):
		_support.expect(not player.play_role("combat_body_impact", "impact-%d" % index).is_empty(), "approved impact cue can play")
	_support.expect_equal(player.play_history.size(), 3, "every play request has a bounded diagnostic record")
	_support.expect(player.active_voice_count("combat_body_impact") <= 2, "body impacts never exceed the exact two-voice cap")
	_support.expect(not player.set_mix(false, 101), "out-of-range SFX volume fails closed")
	_support.expect(player.set_mix(true, 37), "valid mute and volume settings apply")
	var muted: Dictionary = player.play_role("ui_reject", "reject-muted")
	_support.expect_equal(muted.get("muted"), true, "muted playback records the cue without creating an audible voice")
	player.discard()
	_support.expect(player.play_history.is_empty() and player.active_voice_count("combat_body_impact") == 0, "discard clears playback history and live voices")
	player.free()


func test_runtime_resource_paths_fail_closed_outside_generated_audio_root() -> void:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	_support.expect(loader._safe_resource_path("res://presentation/audio/assets/generated/combat_swing_01.ogg"), "the exact generated audio root is accepted")
	for unsafe_path: String in [
		"res://content/audio/combat_swing_01.ogg",
		"res://Res" + "earch/audio/combat_swing_01.ogg",
		"res://place" + "holders/private/combat_swing_01.ogg",
		"res://presentation/audio/assets/generated/../combat_swing_01.ogg",
		"user://combat_swing_01.ogg",
	]:
		_support.expect(not loader._safe_resource_path(unsafe_path), "unsafe audio path fails closed: " + unsafe_path)


func test_export_import_fallback_does_not_weaken_source_byte_verification() -> void:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	var row: Dictionary = {"byte_length": 1, "sha256": "0".repeat(64)}
	var missing_path: String = "res://presentation/audio/assets/generated/not_present.ogg"
	_support.expect(
		loader._source_bytes_are_valid(row, missing_path, false),
		"an exported pack may rely on the separately checked decodable AudioStream import",
	)
	_support.expect(
		not loader._source_bytes_are_valid(row, missing_path, true),
		"source/editor proof still requires the exact original bytes",
	)


func _player(loader: AudioManifestLoader) -> AudioCuePlayer:
	var player: AudioCuePlayer = AudioCuePlayer.new()
	(Engine.get_main_loop() as SceneTree).root.add_child(player)
	_support.expect(player.configure(loader), "audio cue player configures from the strict generated manifest")
	return player
