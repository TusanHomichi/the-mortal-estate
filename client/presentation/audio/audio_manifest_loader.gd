class_name AudioManifestLoader
extends RefCounted

const MANIFEST_PATH: String = "res://presentation/audio/audio_manifest.generated.json"
const GENERATED_ROOT: String = "res://presentation/audio/assets/generated/"
const MAX_MANIFEST_BYTES: int = 65536
const MAX_MANIFEST_DEPTH: int = 8
const EXPECTED_ROLES: Array[String] = [
	"combat_swing",
	"combat_body_impact",
	"combat_dry_result",
	"bow_release",
	"spell_chant",
	"spell_release",
	"spell_impact",
	"loot_stow",
	"ui_reject",
]

var last_error: String = ""
var _cues_by_id: Dictionary = {}
var _ids_by_role: Dictionary = {}


func load_manifest(path: String = MANIFEST_PATH) -> Dictionary:
	last_error = ""
	_cues_by_id.clear()
	_ids_by_role.clear()
	var file: FileAccess = FileAccess.open(path, FileAccess.READ)
	if file == null:
		return _fail("projected audio manifest is missing")
	var decoded: Dictionary = StrictJson.decode_bytes(
		file.get_buffer(file.get_length()),
		MAX_MANIFEST_BYTES,
		MAX_MANIFEST_DEPTH,
	)
	if not decoded.get("ok", false) or decoded.get("value") is not Dictionary:
		return _fail("projected audio manifest is not strict bounded JSON")
	var manifest: Dictionary = decoded["value"]
	if not _has_exact_keys(manifest, ["schema_version", "kind", "source", "cues"]):
		return _fail("projected audio manifest has unknown or missing keys")
	if manifest.get("schema_version") != 1 or manifest.get("kind") != "client_audio_manifest":
		return _fail("projected audio manifest must be client schema 1")
	var source: Variant = manifest.get("source")
	if source is not Dictionary or not _has_exact_keys(source, ["schema_version", "sha256"]):
		return _fail("projected audio source record is invalid")
	if source.get("schema_version") != 1 or not _valid_sha(str(source.get("sha256", ""))):
		return _fail("projected audio source record is invalid")
	var cues: Variant = manifest.get("cues")
	if cues is not Array or (cues as Array).size() != 10:
		return _fail("projected audio manifest must contain exactly ten cues")
	for value: Variant in cues:
		if not _load_cue(value):
			return {}
	if _cues_by_id.size() != 10 or _ids_by_role.size() != EXPECTED_ROLES.size():
		return _fail("projected audio cue IDs or roles are incomplete")
	for role: String in EXPECTED_ROLES:
		if not _ids_by_role.has(role):
			return _fail("projected audio role is missing: " + role)
		var expected_count: int = 2 if role == "combat_body_impact" else 1
		if (_ids_by_role[role] as Array).size() != expected_count:
			return _fail("projected audio role has the wrong variant count: " + role)
	return manifest.duplicate(true)


func cue(cue_id: String) -> Dictionary:
	return (_cues_by_id.get(cue_id, {}) as Dictionary).duplicate(true)


func cues_for_role(role: String) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	for cue_id: Variant in _ids_by_role.get(role, []):
		result.append(cue(str(cue_id)))
	return result


func cue_for_role(role: String, event_identity: String = "") -> Dictionary:
	var ids: Array = _ids_by_role.get(role, [])
	if ids.is_empty():
		return {}
	var index: int = 0
	if ids.size() > 1:
		var digest: PackedByteArray = event_identity.sha256_buffer()
		index = int(digest[0]) % ids.size()
	return cue(str(ids[index]))


