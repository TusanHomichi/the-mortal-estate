class_name WorldShellScreen
extends Control

const INSPECT_PLACEHOLDER: String = "Right-click a visible square to inspect from your current position."
const INSPECT_RESET_STATES: Array[String] = ["RECONCILING", "LOGGING_OUT"]
const PATH_PREVIEW_TIMEOUT_MSEC: int = 5000
const POINTER_DRAFT_IDLE_MSEC: int = 10000
const TME_THEME: Theme = preload(
	"res://presentation/TmeTheme.tres"
)

signal reconnect_requested
signal logout_requested
signal intent_requested(intent: Dictionary)
signal path_preview_requested(path: Array[String])
signal social_message_requested(scope: Dictionary, body: String)

@onready var hud: FullHud = %FullHud
@onready var connection_status: Label = hud.connection_status
@onready var readiness_status: Label = hud.readiness_status
@onready var pulse_meter: PulseMeter = hud.pulse_meter
@onready var action_indicator: Label = hud.action_indicator
@onready var world_view: WorldViewSeam = %WorldView
@onready var stairs_up_button: Button = hud.stairs_up_button
@onready var stairs_down_button: Button = hud.stairs_down_button
@onready var inspect_panel: RichTextLabel = hud.inspect_panel
@onready var chat_log: RichTextLabel = hud.chat_log
@onready var feedback_log: RichTextLabel = hud.feedback_log
@onready var reconnect_button: Button = hud.reconnect_button
@onready var domains_button: Button = hud.domains_button
@onready var logout_button: Button = hud.logout_button
@onready var confirmation: TmeConfirmationDialog = %UnsafeConfirmation
@onready var domain_panel: DomainPanel = %DomainPanel
@onready var ground_tray: GroundTray = %GroundTray
@onready var context_palette: ContextPalette = %ContextPalette
@onready var help_overlay: HelpOverlay = %HelpOverlay

var _frame: Dictionary = {}
var _frame_generation: int = 0
var _command_pending: bool = false
var _ordinary_input_enabled: bool = false
var _movement_batch: Array[String] = []
var _movement_batch_started_msec: int = 0
var _draft_path: Array[String] = []
var _draft_target: Vector2i = Vector2i.ZERO
var _has_draft: bool = false
var _path_preview: Dictionary = {}
var _draft_wait_started_msec: int = -1
var _draft_last_input_msec: int = -1
var _draft_commit_armed: bool = false
var _preview_is_current: bool = false
var _presentation_state: PresentationState = PresentationState.new()
var _connection_state: String = ""
var interaction_director: InteractionDirector = InteractionDirector.new()
var interaction_preferences: InteractionPreferences = InteractionPreferences.new()
var _active_character_id: String = ""
var pulse_clock: PulseClock = PulseClock.new()
var audio_manifest_loader: AudioManifestLoader = AudioManifestLoader.new()
var audio_cue_player: AudioCuePlayer
var combat_feel_director: CombatFeelDirector


func _ready() -> void:
	set_process(true)
	set_process_unhandled_input(true)
	visibility_changed.connect(_on_visibility_changed)
	reconnect_button.pressed.connect(func() -> void: reconnect_requested.emit())
	domains_button.pressed.connect(open_domains)
	logout_button.pressed.connect(func() -> void: logout_requested.emit())
	stairs_up_button.pressed.connect(submit_stairs.bind("stairs_up"))
	stairs_down_button.pressed.connect(submit_stairs.bind("stairs_down"))
	world_view.semantic_primary_pressed.connect(_on_semantic_primary_pressed)
	world_view.semantic_primary_released.connect(_on_semantic_primary_released)
	world_view.semantic_pointer_moved.connect(func(_target: Dictionary, position: Vector2) -> void: interaction_director.update_pointer(position))
	world_view.semantic_secondary_pressed.connect(_on_semantic_secondary_pressed)
	confirmation.accepted.connect(_on_unsafe_confirmed)
	domain_panel.close_requested.connect(close_domains)
	domain_panel.intent_requested.connect(_on_domain_intent_requested)
	domain_panel.drop_destination_requested.connect(_on_drop_destination_requested)
	hud.spell_primary_activated.connect(func(spell_id: String) -> void: interaction_director.activate_spell(spell_id))
	hud.ranged_mode_selected.connect(_on_ranged_mode_selected)
	hud.context_requested.connect(open_context_palette)
	hud.help_requested.connect(func() -> void: help_overlay.open_help(hud.help_button))
	hud.social_message_requested.connect(func(scope: Dictionary, body: String) -> void: social_message_requested.emit(scope.duplicate(true), body))
	hud.actor_selected.connect(_on_hud_actor_selected)
	hud.top_clearance_changed.connect(_on_top_clearance_changed)
	hud.configure_presenter(_presentation_state.feedback_presenter)
	audio_cue_player = AudioCuePlayer.new()
	audio_cue_player.name = "FJAudioCuePlayer"
	add_child(audio_cue_player)
	if not audio_cue_player.configure(audio_manifest_loader):
		push_error("FJ audio projection failed: " + audio_manifest_loader.last_error)
	combat_feel_director = CombatFeelDirector.new(
		preload("res://presentation/combat_feel_profile.tres"),
		audio_cue_player,
	)
	_presentation_state.feedback_presenter.configure_feedback_gate(combat_feel_director.gate_feedback_entry)
	combat_feel_director.deferred_feedback_ready.connect(_on_deferred_feedback_ready)
	combat_feel_director.spell_chant_complete.connect(interaction_director.mark_spell_chant_complete)
	combat_feel_director.visual_flash_requested.connect(_on_visual_flash_requested)
	context_palette.option_requested.connect(func(index: int) -> void: submit_action_option(index))
	ground_tray.exact_intent_requested.connect(_on_direct_surface_intent)
	interaction_director.selection_changed.connect(_on_semantic_selection_changed)
	interaction_director.tile_activated.connect(_on_tile_activated)
	interaction_director.ground_target_pinned.connect(_on_ground_target_pinned)
	interaction_director.drag_started.connect(_on_ground_drag_started)
	interaction_director.drag_finished.connect(_on_ground_drag_finished)
	interaction_director.intent_requested.connect(_on_director_intent)
	interaction_director.unsafe_intent_requested.connect(_on_director_unsafe_intent)
	interaction_director.rejected.connect(_on_director_rejected)
	interaction_director.spell_selection_changed.connect(_on_spell_selection_changed)
	interaction_director.spell_prepare_started.connect(_on_spell_prepare_started)
	interaction_director.spell_target_ready.connect(_on_spell_target_ready)
	interaction_preferences.load()
	_rebuild_focus_order()


