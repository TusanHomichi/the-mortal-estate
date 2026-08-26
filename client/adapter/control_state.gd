class_name ControlState
extends RefCounted

var bootstrap: Dictionary = {}
var selected_character_id: String = ""
var lifecycle: String = "SIGNED_OUT"
var active_control_epoch: String = ""
var next_sequence_by_epoch: Dictionary = {}

var _active_login_value: String = ""
var _session_cookie: String = ""
var _csrf_token: String = ""
var _admission_ticket: String = ""
var _pending: PendingCommand = null
var _terminal_commands: Dictionary = {}


func begin_login(value: String) -> void:
	_active_login_value = value


func active_login_value() -> String: return _active_login_value


func finish_login() -> void:
	_active_login_value = ""


func set_session_cookie(value: String) -> void: _session_cookie = value
func has_session_cookie() -> bool: return not _session_cookie.is_empty()
func session_cookie_for_control() -> String: return _session_cookie
func set_csrf_token(value: String) -> void: _csrf_token = value
func csrf_token_for_control() -> String: return _csrf_token
func set_admission_ticket(value: String) -> void: _admission_ticket = value
func has_admission_ticket() -> bool: return not _admission_ticket.is_empty()


func consume_admission_ticket() -> String:
	var value: String = _admission_ticket
	_admission_ticket = ""
	return value


func accept_bootstrap(value: Dictionary) -> void:
	bootstrap = value.duplicate(true)
	_csrf_token = str(value.get("csrf_token", ""))
	var selected: Variant = value.get("selected_character_id")
	selected_character_id = "" if selected == null else str(selected)


func accept_welcome(control_epoch: String) -> void:
	active_control_epoch = control_epoch
	next_sequence_by_epoch[control_epoch] = "1"


func active_next_sequence() -> String:
	return str(next_sequence_by_epoch.get(active_control_epoch, ""))


func install_pending(command: PendingCommand) -> bool:
	if _pending != null: return false
	_pending = command
	return true


func pending_command() -> PendingCommand: return _pending
func has_pending_command() -> bool: return _pending != null


func settle_pending(command_id: String, disposition: Dictionary) -> Dictionary:
	if _terminal_commands.has(command_id): return {"settled": false, "duplicate": true, "consumed": false}
	if _pending == null or _pending.command_id() != command_id: return {"settled": false, "duplicate": false, "consumed": false}
	var consumes: bool = _disposition_consumes(disposition)
	if consumes and _pending.apply_sequence_consumption_once():
		var epoch: String = _pending.control_epoch()
		var current: String = str(next_sequence_by_epoch.get(epoch, _pending.client_sequence()))
		next_sequence_by_epoch[epoch] = CanonicalDecimal.increment(current)
	_terminal_commands[command_id] = true
	_pending = null
	return {"settled": true, "duplicate": false, "consumed": consumes}


func clear_session() -> void:
	bootstrap.clear()
	selected_character_id = ""
	lifecycle = "SIGNED_OUT"
	active_control_epoch = ""
	next_sequence_by_epoch.clear()
	_active_login_value = ""
	_session_cookie = ""
	_csrf_token = ""
	_admission_ticket = ""
	_pending = null
	_terminal_commands.clear()


func debug_summary() -> Dictionary:
	return {"lifecycle": lifecycle, "selected_character_id": selected_character_id, "active_control_epoch": active_control_epoch, "has_bootstrap": not bootstrap.is_empty(), "has_session": has_session_cookie(), "has_pending": _pending != null}


func _disposition_consumes(disposition: Dictionary) -> bool:
	if disposition.get("kind") == "accepted": return true
	return disposition.get("kind") == "rejected" and disposition.get("code") == "rules_rejected"


