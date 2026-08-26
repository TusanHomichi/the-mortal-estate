class_name ClientRoot
extends Control

const DEFAULT_ENDPOINT: EndpointConfig = preload("res://config/default_endpoint.tres")
const ScreenshotCaptureScript: Script = preload("res://presentation/screenshot_capture.gd")
const WELCOME_TIMEOUT_MSEC: int = 15000

@onready var login_screen: LoginScreen = %LoginScreen
@onready var character_screen: CharacterSelectScreen = %CharacterSelectScreen
@onready var world_screen: WorldShellScreen = %WorldShellScreen

var control_state: ControlState = ControlState.new()
var authoritative_state: AuthoritativeState = AuthoritativeState.new()
var presentation_state: PresentationState = PresentationState.new()
var path_preview_state: PathPreviewState = PathPreviewState.new()
var state_machine: ConnectionStateMachine
var facade: ControlFacade
var binding_store: BindingStore = BindingStore.new()
var interaction_preferences: InteractionPreferences = InteractionPreferences.new()

var _socket_pump_active: bool = false
var _welcome_deadline_msec: int = 0
var _pending_result_revision: String = ""
var _pending_interaction_kind: String = ""


func _ready() -> void:
	set_process_unhandled_input(true)
	state_machine = ConnectionStateMachine.new(control_state, authoritative_state)
	var endpoint: EndpointConfig = EndpointConfig.resolve(DEFAULT_ENDPOINT)
	facade = ControlFacade.new(NativeTransport.new(endpoint), endpoint, control_state)
	login_screen.configure_endpoint(endpoint)
	login_screen.login_requested.connect(_on_login_requested)
	character_screen.character_selected.connect(_on_character_selected)
	character_screen.logout_requested.connect(_on_logout_requested)
	world_screen.reconnect_requested.connect(_on_reconnect_requested)
	world_screen.logout_requested.connect(_on_logout_requested)
	world_screen.intent_requested.connect(_on_intent_requested)
	world_screen.path_preview_requested.connect(_on_path_preview_requested)
	world_screen.social_message_requested.connect(_on_social_message_requested)
	world_screen.configure_presentation_state(presentation_state)
	interaction_preferences.load()
	world_screen.configure_interaction_preferences(interaction_preferences)
	binding_store.load_bindings()
	binding_store.load_accessibility()
	apply_text_scale(binding_store.text_scale_percent)
	world_screen.configure_audio_mix(binding_store.sfx_muted, binding_store.sfx_volume_percent)
	show_login_screen()


func show_login_screen() -> void:
	login_screen.apply_credential_prefill(DevCredentials.resolve())
	_show_only(login_screen)
	login_screen.first_focus_control().call_deferred("grab_focus")


func apply_text_scale(percent: int) -> bool:
	if not binding_store.set_text_scale(percent):
		return false
	presentation_state.text_scale_percent = percent
	login_screen.apply_text_scale(percent)
	character_screen.apply_text_scale(percent)
	world_screen.apply_text_scale(percent)
	return true


func _on_login_requested(username: String, login_value: String) -> void:
	if not state_machine.transition(ConnectionStateMachine.AUTHENTICATING):
		return
	login_screen.set_busy(true)
	var result: Dictionary = facade.login(username, login_value)
	login_screen.set_busy(false)
	if not result.get("ok", false):
		state_machine.login_failed()
		login_screen.show_error(str(result.get("error", "Sign-in failed")))
		return
	state_machine.transition(ConnectionStateMachine.BOOTSTRAPPED)
	character_screen.render_bootstrap(control_state.bootstrap)
	_show_only(character_screen)
	character_screen.first_focus_control().call_deferred("grab_focus")


func _on_character_selected(character_id: String) -> void:
	if not state_machine.transition(ConnectionStateMachine.SELECTING_CHARACTER):
		return
	var result: Dictionary = facade.select_character(character_id)
	if not result.get("ok", false):
		state_machine.retryable_failure()
		return
	state_machine.transition(ConnectionStateMachine.BOOTSTRAPPED)
	var bootstrap_result: Dictionary = facade.bootstrap_session()
	if not bootstrap_result.get("ok", false):
		return
	_on_enter_world_requested()


