extends RefCounted

## The viewports the beat band is asserted at. The first is the declared minimum
## resolution. The second is the window every proof capture is taken in
## (`tools/run_pulse_capture.py`, `tools/run_fixture_land_capture.py`), which is
## **narrower than the declared minimum** — so a rail that is comfortable at the
## floor can still be broken in every picture the owner reviews, and was.
const RAIL_WIDTH_VIEWPORTS: Array[Vector2i] = [Vector2i(1280, 720), Vector2i(1024, 768)]

## Input bindings, stored preferences, and the accessibility floor.
##
## What a key is bound to, what may be persisted about it, and the layout and
## focus rules a player who cannot use a mouse depends on. None of this is about
## the world; all of it is about reaching the world.

const ShellSupport: Script = preload("res://tests/shell_test_support.gd")

var _support: TestSupport


func test_every_screen_scene_instantiates() -> void:
	for path: String in ShellSupport.SCREEN_PATHS:
		var packed: PackedScene = load(path) as PackedScene
		_support.expect(packed != null, path + " must load")
		if packed == null:
			continue
		var instance: Node = packed.instantiate()
		_support.expect(instance is Control, path + " must instantiate as Control")
		instance.free()


func test_binding_store_allowlists_only_input_and_accessibility_keys() -> void:
	var store: BindingStore = BindingStore.new()
	var binding_document: Dictionary = store.binding_document()
	_support.expect_equal(ShellSupport.sorted_keys(binding_document), ["actions", "schema_version"], "binding document has the exact v2 keys")
	_support.expect_equal(binding_document["schema_version"], 2, "binding persistence cuts directly to schema 2")
	var expected_action_keys: Array[String] = []
	for action_name: String in InputActions.action_names():
		expected_action_keys.append(action_name)
	expected_action_keys.sort()
	_support.expect_equal(ShellSupport.sorted_keys(binding_document["actions"]), expected_action_keys, "binding actions are the exact Tme names")
	var accessibility_document: Dictionary = store.accessibility_document()
	_support.expect_equal(ShellSupport.sorted_keys(accessibility_document), ["schema_version", "sfx_muted", "sfx_volume_percent", "ui_text_scale_percent"], "accessibility document has the exact v2 keys")
	_support.expect_equal(accessibility_document["schema_version"], 2, "accessibility persistence cuts directly to schema 2")
	for forbidden: String in ["endpoint", "account", "character", "cookie", "csrf", "ticket", "password"]:
		_support.expect(not ShellSupport.dictionary_has_key_fragment(binding_document, forbidden), "binding document excludes " + forbidden)
		_support.expect(not ShellSupport.dictionary_has_key_fragment(accessibility_document, forbidden), "accessibility document excludes " + forbidden)
	var binding_path: String = "user://tme_phase_b_invalid_bindings.json"
	var invalid_bindings: Dictionary = binding_document.duplicate(true)
	invalid_bindings["endpoint"] = "synthetic.invalid"
	_support.expect(ShellSupport.write_json(binding_path, invalid_bindings), "invalid binding test document writes")
	_support.expect(not store.load_bindings(binding_path), "unknown binding key rejects the whole file")
	ShellSupport.remove_file(binding_path)
	var accessibility_path: String = "user://tme_phase_b_invalid_accessibility.json"
	_support.expect(ShellSupport.write_json(accessibility_path, {"schema_version": 2, "ui_text_scale_percent": 125, "sfx_muted": false, "sfx_volume_percent": 80, "identity": "synthetic"}), "invalid accessibility test document writes")
	_support.expect(not store.load_accessibility(accessibility_path), "unknown accessibility key rejects the whole file")
	_support.expect_equal(store.text_scale_percent, 100, "rejected accessibility restores 100 percent")
	ShellSupport.remove_file(accessibility_path)


