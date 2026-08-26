extends RefCounted

var _support: TestSupport


func test_steam_deck_and_tenfoot_linux_sessions_select_the_overlay() -> void:
	_support.expect(
		TextEntryKeyboard.steam_overlay_available_for("Linux", "SteamOS", "", ""),
		"SteamOS selects the overlay even without inherited shortcut flags",
	)
	_support.expect(
		TextEntryKeyboard.steam_overlay_available_for("Linux", "Arch Linux", "1", ""),
		"SteamDeck environment selects the overlay",
	)
	_support.expect(
		TextEntryKeyboard.steam_overlay_available_for("Linux", "Arch Linux", "", "1"),
		"SteamTenfoot environment selects the overlay",
	)
	_support.expect(
		not TextEntryKeyboard.steam_overlay_available_for("Windows", "", "1", "1"),
		"non-Linux platforms do not receive a Steam URI fallback",
	)


func test_overlay_uri_carries_only_the_text_field_rectangle() -> void:
	var uri: String = TextEntryKeyboard.steam_overlay_uri(Rect2(-4.0, 20.4, 320.2, 48.7))
	_support.expect_equal(
		uri,
		"steam://open/keyboard?XPosition=0&YPosition=20&Width=320&Height=49&Mode=0",
		"overlay URI uses the bounded field rectangle and single-line mode",
	)
	_support.expect(not uri.contains("text") and not uri.contains("password"), "overlay URI never carries field contents")


func test_field_rectangle_is_transformed_from_canvas_to_physical_screen() -> void:
	var holder: Control = Control.new()
	(Engine.get_main_loop() as SceneTree).root.add_child(holder)
	holder.position = Vector2(100.0, 40.0)
	holder.size = Vector2(300.0, 60.0)
	var expected: Rect2 = holder.get_viewport().get_screen_transform() * holder.get_global_rect()
	_support.expect_equal(
		TextEntryKeyboard.screen_rect_for(holder),
		expected,
		"keyboard geometry follows the viewport's canvas-to-screen transform",
	)
	holder.get_parent().remove_child(holder)
	holder.free()


func test_line_edit_binding_is_explicit_and_idempotent() -> void:
	var field: LineEdit = LineEdit.new()
	var keyboard: TextEntryKeyboard = TextEntryKeyboard.new()
	keyboard.bind(field, DisplayServer.KEYBOARD_TYPE_PASSWORD)
	keyboard.bind(field, DisplayServer.KEYBOARD_TYPE_DEFAULT)
	_support.expect(field.virtual_keyboard_enabled, "text field explicitly enables a virtual keyboard")
	_support.expect(not field.virtual_keyboard_show_on_focus, "the coordinator owns the single explicit show request")
	_support.expect_equal(field.virtual_keyboard_type, DisplayServer.KEYBOARD_TYPE_PASSWORD, "first binding preserves the password keyboard type")
	_support.expect_equal(field.focus_entered.get_connections().size(), 1, "repeated binding does not duplicate overlay requests")
	field.free()