func configure_presentation_state(state: PresentationState) -> void:
	_presentation_state = state if state != null else PresentationState.new()
	if is_node_ready():
		hud.configure_presenter(_presentation_state.feedback_presenter)
		if combat_feel_director != null:
			_presentation_state.feedback_presenter.configure_feedback_gate(combat_feel_director.gate_feedback_entry)


func configure_interaction_preferences(preferences: InteractionPreferences) -> void:
	interaction_preferences = preferences if preferences != null else InteractionPreferences.new()


func configure_audio_mix(muted: bool, volume_percent: int) -> bool:
	return audio_cue_player != null and audio_cue_player.set_mix(muted, volume_percent)


func note_command_installed(intent: Dictionary) -> void:
	pulse_clock.note_prepared_intent(intent)
	if combat_feel_director != null:
		combat_feel_director.note_command_installed(intent)


func note_command_result(intent_kind: String, accepted: bool, identity: String = "") -> void:
	if combat_feel_director != null:
		combat_feel_director.note_command_result(intent_kind, accepted, identity)


func refresh_discarded_presentation() -> void:
	if is_node_ready():
		set_connection_state("SIGNED_OUT", false)
		set_command_pending(false)
		_frame_generation += 1
		_frame.clear()
		pulse_clock.clear()
		confirmation.invalidate_if_generation_changed(_frame_generation)
		world_view.present_frame({}, _frame_generation)
		interaction_director.cancel_local_state()
		if combat_feel_director != null:
			combat_feel_director.discard()
		interaction_director.present_frame({}, _frame_generation)
		ground_tray.discard()
		context_palette.visible = false
		help_overlay.visible = false
		domain_panel.present_frame({})
		hud.reset_presentation_surface()
		_reset_inspect_placeholder()
		_present_pulse()


func set_connection_state(state_text: String, online: bool) -> void:
	if state_text != _connection_state and state_text in INSPECT_RESET_STATES:
		_reset_inspect_placeholder()
	_connection_state = state_text
	var icon: String = "●" if online else "○"
	connection_status.text = "%s Connection: %s" % [icon, state_text]
	_ordinary_input_enabled = online
	interaction_director.set_input_enabled(online)
	domain_panel.set_connection_state(online)
	reconnect_button.disabled = online
	if not online:
		cancel_movement_batch()
		clear_pointer_draft()
	_update_special_controls()


func set_command_pending(pending: bool) -> void:
	_command_pending = pending
	interaction_director.set_command_pending(pending)
	domain_panel.set_command_pending(pending)
	if pending:
		clear_pointer_draft(false)
	else:
		pulse_clock.clear_prepared_intent()
	hud.set_command_pending(pending)
	_refresh_context_palette()
	_update_special_controls()