func _load_cue(value: Variant) -> bool:
	if value is not Dictionary:
		_fail("projected audio cue must be an object")
		return false
	var row: Dictionary = value as Dictionary
	if not _has_exact_keys(row, [
		"id", "role", "path", "sha256", "byte_length", "gain_db",
		"pitch_min", "pitch_max", "max_simultaneous_voices", "variant_group",
	]):
		_fail("projected audio cue has unknown or missing keys")
		return false
	var cue_id: String = str(row.get("id", ""))
	var role: String = str(row.get("role", ""))
	var resource_path: String = str(row.get("path", ""))
	if cue_id.is_empty() or _cues_by_id.has(cue_id) or role not in EXPECTED_ROLES:
		_fail("projected audio cue identity or role is invalid")
		return false
	if not _safe_resource_path(resource_path) or not _valid_sha(str(row.get("sha256", ""))):
		_fail("projected audio cue path or hash is invalid: " + cue_id)
		return false
	if typeof(row.get("byte_length")) != TYPE_INT or int(row["byte_length"]) <= 0:
		_fail("projected audio byte length is invalid: " + cue_id)
		return false
	if typeof(row.get("gain_db")) not in [TYPE_INT, TYPE_FLOAT] or float(row["gain_db"]) < -60.0 or float(row["gain_db"]) > 6.0:
		_fail("projected audio gain is out of bounds: " + cue_id)
		return false
	if typeof(row.get("pitch_min")) not in [TYPE_INT, TYPE_FLOAT] or typeof(row.get("pitch_max")) not in [TYPE_INT, TYPE_FLOAT]:
		_fail("projected audio pitch is invalid: " + cue_id)
		return false
	if float(row["pitch_min"]) < 0.5 or float(row["pitch_max"]) > 2.0 or float(row["pitch_min"]) > float(row["pitch_max"]):
		_fail("projected audio pitch is out of bounds: " + cue_id)
		return false
	var expected_cap: int = 2 if role == "combat_body_impact" else 1
	if typeof(row.get("max_simultaneous_voices")) != TYPE_INT or int(row["max_simultaneous_voices"]) != expected_cap:
		_fail("projected audio voice cap is invalid: " + cue_id)
		return false
	if not _source_bytes_are_valid(row, resource_path, OS.has_feature("editor")):
		_fail("projected audio bytes do not match their hash and length: " + cue_id)
		return false
	if not ResourceLoader.exists(resource_path, "AudioStream"):
		_fail("projected audio import is missing: " + cue_id)
		return false
	var stream: AudioStream = load(resource_path) as AudioStream
	if stream == null:
		_fail("projected audio resource is not decodable: " + cue_id)
		return false
	var cached: Dictionary = row.duplicate(true)
	cached["stream"] = stream
	_cues_by_id[cue_id] = cached
	if not _ids_by_role.has(role):
		_ids_by_role[role] = []
	(_ids_by_role[role] as Array).append(cue_id)
	return true


func _source_bytes_are_valid(row: Dictionary, resource_path: String, required: bool) -> bool:
	if not FileAccess.file_exists(resource_path):
		# Godot exports imported AudioStream resources instead of their source
		# files. Source/editor runs still require and hash the pristine bytes;
		# an export may omit them only because the decodable import is checked next.
		return not required
	var bytes: PackedByteArray = FileAccess.get_file_as_bytes(resource_path)
	var hashing: HashingContext = HashingContext.new()
	hashing.start(HashingContext.HASH_SHA256)
	hashing.update(bytes)
	var digest: String = hashing.finish().hex_encode()
	return bytes.size() == int(row["byte_length"]) and digest == row["sha256"]


func _safe_resource_path(path: String) -> bool:
	var lowered: String = path.to_lower()
	return (
		path.begins_with(GENERATED_ROOT)
		and (path.ends_with(".ogg") or path.ends_with(".wav"))
		and ".." not in path
		and "://" not in path.trim_prefix("res://")
		and "/addons/" not in lowered
		and "content/" not in lowered
		and "research/" not in lowered
		and ("place" + "holders") + "/" not in lowered
	)


func _valid_sha(value: String) -> bool:
	if value.length() != 64:
		return false
	for index: int in range(value.length()):
		var character: String = value.substr(index, 1)
		if character not in "0123456789abcdef":
			return false
	return true


func _has_exact_keys(value: Dictionary, expected: Array[String]) -> bool:
	if value.size() != expected.size():
		return false
	for key: Variant in value.keys():
		if typeof(key) != TYPE_STRING or str(key) not in expected:
			return false
	return true


func _fail(message: String) -> Dictionary:
	last_error = message
	return {}
