class_name PendingCommand
extends RefCounted

var _encoded_bytes: PackedByteArray
var _command_id: String
var _control_epoch: String
var _client_sequence: String
var _observed_world_revision: String
var _actor_id: String
var _intent_facts: Dictionary
var _sequence_consumption_applied: bool = false


static func create(encoded_bytes: PackedByteArray, command_id: String, control_epoch: String, client_sequence: String, observed_world_revision: String, actor_id: String, intent_facts: Dictionary) -> PendingCommand:
	var command: PendingCommand = PendingCommand.new()
	command._encoded_bytes = encoded_bytes.duplicate()
	command._command_id = command_id
	command._control_epoch = control_epoch
	command._client_sequence = client_sequence
	command._observed_world_revision = observed_world_revision
	command._actor_id = actor_id
	command._intent_facts = intent_facts.duplicate(true)
	return command


func encoded_bytes() -> PackedByteArray: return _encoded_bytes.duplicate()
func command_id() -> String: return _command_id
func control_epoch() -> String: return _control_epoch
func client_sequence() -> String: return _client_sequence
func observed_world_revision() -> String: return _observed_world_revision
func actor_id() -> String: return _actor_id
func intent_facts() -> Dictionary: return _intent_facts.duplicate(true)
func sequence_consumption_applied() -> bool: return _sequence_consumption_applied


func apply_sequence_consumption_once() -> bool:
	if _sequence_consumption_applied: return false
	_sequence_consumption_applied = true
	return true


func debug_summary() -> Dictionary:
	return {"command_id": _command_id, "control_epoch": _control_epoch, "client_sequence": _client_sequence, "actor_id": _actor_id, "sequence_consumption_applied": _sequence_consumption_applied}