func present_frame(
		frame: Dictionary,
		frame_generation: int,
		preserve_pointer_intent: bool = true,
		preview_request_current: bool = true,
) -> void:
	var reconcile_pointer_intent: bool = preserve_pointer_intent and _has_draft
	if not reconcile_pointer_intent:
		clear_pointer_draft()
	_frame = frame.duplicate(true)
	_frame_generation = frame_generation
	confirmation.invalidate_if_generation_changed(frame_generation)
	pulse_clock.note_frame(_frame)
	world_view.present_frame(_frame, frame_generation)
	_active_character_id = str(_frame.get("social", {}).get("character_id", ""))
	var saved_mode: String = interaction_preferences.ranged_mode(_active_character_id)
	interaction_director.set_ranged_mode(saved_mode)
	interaction_director.present_frame(_frame, frame_generation)
	if combat_feel_director != null:
		combat_feel_director.note_authoritative_frame(_frame)
	ground_tray.present_frame(_frame, frame_generation)
	domain_panel.present_frame(_frame)
	hud.present_frame(_frame)
	hud.set_spell_state(interaction_director.selected_spell_id(), interaction_director.preparing_spell_id())
	hud.set_ranged_state(saved_mode, interaction_director.engagement_states(interaction_director.selected_actor_id()) if not interaction_director.selected_actor_id().is_empty() else {})
	_present_pulse()
	_refresh_context_palette()
	_update_special_controls()
	if reconcile_pointer_intent:
		_reconcile_pointer_draft(preview_request_current)


func present_server_envelope(envelope: Dictionary, connection_id: String = "") -> Dictionary:
	return hud.present_envelope(envelope, connection_id)


func activate_square(coordinate: Vector2i, now_msec: int = -1) -> bool:
	var target: Dictionary = world_view.semantic_target_for_coordinate(coordinate)
	if target.is_empty() or target.get("kind") != "tile":
		_reject_pointer_draft_target()
		return false
	return interaction_director.activate_semantic_target(target, now_msec)


func begin_pointer_draft(coordinate: Vector2i, now_msec: int = -1) -> bool:
	var path: Array[String] = _proposed_path(_observation_center(), coordinate)
	if path.is_empty():
		_reject_pointer_draft_target()
		return false
	var now: int = Time.get_ticks_msec() if now_msec < 0 else now_msec
	_draft_path = path
	_draft_target = coordinate
	_has_draft = true
	_path_preview.clear()
	_preview_is_current = false
	_draft_wait_started_msec = now
	_draft_last_input_msec = now
	_draft_commit_armed = false
	domain_panel.set_world_target(coordinate, path)
	world_view.show_pending(_observation_center(), path)
	action_indicator.text = "◇ Preview pending · %d requested step%s" % [path.size(), "" if path.size() == 1 else "s"]
	path_preview_requested.emit(path.duplicate())
	return true


func apply_path_preview_result(result: Dictionary, now_msec: int = -1) -> bool:
	if not _has_draft:
		return false
	var disposition: Dictionary = result.get("disposition", {})
	if disposition.get("kind") == "rejected":
		clear_pointer_draft(false)
		action_indicator.text = "✕ Preview rejected: %s" % str(disposition.get("code", "rejected"))
		return true
	var preview: Variant = result.get("preview")
	if disposition.get("kind") != "previewed" or preview is not Dictionary or preview.get("requested_path") != _draft_path:
		return false
	_path_preview = (preview as Dictionary).duplicate(true)
	_preview_is_current = true
	_draft_wait_started_msec = Time.get_ticks_msec() if now_msec < 0 else now_msec
	world_view.show_preview(_path_preview)
	action_indicator.text = _preview_summary(_path_preview)
	if _draft_commit_armed:
		return _commit_pointer_draft()
	return true


func clear_pointer_draft(reset_indicator: bool = true) -> void:
	_draft_path.clear()
	_draft_target = Vector2i.ZERO
	_has_draft = false
	_path_preview.clear()
	_preview_is_current = false
	_draft_wait_started_msec = -1
	_draft_last_input_msec = -1
	_draft_commit_armed = false
	if is_instance_valid(domain_panel):
		domain_panel.clear_target_selection(false)
	if is_instance_valid(world_view):
		world_view.set_reach_grid_transient_active(false)
		world_view.clear_interaction()
	if reset_indicator and is_instance_valid(action_indicator):
		action_indicator.text = "Pointer: Select a visible square to preview movement"


func draft_path() -> Array[String]:
	return _draft_path.duplicate()


func has_current_preview() -> bool:
	return _preview_is_current and not _path_preview.is_empty()


func pointer_commit_armed() -> bool:
	return _draft_commit_armed


## A draft with no authority-current preview is waiting on the server. Its
## response age starts at draft creation or the last current-preview arrival;
## routine authority reissues never restart that fail-closed clock.
func process_pointer_preview_timeout(now_msec: int = -1) -> bool:
	if not _has_draft or has_current_preview() or _draft_wait_started_msec < 0:
		return false
	var current_msec: int = Time.get_ticks_msec() if now_msec < 0 else now_msec
	var elapsed_msec: int = current_msec - _draft_wait_started_msec
	if elapsed_msec < 0 or elapsed_msec < PATH_PREVIEW_TIMEOUT_MSEC:
		return false
	clear_pointer_draft(false)
	action_indicator.text = "✕ Movement preview timed out; no movement was sent. Tap the square twice to retry."
	return true


