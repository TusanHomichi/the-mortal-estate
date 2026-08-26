class_name BindingStore
extends RefCounted

const BINDINGS_PATH: String = "user://tme_input_bindings_v2.json"
const ACCESSIBILITY_PATH: String = "user://tme_accessibility_v2.json"
const MAX_SETTINGS_BYTES: int = 65536
const MAX_SETTINGS_DEPTH: int = 8
const ALLOWED_TEXT_SCALES: Array[int] = [100, 125, 150, 175, 200]

var text_scale_percent: int = 100
var sfx_muted: bool = false
var sfx_volume_percent: int = 100
var _defaults: Dictionary = {}


func _init() -> void:
	_capture_project_defaults()


func replace_action_bindings(action_name: String, records: Array[Dictionary]) -> bool:
	if not InputActions.is_tme_action(action_name) or not _records_are_valid(records):
		return false
	_apply_records(action_name, records)
	return true


func replace_keyboard(action_name: String, physical_keycode: int, shift: bool = false, ctrl: bool = false, alt: bool = false, meta: bool = false) -> bool:
	var records: Array[Dictionary] = [{
		"kind": "key",
		"physical_keycode": physical_keycode,
		"shift": shift,
		"ctrl": ctrl,
		"alt": alt,
		"meta": meta,
	}]
	return replace_action_bindings(action_name, records)


func replace_mouse_button(action_name: String, button_index: int) -> bool:
	var records: Array[Dictionary] = [{"kind": "mouse_button", "button_index": button_index}]
	return replace_action_bindings(action_name, records)


func replace_joypad_button(action_name: String, button_index: int) -> bool:
	var records: Array[Dictionary] = [{"kind": "joypad_button", "button_index": button_index}]
	return replace_action_bindings(action_name, records)


func reset_bindings(path: String = BINDINGS_PATH) -> bool:
	_restore_project_defaults()
	if not FileAccess.file_exists(path):
		return true
	return DirAccess.remove_absolute(ProjectSettings.globalize_path(path)) == OK


func save_bindings(path: String = BINDINGS_PATH) -> bool:
	var actions: Dictionary = {}
	for action_name: String in InputActions.action_names():
		var records: Array[Dictionary] = []
		for event: InputEvent in InputMap.action_get_events(action_name):
			var record: Dictionary = _record_from_event(event)
			if record.is_empty():
				return false
			records.append(record)
		if not _records_are_valid(records):
			return false
		actions[action_name] = records
	return _write_json(path, {"schema_version": 2, "actions": actions})


func load_bindings(path: String = BINDINGS_PATH) -> bool:
	if not FileAccess.file_exists(path):
		_restore_project_defaults()
		return true
	var decoded: Dictionary = _read_json(path)
	if not decoded.get("ok", false) or not _binding_document_is_valid(decoded["value"]):
		_restore_project_defaults()
		return false
	var actions: Dictionary = decoded["value"]["actions"]
	for action_name: String in InputActions.action_names():
		var records: Array[Dictionary] = []
		for record: Variant in actions[action_name]:
			records.append((record as Dictionary).duplicate(true))
		_apply_records(action_name, records)
	return true


func set_text_scale(percent: int) -> bool:
	if percent not in ALLOWED_TEXT_SCALES:
		return false
	text_scale_percent = percent
	return true


func save_accessibility(path: String = ACCESSIBILITY_PATH) -> bool:
	return _write_json(path, accessibility_document())


func load_accessibility(path: String = ACCESSIBILITY_PATH) -> bool:
	if not FileAccess.file_exists(path):
		_reset_accessibility()
		return true
	var decoded: Dictionary = _read_json(path)
	if not decoded.get("ok", false) or not _accessibility_document_is_valid(decoded["value"]):
		_reset_accessibility()
		return false
	var document: Dictionary = decoded["value"]
	text_scale_percent = document["ui_text_scale_percent"]
	sfx_muted = document["sfx_muted"]
	sfx_volume_percent = document["sfx_volume_percent"]
	return true


func binding_document() -> Dictionary:
	var actions: Dictionary = {}
	for action_name: String in InputActions.action_names():
		var records: Array[Dictionary] = []
		for event: InputEvent in InputMap.action_get_events(action_name):
			records.append(_record_from_event(event))
		actions[action_name] = records
	return {"schema_version": 2, "actions": actions}


func accessibility_document() -> Dictionary:
	return {
		"schema_version": 2,
		"ui_text_scale_percent": text_scale_percent,
		"sfx_muted": sfx_muted,
		"sfx_volume_percent": sfx_volume_percent,
	}


func set_sfx_muted(value: bool) -> void:
	sfx_muted = value


func set_sfx_volume(percent: int) -> bool:
	if percent < 0 or percent > 100:
		return false
	sfx_volume_percent = percent
	return true


func _capture_project_defaults() -> void:
	_defaults.clear()
	for action_name: String in InputActions.action_names():
		var events: Array[InputEvent] = []
		for event: InputEvent in InputMap.action_get_events(action_name):
			events.append(event.duplicate() as InputEvent)
		_defaults[action_name] = events


func _restore_project_defaults() -> void:
	for action_name: String in InputActions.action_names():
		InputMap.action_erase_events(action_name)
		for event: InputEvent in _defaults.get(action_name, []):
			InputMap.action_add_event(action_name, event.duplicate() as InputEvent)


func _apply_records(action_name: String, records: Array[Dictionary]) -> void:
	InputMap.action_erase_events(action_name)
	for record: Dictionary in records:
		InputMap.action_add_event(action_name, _event_from_record(record))


