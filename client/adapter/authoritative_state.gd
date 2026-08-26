class_name AuthoritativeState
extends RefCounted

var frame_generation: int = 0

var _frame: Dictionary = {}
var _static_scene_context: Dictionary = {}
var _connection_id: String = ""
var _actor_id: String = ""
var _server_sequence: String = ""
var _world_revision: String = ""


func accept_welcome(envelope: Dictionary) -> bool:
	if envelope.get("kind") != "server_welcome": return false
	_connection_id = envelope["connection_id"]
	_actor_id = str(envelope.get("actor_id", ""))
	_server_sequence = envelope["server_sequence"]
	_world_revision = envelope["world_revision"]
	_static_scene_context = envelope["static_scene_context"].duplicate(true)
	_frame = envelope["frame"].duplicate(true)
	frame_generation += 1
	return true


## There is one world, so an update carries no world identity to check. The
## admitted socket is the binding, and a welcome-installed frame is the only
## thing an update may replace: an update arriving without authority, or out of
## sequence order, is still refused.
func apply_state_update(envelope: Dictionary) -> String:
	if _frame.is_empty() or envelope.get("kind") != "state_update": return "rejected"
	var ordering: int = _compare_decimal(envelope["server_sequence"], _server_sequence)
	if ordering < 0: return "rejected"
	if ordering == 0:
		if envelope["world_revision"] == _world_revision \
			and envelope["static_scene_context"] == _static_scene_context \
			and envelope["frame"] == _frame: return "duplicate"
		return "rejected"
	_server_sequence = envelope["server_sequence"]
	_world_revision = envelope["world_revision"]
	_static_scene_context = envelope["static_scene_context"].duplicate(true)
	_frame = envelope["frame"].duplicate(true)
	frame_generation += 1
	return "replaced"


func discard_for_reconnect() -> void:
	if not _frame.is_empty(): frame_generation += 1
	_frame.clear()
	_static_scene_context.clear()
	_connection_id = ""
	_actor_id = ""
	_server_sequence = ""
	_world_revision = ""


func has_authority() -> bool: return not _frame.is_empty()
func ordinary_input_enabled() -> bool: return has_authority()
func frame() -> Dictionary: return _frame.duplicate(true)
func static_scene_context() -> Dictionary: return _static_scene_context.duplicate(true)
func presentation_frame() -> Dictionary:
	var value: Dictionary = _frame.duplicate(true)
	value["static_scene_context"] = _static_scene_context.duplicate(true)
	return value
func connection_id() -> String: return _connection_id
func actor_id() -> String: return _actor_id
func server_sequence() -> String: return _server_sequence
func world_revision() -> String: return _world_revision


func world_revision_at_least(required: String) -> bool:
	return _canonical_decimal(_world_revision) and _canonical_decimal(required) and _compare_decimal(_world_revision, required) >= 0


func debug_summary() -> Dictionary:
	return {"has_authority": has_authority(), "connection_id": _connection_id, "actor_id": _actor_id, "server_sequence": _server_sequence, "world_revision": _world_revision, "frame_generation": frame_generation}


func _compare_decimal(left: String, right: String) -> int:
	if left.length() != right.length(): return -1 if left.length() < right.length() else 1
	if left == right: return 0
	return -1 if left < right else 1


func _canonical_decimal(value: String) -> bool:
	if value.is_empty() or (value.length() > 1 and value.begins_with("0")) or value.length() > 20:
		return false
	for codepoint: int in value.to_ascii_buffer():
		if codepoint < 0x30 or codepoint > 0x39:
			return false
	return value.length() < 20 or value <= "18446744073709551615"