## The visible draft marker is the player's live movement intent, so it must not
## outlive their attention. An untouched draft expires instead of waiting
## indefinitely for a second activation that is never coming.
func process_pointer_draft_idle_timeout(now_msec: int = -1) -> bool:
	if not _has_draft or _draft_commit_armed or _draft_last_input_msec < 0:
		return false
	var current_msec: int = Time.get_ticks_msec() if now_msec < 0 else now_msec
	var idle_msec: int = current_msec - _draft_last_input_msec
	if idle_msec < 0 or idle_msec < POINTER_DRAFT_IDLE_MSEC:
		return false
	clear_pointer_draft(false)
	action_indicator.text = "◇ Movement draft expired; no movement was sent. Select a square to start again."
	return true


func request_inspect() -> bool:
	var option: Dictionary = _find_intent_option("inspect")
	if option.is_empty():
		action_indicator.text = "✕ Inspect unavailable: no server option"
		return false
	if not _option_enabled(option):
		action_indicator.text = "✕ Inspect unavailable: %s" % str(option.get("blocked_reason", "server disabled"))
		return false
	clear_pointer_draft(false)
	action_indicator.text = "◇ Inspect submitted from the current square"
	intent_requested.emit((option["intent"] as Dictionary).duplicate(true))
	return true


func submit_stairs(traversal: String) -> bool:
	var option: Dictionary = _find_intent_option("traverse", traversal)
	if option.is_empty() or not _option_enabled(option) or not _ordinary_input_enabled or _command_pending:
		return false
	clear_pointer_draft(false)
	action_indicator.text = "◇ %s submitted" % ("Stairs up" if traversal == "stairs_up" else "Stairs down")
	intent_requested.emit((option["intent"] as Dictionary).duplicate(true))
	return true


func present_observed_events(events: Array, identity_prefix: String = "") -> bool:
	var presented: bool = false
	for event_value: Variant in events:
		var event: Dictionary = event_value as Dictionary
		if event.get("kind") == "inspected":
			hud.set_inspect_text(_inspect_text(event))
			presented = true
	if not hud.present_observed_events(events, identity_prefix).is_empty():
		presented = true
	return presented


func submit_action_option(index: int) -> bool:
	if not _ordinary_input_enabled or _command_pending or index < 0 or index >= _frame.get("action_options", []).size():
		return false
	var option: Dictionary = _frame["action_options"][index]
	if not option.get("enabled", false) or option.get("intent") == null:
		return false
	var intent: Dictionary = (option["intent"] as Dictionary).duplicate(true)
	if _requires_unsafe_confirmation(intent):
		_open_unsafe_confirmation(str(option.get("label", "Action")), intent, _button_for_option(index))
		return true
	intent_requested.emit(intent)
	return true


func queue_movement_action(action_name: String, now_msec: int = -1) -> bool:
	if not _ordinary_input_enabled or _command_pending or confirmation.visible:
		return false
	var direction: String = InputActions.direction_for_action(action_name)
	if direction.is_empty() or _movement_batch.size() >= InputActions.MAX_MOVE_PATH_STEPS:
		return false
	if _movement_batch.is_empty():
		_movement_batch_started_msec = Time.get_ticks_msec() if now_msec < 0 else now_msec
	_movement_batch.append(direction)
	if _movement_batch.size() == InputActions.MAX_MOVE_PATH_STEPS:
		flush_movement_batch()
	return true


func process_movement_batch(now_msec: int = -1) -> bool:
	if _movement_batch.is_empty():
		return false
	var current_msec: int = Time.get_ticks_msec() if now_msec < 0 else now_msec
	if current_msec - _movement_batch_started_msec < InputActions.MOVEMENT_BATCH_MSEC:
		return false
	return flush_movement_batch()


func flush_movement_batch() -> bool:
	if _movement_batch.is_empty() or not _ordinary_input_enabled or _command_pending:
		return false
	var path: Array[String] = _movement_batch.duplicate()
	_movement_batch.clear()
	_movement_batch_started_msec = 0
	interaction_director.note_unrelated_command()
	intent_requested.emit({"kind": "move_path", "path": path})
	return true


func cancel_movement_batch() -> void:
	_movement_batch.clear()
	_movement_batch_started_msec = 0


func movement_batch() -> Array[String]:
	return _movement_batch.duplicate()


func first_focus_control() -> Control:
	return hud.first_interaction_focus()


func open_domains() -> void:
	domain_panel.open_panel()


func open_context_palette() -> void:
	if context_palette.visible:
		context_palette.close_palette()
	else:
		context_palette.open_palette(hud.more_button)


func close_domains() -> void:
	domain_panel.visible = false
	domains_button.grab_focus()


func apply_text_scale(percent: int) -> void:
	var scaled_theme: Theme = TME_THEME.duplicate(true) as Theme
	scaled_theme.default_font_size = 16 * percent / 100
	theme = scaled_theme
	hud.apply_text_scale(percent)