func test_binding_store_round_trips_keyboard_mouse_and_joypad_rebinds() -> void:
	var path: String = "user://tme_phase_b_round_trip_bindings.json"
	ShellSupport.remove_file(path)
	var store: BindingStore = BindingStore.new()
	var defaults: Dictionary = store.binding_document()
	_support.expect(store.replace_keyboard("tme_move_north", 82, true, false, false, false), "keyboard replacement succeeds")
	_support.expect(store.replace_mouse_button("tme_world_primary", 3), "mouse replacement succeeds")
	_support.expect(store.replace_joypad_button("tme_reconnect", 4), "joypad replacement succeeds")
	var saved: Dictionary = store.binding_document()
	_support.expect(store.save_bindings(path), "typed replacement document saves")
	store.replace_keyboard("tme_move_north", 84)
	store.replace_mouse_button("tme_world_primary", 2)
	store.replace_joypad_button("tme_reconnect", 5)
	_support.expect(store.load_bindings(path), "typed replacement document reloads")
	_support.expect_equal(store.binding_document(), saved, "keyboard, mouse, and joypad records round trip exactly")
	_support.expect(store.reset_bindings(path), "reset removes the user override")
	_support.expect_equal(store.binding_document(), defaults, "reset restores project defaults")
	_support.expect(not FileAccess.file_exists(path), "reset removes only the test override file")


func test_schema_two_accessibility_and_schema_one_interaction_preferences_are_strict() -> void:
	_support.expect_equal(BindingStore.BINDINGS_PATH, "user://tme_input_bindings_v2.json", "binding store does not consult the retired v1 path")
	_support.expect_equal(BindingStore.ACCESSIBILITY_PATH, "user://tme_accessibility_v2.json", "accessibility store does not consult the retired v1 path")
	var accessibility_path: String = "user://tme_fj_accessibility_round_trip.json"
	ShellSupport.remove_file(accessibility_path)
	var store: BindingStore = BindingStore.new()
	_support.expect(store.set_text_scale(175), "supported text scale is accepted")
	store.set_sfx_muted(true)
	_support.expect(store.set_sfx_volume(37), "bounded SFX volume is accepted")
	_support.expect(not store.set_sfx_volume(101), "out-of-range SFX volume is rejected")
	_support.expect(store.save_accessibility(accessibility_path), "schema-2 accessibility saves")
	var loaded: BindingStore = BindingStore.new()
	_support.expect(loaded.load_accessibility(accessibility_path), "schema-2 accessibility loads")
	_support.expect_equal(loaded.accessibility_document(), {"schema_version": 2, "ui_text_scale_percent": 175, "sfx_muted": true, "sfx_volume_percent": 37}, "text and SFX settings round trip exactly")
	_support.expect(ShellSupport.write_json(accessibility_path, {"schema_version": 1, "ui_text_scale_percent": 200}), "retired schema fixture writes")
	_support.expect(not loaded.load_accessibility(accessibility_path), "schema 1 is rejected rather than migrated")
	_support.expect_equal(loaded.accessibility_document(), {"schema_version": 2, "ui_text_scale_percent": 100, "sfx_muted": false, "sfx_volume_percent": 100}, "invalid accessibility resets to project defaults")
	ShellSupport.remove_file(accessibility_path)

	var preference_path: String = "user://tme_fj_interaction_preferences_round_trip.json"
	ShellSupport.remove_file(preference_path)
	var preferences: InteractionPreferences = InteractionPreferences.new()
	_support.expect_equal(preferences.ranged_mode("character-a"), "jumpkick", "Leap is the per-character default")
	_support.expect(preferences.set_ranged_mode("character-a", "shoot"), "Shoot is a finite preference")
	_support.expect(preferences.set_ranged_mode("character-b", "throw"), "Throw is a finite preference")
	_support.expect(not preferences.set_ranged_mode("character-a", "fight"), "close mode cannot enter ranged preferences")
	_support.expect(preferences.save(preference_path), "preferences save")
	var restored: InteractionPreferences = InteractionPreferences.new()
	_support.expect(restored.load(preference_path), "preferences load")
	_support.expect_equal(restored.ranged_mode("character-a"), "shoot", "preference is keyed by selected character")
	_support.expect_equal(restored.ranged_mode("character-b"), "throw", "a second character retains its own preference")
	_support.expect(ShellSupport.write_json(preference_path, {"schema_version": 1, "characters": {"character-a": {"ranged_mode": "fallback"}}}), "invalid finite preference fixture writes")
	_support.expect(not restored.load(preference_path), "unknown preference resets the entire document")
	_support.expect_equal(restored.ranged_mode("character-a"), "jumpkick", "invalid preference resets safely to Leap")
	ShellSupport.remove_file(preference_path)


