class_name PathPreviewState
extends RefCounted

const DIRECTIONS: Array[String] = [
	"north",
	"northeast",
	"east",
	"southeast",
	"south",
	"southwest",
	"west",
	"northwest",
]

var _request: Dictionary = {}
var _result: Dictionary = {}


func begin(
		path: Array[String],
		control_epoch: String,
		observed_world_revision: String,
		actor_id: String,
		preview_id: String = "",
) -> Dictionary:
	if not _valid_path(path) or control_epoch.is_empty() or observed_world_revision.is_empty() or actor_id.is_empty():
		clear()
		return {}
	var correlation: String = preview_id if not preview_id.is_empty() else _random_uuid_v4()
	if correlation.is_empty():
		clear()
		return {}
	_request = {
		"kind": "path_preview",
		"preview_id": correlation,
		"control_epoch": control_epoch,
		"observed_world_revision": observed_world_revision,
		"actor_id": actor_id,
		"path": path.duplicate(),
	}
	_result.clear()
	return wire_request()


func accept_result(
		envelope: Dictionary,
		current_control_epoch: String,
		current_world_revision: String,
		current_actor_id: String,
) -> String:
	if _request.is_empty() or envelope.get("kind") != "path_preview_result":
		return "discarded"
	if envelope.get("preview_id") != _request["preview_id"] \
			or envelope.get("control_epoch") != current_control_epoch \
			or envelope.get("world_revision") != current_world_revision \
			or envelope.get("actor_id") != current_actor_id \
			or _request["control_epoch"] != current_control_epoch \
			or _request["observed_world_revision"] != current_world_revision \
			or _request["actor_id"] != current_actor_id:
		return "discarded"
	var disposition: Dictionary = envelope.get("disposition", {})
	if disposition.get("kind") == "previewed":
		var preview: Variant = envelope.get("preview")
		if preview is not Dictionary or preview.get("requested_path") != _request["path"]:
			return "discarded"
	elif disposition.get("kind") == "rejected":
		if envelope.get("preview") != null:
			return "discarded"
	else:
		return "discarded"
	_result = envelope.duplicate(true)
	return "accepted"


func wire_request() -> Dictionary:
	if _request.is_empty():
		return {}
	return _request.duplicate(true)


func request_matches_authority(
		current_control_epoch: String,
		current_world_revision: String,
		current_actor_id: String,
) -> bool:
	return (
		not _request.is_empty()
		and _request.get("control_epoch") == current_control_epoch
		and _request.get("observed_world_revision") == current_world_revision
		and _request.get("actor_id") == current_actor_id
	)


func requested_path() -> Array[String]:
	if _request.is_empty():
		return []
	var path: Array[String] = []
	path.assign(_request["path"])
	return path


func current_result() -> Dictionary:
	return _result.duplicate(true)


func has_request() -> bool:
	return not _request.is_empty()


func has_preview() -> bool:
	return not _result.is_empty() and _result.get("disposition", {}).get("kind") == "previewed"


func clear() -> void:
	_request.clear()
	_result.clear()


func debug_summary() -> Dictionary:
	return {
		"has_request": has_request(),
		"has_preview": has_preview(),
		"preview_id": str(_request.get("preview_id", "")),
		"requested_steps": requested_path().size(),
	}


func _valid_path(path: Array[String]) -> bool:
	if path.is_empty() or path.size() > InputActions.MAX_MOVE_PATH_STEPS:
		return false
	for direction: String in path:
		if direction not in DIRECTIONS:
			return false
	return true


func _random_uuid_v4() -> String:
	var bytes: PackedByteArray = Crypto.new().generate_random_bytes(16)
	if bytes.size() != 16:
		return ""
	bytes[6] = (bytes[6] & 0x0f) | 0x40
	bytes[8] = (bytes[8] & 0x3f) | 0x80
	var hex: String = bytes.hex_encode()
	return hex.substr(0, 8) + "-" + hex.substr(8, 4) + "-" + hex.substr(12, 4) + "-" + hex.substr(16, 4) + "-" + hex.substr(20, 12)