func _show_world_shell() -> void:
	_show_only(world_screen)
	_sync_world_state()
	world_screen.first_focus_control().call_deferred("grab_focus")


func _on_enter_world_requested() -> void:
	_show_world_shell()
	match state_machine.current:
		ConnectionStateMachine.BOOTSTRAPPED:
			if not state_machine.transition(ConnectionStateMachine.ISSUING_TICKET): return
			_sync_world_state()
			_begin_socket_attempt(true)
		ConnectionStateMachine.RETRYABLE_ERROR, ConnectionStateMachine.RECONCILING:
			_on_reconnect_requested()


func _begin_socket_attempt(initial: bool) -> void:
	var ticket: Dictionary = facade.issue_socket_ticket()
	if not ticket.get("ok", false):
		_handle_retryable_failure()
		return
	if initial:
		if not state_machine.transition(ConnectionStateMachine.SOCKET_CONNECTING):
			_handle_fail_closed()
			return
		_sync_world_state()
	var connected: Dictionary = facade.connect_socket()
	if not connected.get("ok", false):
		_handle_retryable_failure()
		return
	if initial:
		if not state_machine.transition(ConnectionStateMachine.AWAITING_WELCOME):
			_handle_fail_closed()
			return
	_socket_pump_active = true
	_welcome_deadline_msec = Time.get_ticks_msec() + WELCOME_TIMEOUT_MSEC
	_sync_world_state()


func _on_reconnect_requested() -> void:
	if state_machine.current == ConnectionStateMachine.RETRYABLE_ERROR:
		if not state_machine.transition(ConnectionStateMachine.RECONCILING): return
	elif state_machine.current == ConnectionStateMachine.ONLINE:
		if not state_machine.transition(ConnectionStateMachine.RECONCILING): return
	elif state_machine.current != ConnectionStateMachine.RECONCILING:
		return
	_teardown_socket()
	_present_empty_authority_if_needed()
	var bootstrap_result: Dictionary = facade.bootstrap_session()
	if not bootstrap_result.get("ok", false):
		_handle_retryable_failure()
		return
	_begin_socket_attempt(false)


func _on_logout_requested() -> void:
	if not state_machine.transition(ConnectionStateMachine.LOGGING_OUT):
		return
	_socket_pump_active = false
	_welcome_deadline_msec = 0
	path_preview_state.clear()
	_pending_result_revision = ""
	_pending_interaction_kind = ""
	_sync_world_state()
	var result: Dictionary = facade.logout()
	facade.close_socket()
	if result.get("ok", false):
		state_machine.transition(ConnectionStateMachine.SIGNED_OUT)
		presentation_state.discard()
		world_screen.refresh_discarded_presentation()
		show_login_screen()
	else:
		state_machine.transition(ConnectionStateMachine.RETRYABLE_ERROR)
		_sync_world_state()


func _on_intent_requested(intent: Dictionary) -> void:
	path_preview_state.clear()
	world_screen.clear_pointer_draft(false)
	_pending_interaction_kind = str(intent.get("kind", ""))
	if _pending_interaction_kind != "nock":
		world_screen.interaction_director.note_unrelated_command()
	var result: Dictionary = facade.submit_command(intent, authoritative_state)
	if result.get("pending_installed", false):
		world_screen.set_command_pending(true)
		world_screen.note_command_installed(intent)
	if not result.get("ok", false):
		_handle_socket_loss()


func _on_path_preview_requested(path: Array[String]) -> void:
	var result: Dictionary = facade.submit_path_preview(path, path_preview_state, authoritative_state)
	if not result.get("ok", false):
		_handle_socket_loss()


func _on_social_message_requested(scope: Dictionary, body: String) -> void:
	var result: Dictionary = facade.send_social_message(scope, body, authoritative_state)
	if not result.get("ok", false):
		_handle_socket_loss()