func test_essential_state_has_text_or_icon_beside_color() -> void:
	var character: CharacterSelectScreen = ShellSupport.add_screen("res://scenes/CharacterSelectScreen.tscn") as CharacterSelectScreen
	character.render_bootstrap({"player_kill_marks": {"active_count": 2, "gameplay_locked": true}, "characters": []})
	_support.expect("◆" in character.mark_status.text and "LOCKED" in character.mark_status.text, "gameplay lock uses icon and text")
	character.free()
	var world: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	world.set_connection_state("ONLINE", true)
	_support.expect("● Connection: ONLINE" in world.connection_status.text, "online state uses icon and text")
	world.present_frame(ShellSupport.frame_with_options([{"id": "action:disabled", "label": "Wait", "enabled": false, "blocked_reason": "server unavailable", "intent": null}]), 1)
	world.open_context_palette()
	var disabled_button: Button = world.context_palette.buttons()[0]
	_support.expect("[Unavailable: server unavailable]" in disabled_button.text, "disabled option explains state in text")
	world.free()


func test_character_touch_selection_requires_an_explicit_continue_action() -> void:
	var screen: CharacterSelectScreen = ShellSupport.add_screen("res://scenes/CharacterSelectScreen.tscn") as CharacterSelectScreen
	var emitted: Array[String] = []
	screen.character_selected.connect(func(character_id: String) -> void: emitted.append(character_id))
	screen.render_bootstrap({
		"player_kill_marks": {"active_count": 0, "gameplay_locked": false},
		"characters": [
			{"slot": 1, "display_name": "First", "character_id": "character-a"},
			{"slot": 2, "display_name": "Second", "character_id": "character-b"},
		],
	})
	_support.expect(screen.continue_button.disabled, "Continue starts disabled until the player selects a character")
	screen._character_buttons[1].pressed.emit()
	_support.expect_equal(emitted, [], "touching a character does not silently advance to another screen")
	_support.expect_equal(screen.selected_character_id(), "character-b", "touching a row selects that exact character")
	_support.expect(not screen.continue_button.disabled, "touching a row enables the visible Continue action")
	_support.expect_equal(screen.get_viewport().gui_get_focus_owner(), screen.continue_button, "selection leaves focus on Continue without making focus a prerequisite")
	screen.continue_button.pressed.emit()
	_support.expect_equal(emitted, ["character-b"], "Continue emits the selected character")
	screen.free()


func test_input_map_contains_only_exact_namespaced_actions() -> void:
	var actual: PackedStringArray = PackedStringArray()
	for action_value: Variant in InputMap.get_actions():
		var action_name: String = str(action_value)
		if action_name.begins_with("tme_"):
			actual.append(action_name)
	actual.sort()
	var expected_names: PackedStringArray = InputActions.action_names()
	expected_names.sort()
	_support.expect_equal(actual, expected_names, "InputMap has exactly the twenty-six namespaced Tme actions")
	var expected: Dictionary = ShellSupport.expected_default_binding_signatures()
	for action_name: String in InputActions.action_names():
		_support.expect(InputMap.has_action(action_name), action_name + " exists")
		_support.expect_equal(ShellSupport.event_signatures(InputMap.action_get_events(action_name)), expected[action_name], action_name + " has exact ordered defaults")


func test_keyboard_focus_order_and_modal_restoration() -> void:
	var login: LoginScreen = ShellSupport.add_screen("res://scenes/LoginScreen.tscn") as LoginScreen
	_support.expect(not login.username_edit.focus_neighbor_bottom.is_empty(), "username has explicit next focus")
	_support.expect(not login.login_value_edit.focus_neighbor_top.is_empty(), "password has explicit previous focus")
	_support.expect("saves no credential" in login.credential_source.text, "the login screen states that it stores nothing")
	_support.expect(login.username_edit.virtual_keyboard_enabled, "username explicitly permits a virtual keyboard")
	_support.expect(login.login_value_edit.virtual_keyboard_type == DisplayServer.KEYBOARD_TYPE_PASSWORD, "password requests the password keyboard type")
	_support.expect(not login.login_button.focus_neighbor_bottom.is_empty(), "login button closes the deterministic focus loop")
	login.free()
	var holder: Control = Control.new()
	ShellSupport.add_to_tree(holder)
	var origin: Button = Button.new()
	origin.text = "Origin"
	holder.add_child(origin)
	var dialog: TmeConfirmationDialog = (load("res://scenes/components/TmeConfirmationDialog.tscn") as PackedScene).instantiate() as TmeConfirmationDialog
	holder.add_child(dialog)
	origin.grab_focus()
	dialog.open_confirmation("Synthetic action", "Synthetic target", 4, origin)
	_support.expect(dialog.cancel_has_default_focus(), "confirmation focuses Cancel by default")
	dialog.cancel()
	_support.expect_equal(holder.get_viewport().gui_get_focus_owner(), origin, "modal closure restores the valid origin")
	holder.free()


