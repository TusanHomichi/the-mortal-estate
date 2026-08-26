class_name InteractionPreferences
extends RefCounted

const PREFERENCES_PATH: String = "user://tme_interaction_preferences_v1.json"
const SCHEMA_VERSION: int = 1
const DEFAULT_RANGED_MODE: String = "jumpkick"
const RANGED_MODES: Array[String] = ["jumpkick", "throw", "shoot"]
const MAX_SETTINGS_BYTES: int = 65536
const MAX_SETTINGS_DEPTH: int = 8

var _ranged_by_character: Dictionary = {}


func ranged_mode(character_id: String) -> String:
	if character_id.is_empty():
		return DEFAULT_RANGED_MODE
	return str(_ranged_by_character.get(character_id, DEFAULT_RANGED_MODE))


func set_ranged_mode(character_id: String, mode: String) -> bool:
	if character_id.is_empty() or mode not in RANGED_MODES:
		return false
	_ranged_by_character[character_id] = mode
	return true


func document() -> Dictionary:
	var characters: Dictionary = {}
	var ids: Array[String] = []
	for key: Variant in _ranged_by_character.keys():
		ids.append(str(key))
	ids.sort()
	for character_id: String in ids:
		characters[character_id] = {"ranged_mode": str(_ranged_by_character[character_id])}
	return {"schema_version": SCHEMA_VERSION, "characters": characters}


func save(path: String = PREFERENCES_PATH) -> bool:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(document()))
	return file.get_error() == OK


func load(path: String = PREFERENCES_PATH) -> bool:
	if not FileAccess.file_exists(path):
		_ranged_by_character.clear()
		return true
	var file: FileAccess = FileAccess.open(path, FileAccess.READ)
	if file == null:
		_ranged_by_character.clear()
		return false
	var decoded: Dictionary = StrictJson.decode_bytes(
		file.get_buffer(file.get_length()),
		MAX_SETTINGS_BYTES,
		MAX_SETTINGS_DEPTH,
	)
	if not decoded.get("ok", false) or not _document_is_valid(decoded.get("value")):
		_ranged_by_character.clear()
		return false
	_ranged_by_character.clear()
	for character_id: Variant in (decoded["value"]["characters"] as Dictionary).keys():
		_ranged_by_character[str(character_id)] = str(decoded["value"]["characters"][character_id]["ranged_mode"])
	return true


func reset(path: String = PREFERENCES_PATH) -> bool:
	_ranged_by_character.clear()
	if not FileAccess.file_exists(path):
		return true
	return DirAccess.remove_absolute(ProjectSettings.globalize_path(path)) == OK


func _document_is_valid(value: Variant) -> bool:
	if value is not Dictionary:
		return false
	var document_value: Dictionary = value as Dictionary
	if not _has_exact_keys(document_value, ["schema_version", "characters"]):
		return false
	if typeof(document_value["schema_version"]) != TYPE_INT or document_value["schema_version"] != SCHEMA_VERSION:
		return false
	if typeof(document_value["characters"]) != TYPE_DICTIONARY:
		return false
	for character_id: Variant in (document_value["characters"] as Dictionary).keys():
		if typeof(character_id) != TYPE_STRING or str(character_id).is_empty():
			return false
		var row: Variant = document_value["characters"][character_id]
		if row is not Dictionary or not _has_exact_keys(row, ["ranged_mode"]):
			return false
		if typeof(row["ranged_mode"]) != TYPE_STRING or str(row["ranged_mode"]) not in RANGED_MODES:
			return false
	return true


func _has_exact_keys(value: Dictionary, expected: Array[String]) -> bool:
	if value.size() != expected.size():
		return false
	for key: Variant in value.keys():
		if typeof(key) != TYPE_STRING or str(key) not in expected:
			return false
	return true