func _process(_delta: float) -> void:
	if not _socket_pump_active or facade == null: return
	var polled: Dictionary = facade.poll_server_envelopes()
	for envelope_value: Variant in polled.get("envelopes", []):
		_consume_server_envelope(envelope_value as Dictionary)
		if not _socket_pump_active: return
	if not polled.get("ok", false):
		_handle_fail_closed()
		return
	if not polled.get("socket_open", false):
		_handle_socket_loss()
		return
	if _welcome_deadline_msec > 0 and Time.get_ticks_msec() >= _welcome_deadline_msec:
		_handle_retryable_failure()


func _consume_server_envelope(envelope: Dictionary) -> void:
	var kind: String = str(envelope.get("kind", ""))
	match kind:
		"server_welcome":
			_accept_live_welcome(envelope)
		"state_update":
			var reduction: String = authoritative_state.apply_state_update(envelope)
			if reduction == "rejected":
				_handle_fail_closed()
				return
			_present_envelope_and_events(envelope)
			if reduction == "replaced":
				var preview_request_current: bool = path_preview_state.request_matches_authority(
					control_state.active_control_epoch,
					authoritative_state.world_revision(),
					authoritative_state.actor_id(),
				)
				if not preview_request_current:
					path_preview_state.clear()
				world_screen.present_frame(
					authoritative_state.presentation_frame(),
					authoritative_state.frame_generation,
					true,
					preview_request_current,
				)
			_maybe_finish_pending_result()
		"command_result":
			_present_envelope_and_events(envelope)
			_settle_matching_command(envelope)
		"path_preview_result":
			world_screen.present_server_envelope(envelope, authoritative_state.connection_id())
			if path_preview_state.accept_result(
				envelope, control_state.active_control_epoch,
				authoritative_state.world_revision(), authoritative_state.actor_id(),
			) == "accepted":
				world_screen.apply_path_preview_result(envelope)
		"social_message", "message_result":
			_present_envelope_and_events(envelope)
		"server_draining", "error":
			_present_envelope_and_events(envelope)
			_handle_retryable_server_failure()
		_:
			_handle_fail_closed()


func _accept_live_welcome(envelope: Dictionary) -> void:
	if _welcome_deadline_msec == 0:
		_handle_fail_closed()
		return
	var reconnecting: bool = state_machine.current == ConnectionStateMachine.RECONCILING
	if not state_machine.accept_welcome(envelope):
		_handle_fail_closed()
		return
	_welcome_deadline_msec = 0
	path_preview_state.clear()
	world_screen.present_server_envelope(envelope, str(envelope.get("connection_id", "")))
	world_screen.present_frame(
		authoritative_state.presentation_frame(),
		authoritative_state.frame_generation,
		false,
		false,
	)
	if reconnecting:
		if control_state.has_pending_command():
			world_screen.set_command_pending(true)
			if not facade.replay_pending_command().get("ok", false):
				_handle_socket_loss()
				return
		else:
			state_machine.complete_reconciliation(true)
	_sync_world_state()


func _present_envelope_and_events(envelope: Dictionary) -> void:
	var connection_id: String = authoritative_state.connection_id()
	world_screen.present_server_envelope(envelope, connection_id)
	var events: Array = envelope.get("events", [])
	if events.is_empty(): return
	var server_sequence: String = str(envelope.get("server_sequence", ""))
	var identity: String = ""
	if not server_sequence.is_empty():
		identity = "event:%s:%s" % [connection_id, server_sequence]
	elif envelope.get("kind") == "command_result":
		identity = "command:" + str(envelope.get("command_id", ""))
	world_screen.present_observed_events(events, identity)


func _settle_matching_command(envelope: Dictionary) -> void:
	var pending: PendingCommand = control_state.pending_command()
	if pending == null or pending.command_id() != envelope.get("command_id"): return
	var settlement: Dictionary = control_state.settle_pending(pending.command_id(), envelope["disposition"])
	if not settlement.get("settled", false): return
	world_screen.interaction_director.note_command_result(
		_pending_interaction_kind,
		envelope.get("disposition", {}).get("kind") == "accepted",
	)
	world_screen.note_command_result(
		_pending_interaction_kind,
		envelope.get("disposition", {}).get("kind") == "accepted",
		"command:" + str(envelope.get("command_id", "")),
	)
	_pending_interaction_kind = ""
	var after_revision: Variant = envelope.get("after_revision")
	_pending_result_revision = "" if after_revision == null else str(after_revision)
	_maybe_finish_pending_result()