func _on_top_clearance_changed(clearance: float) -> void:
	domain_panel.offset_top = clearance


func _on_visibility_changed() -> void:
	if visible:
		_reset_inspect_placeholder()


func _reset_inspect_placeholder() -> void:
	hud.set_inspect_text(INSPECT_PLACEHOLDER)


func _process(_delta: float) -> void:
	process_movement_batch()
	process_pointer_preview_timeout()
	process_pointer_draft_idle_timeout()
	_present_pulse()
	if combat_feel_director != null:
		combat_feel_director.advance()


func _unhandled_input(event: InputEvent) -> void:
	if confirmation.visible:
		if event.is_action_pressed("tme_ui_cancel"):
			confirmation.cancel()
			get_viewport().set_input_as_handled()
		return
	for action_name: String in InputActions.movement_action_names():
		if event.is_action_pressed(action_name):
			if queue_movement_action(action_name):
				get_viewport().set_input_as_handled()
			return
	if event.is_action_pressed("tme_ui_accept"):
		if flush_movement_batch():
			get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_ui_cancel"):
		cancel_movement_batch()
		clear_pointer_draft()
		interaction_director.cancel_local_state()
		if context_palette.visible:
			context_palette.close_palette()
		if help_overlay.visible:
			help_overlay.close_help()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_stairs_up"):
		if submit_stairs("stairs_up"):
			get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_stairs_down"):
		if submit_stairs("stairs_down"):
			get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_context_palette"):
		open_context_palette()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_help"):
		help_overlay.open_help(hud.help_button)
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_grid_toggle"):
		world_view.toggle_grid()
		action_indicator.text = "Reach grid: %s" % world_view.grid_control_state().replace("_", " ")
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_reconnect") and not reconnect_button.disabled:
		reconnect_requested.emit()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("tme_logout"):
		logout_requested.emit()
		get_viewport().set_input_as_handled()


func _refresh_context_palette() -> void:
	context_palette.present_options(
		_frame.get("action_options", []),
		_command_pending,
		bool(_frame.get("action_options_truncated", false)),
	)
	_rebuild_focus_order()


func _update_special_controls() -> void:
	_configure_traversal_button(stairs_up_button, "stairs_up", "Up [PgUp]")
	_configure_traversal_button(stairs_down_button, "stairs_down", "Down [PgDn]")


func _rebuild_focus_order() -> void:
	var controls: Array[Control] = []
	for button: Button in hud.focus_buttons():
		if not button.disabled:
			controls.append(button)
	if not stairs_up_button.disabled:
		controls.append(stairs_up_button)
	if not stairs_down_button.disabled:
		controls.append(stairs_down_button)
	controls.append(reconnect_button)
	controls.append(domains_button)
	controls.append(logout_button)
	controls.append(hud.group_launcher)
	controls.append(hud.focus_launcher)
	controls.append(hud.chat_launcher)
	controls.append(hud.feedback_launcher)
	for index: int in range(controls.size()):
		var current: Control = controls[index]
		var previous: Control = controls[(index - 1 + controls.size()) % controls.size()]
		var next: Control = controls[(index + 1) % controls.size()]
		current.focus_neighbor_top = current.get_path_to(previous)
		current.focus_neighbor_bottom = current.get_path_to(next)


func _requires_unsafe_confirmation(intent: Dictionary) -> bool:
	return intent.get("authorization") == "confirmed_unsafe"


func _open_unsafe_confirmation(label: String, intent: Dictionary, origin: Control) -> void:
	clear_pointer_draft(false)
	confirmation.open_confirmation(
		label,
		_target_name(intent),
		_frame_generation,
		origin,
		{"intent": intent.duplicate(true)},
	)


func _target_name(intent: Dictionary) -> String:
	var target_actor_id: String = str(intent.get("target_actor_id", ""))
	var target_value: Variant = intent.get("target")
	if target_actor_id.is_empty() and target_value is Dictionary and target_value.get("kind") == "actor":
		target_actor_id = str(target_value.get("actor_id", ""))
	for actor_value: Variant in _frame.get("actors", []):
		var actor: Dictionary = actor_value as Dictionary
		if actor.get("actor_id") == target_actor_id:
			return "%s (%s)" % [actor.get("name", "Unknown actor"), target_actor_id]
	return target_actor_id if not target_actor_id.is_empty() else "server-selected target"


func _on_unsafe_confirmed(payload: Dictionary) -> void:
	if payload.get("intent") is Dictionary:
		intent_requested.emit((payload["intent"] as Dictionary).duplicate(true))


func _on_domain_intent_requested(intent: Dictionary, label: String, origin: Control) -> void:
	if not _ordinary_input_enabled or _command_pending:
		return
	if _requires_unsafe_confirmation(intent):
		_open_unsafe_confirmation(label, intent, origin)
		return
	intent_requested.emit(intent.duplicate(true))