## The readiness line carries the beat in words, and it is the same string the
## meter draws. Nothing here asserts that it is *wide*; it asserts that it is
## **whole**.
##
## The first version of this test asserted a 48 px width floor, which passed
## while the line read `◆ Ready · beat …` in every capture — the wait, the
## frame's own times, and the prepared band all ellipsized away. A floor is the
## wrong invariant: the right one is that the text is never trimmed, that at the
## ordinary text scale the whole sentence fits on one line at both the declared
## minimum resolution and the narrower window captures are taken in, and that
## when enlarged text does force a wrap the rail grows to show every line rather
## than clipping them.
##
## The fixture is deliberately the **longest** state the line has: a wait of
## several beats, an unmeasured fill, both of the frame's times, and a warmed
## spell in the preparation band.
func test_the_readiness_line_is_never_truncated() -> void:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	for viewport: Vector2i in RAIL_WIDTH_VIEWPORTS:
		tree.root.size = viewport
		screen.size = Vector2(viewport)
		for percent: int in [100, 200]:
			screen.call("apply_text_scale", percent)
			screen.present_frame(ShellSupport.longest_beat_frame(), 1)
			for _pass: int in 5:
				await tree.process_frame
			var label: Label = screen.readiness_status
			var where: String = "%dx%d at %d percent" % [viewport.x, viewport.y, percent]
			_support.expect_equal(
				label.text,
				screen.pulse_meter.meter_text(),
				"the readiness line is the meter's own sentence at " + where,
			)
			_support.expect(
				label.text.contains("preparing") and label.text.contains("world T"),
				"the fixture is the longest state the line has at " + where,
			)
			_support.expect_equal(
				label.text_overrun_behavior,
				TextServer.OVERRUN_NO_TRIMMING,
				"the readiness line is never trimmed at " + where,
			)
			_support.expect(
				label.get_rect().size.y + 0.5 >= label.get_combined_minimum_size().y,
				"every line the readiness text wraps to is drawn at %s: %d px of room for %d px of text" % [
					where, int(label.get_rect().size.y), int(label.get_combined_minimum_size().y)
				],
			)
			_support.expect(
				screen.hud.stacked_rail_height_is_bounded(),
				"the rail stays inside its viewport bound at " + where,
			)
			if percent == 100:
				var natural: float = ShellSupport.unwrapped_text_width(label)
				_support.expect(
					natural <= label.get_rect().size.x,
					"the whole sentence fits one line at %s: needs %d px, has %d px" % [
						where, int(natural), int(label.get_rect().size.x)
					],
				)
	screen.call("apply_text_scale", 100)
	tree.root.size = Vector2i(1280, 720)
	screen.free()


func test_minimum_1280x720_layout_has_no_clipped_essential_controls() -> void:
	for path: String in ShellSupport.SCREEN_PATHS:
		var screen: Control = ShellSupport.add_screen(path)
		screen.call("apply_text_scale", 200)
		_support.expect_equal(screen.size, Vector2(1280, 720), path + " receives the exact minimum viewport at 200 percent")
		_support.expect(screen.anchor_left == 0.0 and screen.anchor_top == 0.0 and screen.anchor_right == 0.0 and screen.anchor_bottom == 0.0, path + " smoke fixture owns a deterministic 1280x720 rectangle")
		screen.call("apply_text_scale", 100)
		_support.expect_equal(screen.size, Vector2(1280, 720), path + " remains at the exact minimum viewport at 100 percent")
		screen.free()