func _maybe_finish_pending_result() -> void:
	var revision_covered: bool = _pending_result_revision.is_empty() or authoritative_state.world_revision_at_least(_pending_result_revision)
	if revision_covered: _pending_result_revision = ""
	var command_waiting: bool = control_state.has_pending_command() or not _pending_result_revision.is_empty()
	world_screen.set_command_pending(command_waiting)
	if not command_waiting:
		world_screen.interaction_director.reconsider_current_frame()
	if state_machine.current == ConnectionStateMachine.RECONCILING and not command_waiting:
		state_machine.complete_reconciliation(true)
	_sync_world_state()


func _handle_socket_loss() -> void:
	var was_online: bool = state_machine.current == ConnectionStateMachine.ONLINE
	_teardown_socket()
	if was_online:
		state_machine.socket_disconnected()
	else:
		state_machine.retryable_failure()
	_present_empty_authority_if_needed()
	_sync_world_state()


func _handle_retryable_server_failure() -> void:
	var was_online: bool = state_machine.current == ConnectionStateMachine.ONLINE
	_teardown_socket()
	if was_online: state_machine.socket_disconnected()
	state_machine.retryable_failure()
	_present_empty_authority_if_needed()
	_sync_world_state()


func _handle_retryable_failure() -> void:
	_teardown_socket()
	state_machine.retryable_failure()
	_present_empty_authority_if_needed()
	_sync_world_state()


func _handle_fail_closed() -> void:
	_teardown_socket()
	state_machine.fail_closed()
	_present_empty_authority_if_needed()
	_sync_world_state()


func _teardown_socket() -> void:
	_socket_pump_active = false
	_welcome_deadline_msec = 0
	_pending_result_revision = ""
	_pending_interaction_kind = ""
	path_preview_state.clear()
	if facade != null: facade.close_socket()
	if world_screen != null and is_instance_valid(world_screen):
		world_screen.set_connection_state(state_machine.current, false)


func _present_empty_authority_if_needed() -> void:
	if not authoritative_state.has_authority():
		world_screen.present_frame({}, authoritative_state.frame_generation)


func _sync_world_state() -> void:
	if world_screen == null or not is_instance_valid(world_screen): return
	var online: bool = state_machine.current == ConnectionStateMachine.ONLINE and authoritative_state.has_authority()
	world_screen.set_connection_state(state_machine.current, online)
	world_screen.set_command_pending(control_state.has_pending_command() or not _pending_result_revision.is_empty())


func _exit_tree() -> void:
	_socket_pump_active = false
	_welcome_deadline_msec = 0
	if facade != null: facade.close_socket()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("tme_screenshot"):
		ScreenshotCaptureScript.capture(get_viewport())
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_text_scale_increase"):
		_change_text_scale(1)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_text_scale_decrease"):
		_change_text_scale(-1)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_text_scale_reset"):
		apply_text_scale(100)
		binding_store.save_accessibility()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_focus_next"):
		_move_focus(true)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_focus_previous"):
		_move_focus(false)
		get_viewport().set_input_as_handled()


func _change_text_scale(direction: int) -> void:
	var scales: Array[int] = BindingStore.ALLOWED_TEXT_SCALES
	var index: int = scales.find(binding_store.text_scale_percent)
	var next_index: int = clampi(index + direction, 0, scales.size() - 1)
	if apply_text_scale(scales[next_index]):
		binding_store.save_accessibility()


func _move_focus(forward: bool) -> void:
	var current_focus: Control = get_viewport().gui_get_focus_owner()
	if current_focus == null:
		return
	var next_focus: Control = current_focus.find_next_valid_focus() if forward else current_focus.find_prev_valid_focus()
	if next_focus != null:
		next_focus.grab_focus()


func _show_only(active_screen: Control) -> void:
	for screen: Control in [login_screen, character_screen, world_screen]:
		screen.visible = screen == active_screen