func _on_hud_actor_selected(actor_id: String) -> void:
	domain_panel.set_actor_target(actor_id)


func _on_semantic_primary_pressed(target: Dictionary, display_position: Vector2) -> void:
	if confirmation.visible or context_palette.visible or help_overlay.visible:
		return
	if target.get("kind") == "tile":
		world_view.set_reach_grid_transient_active(true)
	if not interaction_director.begin_primary_press(target, display_position):
		world_view.set_reach_grid_transient_active(false)


func _on_semantic_primary_released(_target: Dictionary, display_position: Vector2) -> void:
	world_view.set_reach_grid_transient_active(false)
	if not interaction_director.drag_state().is_empty():
		var destination: String = domain_panel.drop_destination_at(world_view.pointer_surface().to_global(display_position))
		if not destination.is_empty():
			interaction_director.complete_item_drag(destination)
			return
	interaction_director.end_primary_press()


func _on_semantic_secondary_pressed(target: Dictionary, _display_position: Vector2) -> void:
	if confirmation.visible:
		return
	if target.get("kind") in ["tile", "corpse", "ground_item", "gold_pile"]:
		_on_ground_target_pinned(target)
	context_palette.open_palette(hud.more_button)


func _on_semantic_selection_changed(target: Dictionary) -> void:
	var kind: String = str(target.get("kind", ""))
	if kind == "actor":
		var actor_id: String = str(target.get("source_identity", ""))
		var actor: Dictionary = target.get("source", {})
		hud.set_selected_actor(actor_id)
		domain_panel.set_actor_target(actor_id)
		hud.set_ranged_state(interaction_director.ranged_mode(), interaction_director.engagement_states(actor_id))
		action_indicator.text = "◆ Focus · %s · HP %s/%s · %s" % [
			actor.get("name", actor_id),
			actor.get("hp", "?"),
			actor.get("max_hp", "?"),
			str(actor.get("attack_safety", "unknown")).replace("_", " "),
		]
	elif kind == "ground_item":
		var item: Dictionary = target.get("source", {})
		domain_panel.set_item_target(str(target.get("source_identity", "")))
		action_indicator.text = "◆ Ground · %s ×%s" % [item.get("name", "Loose item"), item.get("quantity", 1)]
	elif kind == "corpse":
		var corpse: Dictionary = target.get("source", {})
		action_indicator.text = "◆ Ground · %s corpse · %s" % [
			corpse.get("origin_name", "Unknown"),
			"Searched" if bool(corpse.get("searched", false)) else "Contents not searched",
		]
	elif kind == "gold_pile":
		action_indicator.text = "◆ Ground · %s gold" % target.get("source", {}).get("amount", "?")
	elif kind == "tile":
		var coordinate: Vector2i = target.get("coordinate", Vector2i.ZERO)
		domain_panel.set_world_target(coordinate, _proposed_path(_observation_center(), coordinate))
	elif target.is_empty():
		hud.set_selected_actor("")
		hud.set_ranged_state(interaction_director.ranged_mode(), {})
	_try_submit_prepared_target()


func _on_tile_activated(target: Dictionary, activation_msec: int) -> void:
	var coordinate: Vector2i = target.get("coordinate", Vector2i.ZERO)
	if not _has_draft or coordinate != _draft_target:
		begin_pointer_draft(coordinate, activation_msec)
		return
	_draft_last_input_msec = activation_msec
	if has_current_preview():
		_commit_pointer_draft()
		return
	_draft_commit_armed = true
	action_indicator.text = "◇ Movement confirmed · waiting for authoritative preview"


func _on_ground_target_pinned(target: Dictionary) -> void:
	ground_tray.pin_target(target)
	ground_tray.present_frame(_frame, _frame_generation)


func _on_ground_drag_started(target: Dictionary) -> void:
	var item: Dictionary = target.get("source", {})
	var item_name: String = str(item.get("name", target.get("source_identity", "item")))
	domain_panel.select_mode("Inventory")
	domain_panel.open_panel()
	domain_panel.set_ground_drag_active(true, item_name)
	action_indicator.text = "◇ Drag %s · choose Sack or one exact paper-doll destination" % item_name


func _on_ground_drag_finished() -> void:
	domain_panel.set_ground_drag_active(false)


func _on_drop_destination_requested(destination_position: String) -> void:
	if interaction_director.complete_item_drag(destination_position):
		action_indicator.text = "◇ Item move submitted to " + destination_position.replace("_", " ").capitalize()


func _on_director_intent(intent: Dictionary, label: String) -> void:
	action_indicator.text = "◇ %s submitted" % label
	intent_requested.emit(intent.duplicate(true))


func _on_director_unsafe_intent(intent: Dictionary, label: String) -> void:
	_open_unsafe_confirmation(label, intent, hud.more_button)