func _event_from_record(record: Dictionary) -> InputEvent:
	match record["kind"]:
		"key":
			var key_event: InputEventKey = InputEventKey.new()
			key_event.physical_keycode = record["physical_keycode"]
			key_event.shift_pressed = record["shift"]
			key_event.ctrl_pressed = record["ctrl"]
			key_event.alt_pressed = record["alt"]
			key_event.meta_pressed = record["meta"]
			return key_event
		"mouse_button":
			var mouse_event: InputEventMouseButton = InputEventMouseButton.new()
			mouse_event.button_index = record["button_index"]
			return mouse_event
		_:
			var joypad_event: InputEventJoypadButton = InputEventJoypadButton.new()
			joypad_event.button_index = record["button_index"]
			return joypad_event


func _record_from_event(event: InputEvent) -> Dictionary:
	if event is InputEventKey:
		var key_event: InputEventKey = event as InputEventKey
		if key_event.physical_keycode <= 0:
			return {}
		return {
			"kind": "key",
			"physical_keycode": key_event.physical_keycode,
			"shift": key_event.shift_pressed,
			"ctrl": key_event.ctrl_pressed,
			"alt": key_event.alt_pressed,
			"meta": key_event.meta_pressed,
		}
	if event is InputEventMouseButton:
		return {"kind": "mouse_button", "button_index": (event as InputEventMouseButton).button_index}
	if event is InputEventJoypadButton:
		return {"kind": "joypad_button", "button_index": (event as InputEventJoypadButton).button_index}
	return {}


func _binding_document_is_valid(value: Variant) -> bool:
	if not _has_exact_keys(value, ["schema_version", "actions"]):
		return false
	var document: Dictionary = value as Dictionary
	if typeof(document["schema_version"]) != TYPE_INT or document["schema_version"] != 2:
		return false
	if typeof(document["actions"]) != TYPE_DICTIONARY:
		return false
	var actions: Dictionary = document["actions"]
	if actions.size() != InputActions.action_names().size():
		return false
	for key: Variant in actions.keys():
		if typeof(key) != TYPE_STRING or not InputActions.is_tme_action(str(key)):
			return false
	for action_name: String in InputActions.action_names():
		if not actions.has(action_name) or typeof(actions[action_name]) != TYPE_ARRAY:
			return false
		if not _records_variant_is_valid(actions[action_name]):
			return false
	return true


func _accessibility_document_is_valid(value: Variant) -> bool:
	if not _has_exact_keys(value, ["schema_version", "ui_text_scale_percent", "sfx_muted", "sfx_volume_percent"]):
		return false
	var document: Dictionary = value as Dictionary
	return typeof(document["schema_version"]) == TYPE_INT \
		and document["schema_version"] == 2 \
		and typeof(document["ui_text_scale_percent"]) == TYPE_INT \
		and document["ui_text_scale_percent"] in ALLOWED_TEXT_SCALES \
		and typeof(document["sfx_muted"]) == TYPE_BOOL \
		and typeof(document["sfx_volume_percent"]) == TYPE_INT \
		and document["sfx_volume_percent"] >= 0 \
		and document["sfx_volume_percent"] <= 100


func _reset_accessibility() -> void:
	text_scale_percent = 100
	sfx_muted = false
	sfx_volume_percent = 100


func _records_are_valid(records: Array[Dictionary]) -> bool:
	return _records_variant_is_valid(records)


func _records_variant_is_valid(records_value: Variant) -> bool:
	if typeof(records_value) != TYPE_ARRAY or (records_value as Array).is_empty():
		return false
	var seen: Dictionary = {}
	for record_value: Variant in records_value:
		if not _record_is_valid(record_value):
			return false
		var signature: String = JSON.stringify(record_value)
		if seen.has(signature):
			return false
		seen[signature] = true
	return true


func _record_is_valid(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var record: Dictionary = value as Dictionary
	if not record.has("kind") or typeof(record["kind"]) != TYPE_STRING:
		return false
	match record["kind"]:
		"key":
			if not _has_exact_keys(record, ["kind", "physical_keycode", "shift", "ctrl", "alt", "meta"]):
				return false
			return typeof(record["physical_keycode"]) == TYPE_INT \
				and record["physical_keycode"] > 0 \
				and typeof(record["shift"]) == TYPE_BOOL \
				and typeof(record["ctrl"]) == TYPE_BOOL \
				and typeof(record["alt"]) == TYPE_BOOL \
				and typeof(record["meta"]) == TYPE_BOOL
		"mouse_button":
			return _has_exact_keys(record, ["kind", "button_index"]) \
				and typeof(record["button_index"]) == TYPE_INT \
				and record["button_index"] > 0
		"joypad_button":
			return _has_exact_keys(record, ["kind", "button_index"]) \
				and typeof(record["button_index"]) == TYPE_INT \
				and record["button_index"] >= 0
	return false


func _has_exact_keys(value: Variant, expected: Array[String]) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var dictionary: Dictionary = value as Dictionary
	if dictionary.size() != expected.size():
		return false
	for key: Variant in dictionary.keys():
		if typeof(key) != TYPE_STRING or str(key) not in expected:
			return false
	return true


func _read_json(path: String) -> Dictionary:
	var file: FileAccess = FileAccess.open(path, FileAccess.READ)
	if file == null:
		return {"ok": false}
	var bytes: PackedByteArray = file.get_buffer(file.get_length())
	return StrictJson.decode_bytes(bytes, MAX_SETTINGS_BYTES, MAX_SETTINGS_DEPTH)


func _write_json(path: String, value: Dictionary) -> bool:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(value))
	return file.get_error() == OK
