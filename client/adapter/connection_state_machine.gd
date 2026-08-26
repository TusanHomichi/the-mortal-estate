class_name ConnectionStateMachine
extends RefCounted

const SIGNED_OUT: String = "SIGNED_OUT"
const AUTHENTICATING: String = "AUTHENTICATING"
const BOOTSTRAPPED: String = "BOOTSTRAPPED"
const SELECTING_CHARACTER: String = "SELECTING_CHARACTER"
const ISSUING_TICKET: String = "ISSUING_TICKET"
const SOCKET_CONNECTING: String = "SOCKET_CONNECTING"
const AWAITING_WELCOME: String = "AWAITING_WELCOME"
const ONLINE: String = "ONLINE"
const RECONCILING: String = "RECONCILING"
const LOGGING_OUT: String = "LOGGING_OUT"
const RETRYABLE_ERROR: String = "RETRYABLE_ERROR"

var current: String = SIGNED_OUT
var control: ControlState
var authority: AuthoritativeState


func _init(control_state: ControlState, authoritative_state: AuthoritativeState) -> void:
	control = control_state
	authority = authoritative_state


func transition(next: String) -> bool:
	if next not in _allowed().get(current, []): return false
	current = next
	control.lifecycle = current
	if next == RECONCILING: authority.discard_for_reconnect()
	if next == SIGNED_OUT:
		authority.discard_for_reconnect()
		control.clear_session()
	return true


func login_failed() -> bool:
	return current == AUTHENTICATING and transition(SIGNED_OUT)


func socket_disconnected() -> bool:
	return current == ONLINE and transition(RECONCILING)


func retryable_failure() -> bool:
	if current not in [RECONCILING, ISSUING_TICKET, SOCKET_CONNECTING, AWAITING_WELCOME]: return false
	return transition(RETRYABLE_ERROR)


## One world means a welcome carries no world identity to match. What still has
## to hold is that this connection was admitted for a character this account
## actually selected, so the guard checks the selection instead.
func accept_welcome(envelope: Dictionary) -> bool:
	if current not in [AWAITING_WELCOME, RECONCILING]: return false
	if not _has_selected_character(): return false
	var frame: Variant = envelope.get("frame")
	if frame is not Dictionary or envelope.get("actor_id") != frame.get("observer_actor_id"): return false
	if not authority.accept_welcome(envelope): return false
	control.accept_welcome(envelope["control_epoch"])
	if current == AWAITING_WELCOME: return transition(ONLINE)
	return true


func complete_reconciliation(retained_command_decided: bool) -> bool:
	return current == RECONCILING and authority.has_authority() and retained_command_decided and transition(ONLINE)


func fail_closed() -> void:
	authority.discard_for_reconnect()
	current = RETRYABLE_ERROR
	control.lifecycle = current


func _has_selected_character() -> bool:
	if control.selected_character_id.is_empty(): return false
	for character_value: Variant in control.bootstrap.get("characters", []):
		var character: Dictionary = character_value as Dictionary
		if character.get("character_id") == control.selected_character_id:
			return true
	return false


func _allowed() -> Dictionary:
	return {
		SIGNED_OUT: [AUTHENTICATING],
		AUTHENTICATING: [BOOTSTRAPPED, SIGNED_OUT],
		BOOTSTRAPPED: [SELECTING_CHARACTER, ISSUING_TICKET, LOGGING_OUT],
		SELECTING_CHARACTER: [BOOTSTRAPPED, RECONCILING, RETRYABLE_ERROR, LOGGING_OUT],
		ISSUING_TICKET: [SOCKET_CONNECTING, RECONCILING, RETRYABLE_ERROR, LOGGING_OUT],
		SOCKET_CONNECTING: [AWAITING_WELCOME, RECONCILING, RETRYABLE_ERROR, LOGGING_OUT],
		AWAITING_WELCOME: [ONLINE, RECONCILING, RETRYABLE_ERROR, LOGGING_OUT],
		ONLINE: [RECONCILING, LOGGING_OUT],
		RECONCILING: [ONLINE, RETRYABLE_ERROR, LOGGING_OUT],
		LOGGING_OUT: [SIGNED_OUT, RETRYABLE_ERROR],
		RETRYABLE_ERROR: [RECONCILING, LOGGING_OUT],
	}