func _on_direct_surface_intent(intent: Dictionary, label: String, origin: Control) -> void:
	if not _ordinary_input_enabled or _command_pending:
		return
	if _requires_unsafe_confirmation(intent):
		_open_unsafe_confirmation(label, intent, origin)
	else:
		intent_requested.emit(intent.duplicate(true))


func _on_director_rejected(message: String) -> void:
	action_indicator.text = "✕ " + message


func _on_ranged_mode_selected(mode: String) -> void:
	if not interaction_director.set_ranged_mode(mode):
		return
	if not _active_character_id.is_empty():
		interaction_preferences.set_ranged_mode(_active_character_id, mode)
		interaction_preferences.save()
	var actor_id: String = interaction_director.selected_actor_id()
	hud.set_ranged_state(mode, interaction_director.engagement_states(actor_id) if not actor_id.is_empty() else {})
	action_indicator.text = "Ranged preference: " + mode.replace("_", " ").capitalize()


func _on_spell_selection_changed(spell_id: String) -> void:
	domain_panel.select_spell(spell_id)
	hud.set_spell_state(spell_id, interaction_director.preparing_spell_id())


func _on_spell_prepare_started(spell_id: String) -> void:
	hud.set_spell_state(spell_id, spell_id)
	action_indicator.text = "◇ Prepare · %s · chant" % spell_id
	combat_feel_director.begin_spell_prepare(spell_id)


func _on_spell_target_ready(spell_id: String) -> void:
	domain_panel.select_mode("Magic")
	domain_panel.select_spell(spell_id)
	domain_panel.open_panel()
	action_indicator.text = "◆ %s prepared · choose an exact target" % spell_id


func _try_submit_prepared_target() -> void:
	if not interaction_director.spell_target_is_ready():
		return
	var intent: Dictionary = domain_panel.build_selected_cast_intent()
	if not intent.is_empty():
		interaction_director.submit_prepared_spell_target(intent, "Cast " + interaction_director.preparing_spell_id())


func _on_deferred_feedback_ready(entry: Dictionary) -> void:
	_presentation_state.feedback_presenter.append_deferred_feedback(entry)
	hud.refresh_scrollback()


func _on_visual_flash_requested(kind: String) -> void:
	hud.cue_banner.modulate = Color(1.0, 0.86, 0.62, 1.0)
	hud.cue_banner.set_meta("fj_flash_kind", kind)
	var world_feedback: Dictionary = world_view.present_feedback(kind)
	hud.cue_banner.set_meta("gpg1_world_feedback", world_feedback)


func _commit_pointer_draft() -> bool:
	if _draft_path.is_empty() or not has_current_preview():
		return false
	var path: Array[String] = _draft_path.duplicate()
	clear_pointer_draft(false)
	action_indicator.text = "◆ Movement submitted · %d requested step%s" % [path.size(), "" if path.size() == 1 else "s"]
	interaction_director.note_unrelated_command()
	intent_requested.emit({"kind": "move_path", "path": path})
	return true


func _reconcile_pointer_draft(preview_request_current: bool) -> void:
	var target: Dictionary = world_view.semantic_target_for_coordinate(_draft_target)
	var path: Array[String] = _proposed_path(_observation_center(), _draft_target)
	if target.is_empty() or target.get("kind") != "tile" or path.is_empty():
		clear_pointer_draft(false)
		action_indicator.text = "✕ Movement target is no longer available; no movement was sent"
		return
	var path_changed: bool = path != _draft_path
	_draft_path = path
	domain_panel.set_world_target(_draft_target, path)
	if path_changed:
		_path_preview.clear()
	var request_current: bool = preview_request_current and not path_changed
	_preview_is_current = request_current and not _path_preview.is_empty()
	if _preview_is_current:
		world_view.show_preview(_path_preview)
		if _draft_commit_armed:
			_commit_pointer_draft()
		else:
			action_indicator.text = _preview_summary(_path_preview)
		return
	if not request_current:
		# World revision advances on every pulse, so an in-flight request goes
		# stale on a timer the player never sees. Reissue it against current
		# authority without restarting the total unanswered-response clock.
		path_preview_requested.emit(path.duplicate())
	# Keep the last returned preview on screen while the refresh is in flight.
	# Dropping the marker back to pending every pulse read as the world
	# refusing to move.
	if _path_preview.is_empty():
		world_view.show_pending(_observation_center(), _draft_path)
	else:
		world_view.show_preview(_path_preview)
	action_indicator.text = (
		"◇ Movement confirmed · waiting for authoritative preview"
		if _draft_commit_armed
		else "◇ Preview pending · %d requested step%s" % [path.size(), "" if path.size() == 1 else "s"]
	)


func _reject_pointer_draft_target() -> void:
	clear_pointer_draft(false)
	action_indicator.text = "Pointer: Choose another visible square within three steps"


func _proposed_path(start: Vector2i, target: Vector2i) -> Array[String]:
	if start != _observation_center():
		return []
	return ClientReachability.proposed_path(_frame, target)


