class_name TextEntryKeyboard
extends RefCounted

const STRATEGY_NATIVE: String = "godot_native"
const STRATEGY_STEAM_OVERLAY: String = "steam_overlay"
const STRATEGY_MANUAL: String = "manual"

var _fields: Array[LineEdit] = []


func bind(field: LineEdit, keyboard_type: int = DisplayServer.KEYBOARD_TYPE_DEFAULT) -> void:
	if field == null or field in _fields:
		return
	_fields.append(field)
	field.virtual_keyboard_enabled = true
	field.virtual_keyboard_show_on_focus = false
	field.virtual_keyboard_type = keyboard_type
	field.focus_entered.connect(request_for.bind(field))


func request_for(field: LineEdit) -> bool:
	if field == null:
		return false
	var field_screen_rect: Rect2 = screen_rect_for(field)
	if DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD):
		var maximum: int = field.max_length if field.max_length > 0 else -1
		DisplayServer.virtual_keyboard_show(
			field.text,
			field_screen_rect,
			_display_keyboard_type(field.virtual_keyboard_type),
			maximum,
			field.caret_column,
			field.caret_column,
		)
		return true
	if strategy() == STRATEGY_STEAM_OVERLAY:
		return OS.shell_open(steam_overlay_uri(field_screen_rect)) == OK
	return false


func _display_keyboard_type(line_edit_type: int) -> DisplayServer.VirtualKeyboardType:
	match line_edit_type:
		LineEdit.KEYBOARD_TYPE_MULTILINE:
			return DisplayServer.KEYBOARD_TYPE_MULTILINE
		LineEdit.KEYBOARD_TYPE_NUMBER:
			return DisplayServer.KEYBOARD_TYPE_NUMBER
		LineEdit.KEYBOARD_TYPE_NUMBER_DECIMAL:
			return DisplayServer.KEYBOARD_TYPE_NUMBER_DECIMAL
		LineEdit.KEYBOARD_TYPE_PHONE:
			return DisplayServer.KEYBOARD_TYPE_PHONE
		LineEdit.KEYBOARD_TYPE_EMAIL_ADDRESS:
			return DisplayServer.KEYBOARD_TYPE_EMAIL_ADDRESS
		LineEdit.KEYBOARD_TYPE_PASSWORD:
			return DisplayServer.KEYBOARD_TYPE_PASSWORD
		LineEdit.KEYBOARD_TYPE_URL:
			return DisplayServer.KEYBOARD_TYPE_URL
	return DisplayServer.KEYBOARD_TYPE_DEFAULT


func hide() -> void:
	if DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD):
		DisplayServer.virtual_keyboard_hide()
	elif strategy() == STRATEGY_STEAM_OVERLAY:
		OS.shell_open("steam://close/keyboard")


func strategy() -> String:
	if DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD):
		return STRATEGY_NATIVE
	if steam_overlay_available_for(
		OS.get_name(),
		OS.get_distribution_name(),
		OS.get_environment("SteamDeck"),
		OS.get_environment("SteamTenfoot"),
	):
		return STRATEGY_STEAM_OVERLAY
	return STRATEGY_MANUAL


static func steam_overlay_uri(rect: Rect2) -> String:
	return "steam://open/keyboard?XPosition=%d&YPosition=%d&Width=%d&Height=%d&Mode=0" % [
		maxi(0, roundi(rect.position.x)),
		maxi(0, roundi(rect.position.y)),
		maxi(0, roundi(rect.size.x)),
		maxi(0, roundi(rect.size.y)),
	]


static func screen_rect_for(field: Control) -> Rect2:
	if field == null or field.get_viewport() == null:
		return Rect2()
	# Control.get_global_rect() is canvas-space. Virtual-keyboard APIs need the
	# actual window/screen rectangle after viewport stretching (notably the
	# 1920x1080 client canvas displayed in a 1280x800 Steam Deck window).
	return field.get_viewport().get_screen_transform() * field.get_global_rect()


static func steam_overlay_available_for(
	os_name: String,
	distribution_name: String,
	steam_deck: String,
	steam_tenfoot: String,
) -> bool:
	if os_name != "Linux":
		return false
	return (
		steam_deck == "1"
		or steam_tenfoot == "1"
		or distribution_name.to_lower().contains("steamos")
	)