func _observation_center() -> Vector2i:
	var value: Dictionary = _frame.get("observation_center", {})
	var coordinate: Dictionary = value.get("position", value)
	return Vector2i(int(coordinate.get("x", 0)), int(coordinate.get("y", 0)))


func _preview_summary(preview: Dictionary) -> String:
	var requested: int = _draft_path.size()
	var accepted: String = str(preview.get("accepted_steps", "0"))
	var facts: Array[String] = ["◆ %s · %s/%d steps" % [str(preview.get("pace", "path")).capitalize(), accepted, requested]]
	var opens_door: bool = false
	var transitions: bool = false
	for step_value: Variant in preview.get("steps", []):
		var step: Dictionary = step_value as Dictionary
		opens_door = opens_door or bool(step.get("opens_door", false))
		transitions = transitions or step.get("outcome", {}).get("kind") == "transitioned"
	if opens_door:
		facts.append("opens door")
	if transitions:
		facts.append("transition")
	if str(preview.get("stop_reason", "full_path_accepted")) != "full_path_accepted":
		facts.append(str(preview.get("stop_reason", "stopped")).replace("_", " "))
	facts.append("activate target again to commit")
	return " · ".join(facts)


func _find_intent_option(kind: String, traversal: String = "") -> Dictionary:
	for option_value: Variant in _frame.get("action_options", []):
		var option: Dictionary = option_value as Dictionary
		var intent: Variant = option.get("intent")
		if intent is not Dictionary or intent.get("kind") != kind:
			continue
		if kind == "traverse" and intent.get("traversal") != traversal:
			continue
		return option
	return {}


func _option_enabled(option: Dictionary) -> bool:
	return _ordinary_input_enabled and not _command_pending and bool(option.get("enabled", false)) and option.get("intent") is Dictionary


func _configure_traversal_button(button: Button, traversal: String, label: String) -> void:
	var option: Dictionary = _find_intent_option("traverse", traversal)
	button.text = label
	button.disabled = option.is_empty() or not _option_enabled(option)
	button.tooltip_text = ""
	button.accessibility_description = ""
	if button.disabled:
		var reason: String = str(option.get("blocked_reason", "no server option"))
		button.text += " [X]"
		button.tooltip_text = "Unavailable: " + reason
		button.accessibility_description = "Unavailable: " + reason


## The beat, presented. Runs every drawn frame as well as on every accepted
## frame, because a pulse that only refreshed when authority arrived would be a
## still picture of one.
##
## The readiness line and the meter are one statement in two forms, and the
## world view and the feedback director are handed the same account, so none of
## the four can describe a different beat from the others. Nothing here reads a
## clock for gameplay: readiness is the frame's `can_act` throughout, and the
## only local quantity is how far the measured beat has got.
func _present_pulse() -> void:
	var state: Dictionary = pulse_clock.state()
	pulse_meter.present_pulse(state)
	var line: String = pulse_meter.meter_text()
	if readiness_status.text != line:
		readiness_status.text = line
	world_view.present_pulse(state)
	if combat_feel_director != null:
		combat_feel_director.set_beat_deadline(int(state.get("beat_deadline_msec", -1)))


func _is_dedicated_traversal(option: Dictionary) -> bool:
	var intent: Variant = option.get("intent")
	return intent is Dictionary and intent.get("kind") == "traverse" and intent.get("traversal") in ["stairs_up", "stairs_down"]


func _button_for_option(option_index: int) -> Button:
	for button: Button in context_palette.buttons():
		if int(button.get_meta("option_index", -1)) == option_index:
			return button
	return reconnect_button


func _inspect_text(event: Dictionary) -> String:
	var lines: Array[String] = ["%s · move cost %s" % [str(event.get("tile", "Unknown tile")), "—" if event.get("tile_move_cost") == null else str(event.get("tile_move_cost"))]]
	for exit_value: Variant in event.get("exits", []):
		var exit: Dictionary = exit_value as Dictionary
		var status: Dictionary = exit.get("status", {})
		var status_text: String = str(status.get("kind", "unknown")).replace("_", " ")
		if status.get("kind") == "door":
			status_text = "door " + ("open" if bool(status.get("open", false)) else "closed")
		lines.append("Exit %s: %s" % [str(exit.get("direction", "?")).capitalize(), status_text])
	for actor_value: Variant in event.get("nearby_actors", []):
		var actor: Dictionary = actor_value as Dictionary
		lines.append("%s: %s · %s · HP %s" % [str(actor.get("direction", "here")).capitalize(), str(actor.get("actor", "Unknown actor")), str(actor.get("kind", "actor")), str(actor.get("hp", "?"))])
	for item_value: Variant in event.get("ground_items", []):
		var item: Dictionary = item_value as Dictionary
		var direction: String = "Here" if item.get("direction") == null else str(item.get("direction")).capitalize()
		lines.append("%s: %s ×%s" % [direction, str(item.get("name", "Unknown item")), str(item.get("quantity", 1))])
	return "\n".join(lines)
