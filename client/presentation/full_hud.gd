class_name FullHud
extends Control

signal social_message_requested(scope: Dictionary, body: String)
signal actor_selected(actor_id: String)
signal spell_primary_activated(spell_id: String)
signal ranged_mode_selected(mode: String)
signal context_requested
signal help_requested
signal top_clearance_changed(clearance: float)

const LARGE_LAYOUT_THRESHOLD: float = 1500.0
const CUE_DURATION_SECONDS: float = 2.2
const PANE_NAMES: Array[String] = ["group", "focus", "chat", "feedback"]
const DEFAULT_DOMAIN_TOP: float = 64.0
const DEFAULT_SIDE_PANE_TOP: float = 82.0
const DEFAULT_CUE_HEIGHT: float = 44.0
const DEFAULT_TOP_RAIL_HEIGHT: float = 52.0
const DEFAULT_RESOURCE_PLATE_HEIGHT: float = 112.0
const STACKED_RAIL_GAP: float = 8.0
const DEFAULT_SIDE_LAUNCHER_OFFSET_TOP: float = -56.0
const DEFAULT_SIDE_LAUNCHER_OFFSET_BOTTOM: float = -20.0
const DEFAULT_SIDE_LAUNCHER_HEIGHT: float = 36.0
const DEFAULT_BOTTOM_LAUNCHER_OFFSET_TOP: float = -44.0
const DEFAULT_BOTTOM_LAUNCHER_OFFSET_BOTTOM: float = -8.0
const BOTTOM_LAUNCHER_GAP: float = 8.0
const DEFAULT_FEEDBACK_PANE_OFFSET_RIGHT: float = -16.0
const BOTTOM_PANE_RESOURCE_GAP: float = 8.0
const HUD_EDGE_GAP: float = 4.0
## The most of the viewport the top rail may occupy once text is enlarged and it
## stacks. The bound exists so the rail cannot grow without limit and swallow the
## world; it is not a claim that the rail is small.
##
## It was 0.35, calibrated when the rail carried three short single-line rows.
## The beat band raised the measured height at 200 percent text scale to 282 px —
## 39.2 percent of a 720-line viewport — because the readiness line is a real
## sentence and at doubled text it takes three wrapped lines. Trimming it is not
## an option (that is the defect this replaced) and hiding the meter at large text
## sizes would take the visual pulse away from exactly the players who enlarged
## the text. So the bound moved to the measured need plus headroom, and
## `test_full_hud` now actually asserts it — before this it was a public helper
## nothing checked.
const MAX_STACKED_RAIL_VIEWPORT_FRACTION: float = 0.42

@onready var top_rail: PanelContainer = %TopRail
@onready var top_content: GridContainer = %TopContent
@onready var status_row: HBoxContainer = %StatusRow
@onready var action_row: HBoxContainer = %ActionRow
@onready var route_row: HBoxContainer = %RouteRow

@onready var connection_status: Label = %ConnectionStatus
@onready var readiness_status: Label = %ReadinessStatus
@onready var cooldown_meter: CooldownMeter = %CooldownMeter
@onready var action_indicator: Label = %ActionIndicator
@onready var reconnect_button: Button = %ReconnectButton
@onready var domains_button: Button = %DomainsButton
@onready var logout_button: Button = %LogoutButton
@onready var cue_banner: PanelContainer = %CueBanner
@onready var cue_label: Label = %CueLabel
@onready var resource_plate: PanelContainer = %ResourcePlate

@onready var hp_bar: ProgressBar = %HpBar
@onready var hp_label: Label = %HpLabel
@onready var stamina_bar: ProgressBar = %StaminaBar
@onready var stamina_label: Label = %StaminaLabel
@onready var mp_bar: ProgressBar = %MpBar
@onready var mp_label: Label = %MpLabel
@onready var spell_palette: SpellPalette = %HudSpellPalette
@onready var ranged_selector: OptionButton = %RangedSelector
@onready var more_button: Button = %MoreButton
@onready var help_button: Button = %HelpButton

@onready var group_launcher: Button = %GroupLauncher
@onready var focus_launcher: Button = %FocusLauncher
@onready var chat_launcher: Button = %ChatLauncher
@onready var feedback_launcher: Button = %FeedbackLauncher
@onready var group_pane: PanelContainer = %GroupPane
@onready var focus_pane: PanelContainer = %FocusPane
@onready var chat_pane: PanelContainer = %ChatPane
@onready var feedback_pane: PanelContainer = %FeedbackPane
@onready var group_list: VBoxContainer = %GroupList
@onready var focus_list: VBoxContainer = %FocusList
@onready var inspect_panel: RichTextLabel = %InspectPanel
@onready var stairs_up_button: Button = %StairsUpButton
@onready var stairs_down_button: Button = %StairsDownButton

@onready var chat_log: RichTextLabel = %ChatLog
@onready var chat_scope: OptionButton = %ChatScope
@onready var page_recipient: OptionButton = %PageRecipient
@onready var chat_body: LineEdit = %ChatBody
@onready var chat_send: Button = %ChatSend
@onready var chat_size_button: Button = %ChatSizeButton
@onready var feedback_log: RichTextLabel = %FeedbackLog
@onready var feedback_filter: OptionButton = %FeedbackFilter
@onready var feedback_size_button: Button = %FeedbackSizeButton

var presenter: FeedbackPresenter = FeedbackPresenter.new()
var _frame: Dictionary = {}
var _focus_buttons: Array[Button] = []
var _selected_actor_id: String = ""
var _command_pending: bool = false
var _last_compact_layout: Variant = null
var _output_expanded: Dictionary = {"chat": false, "feedback": false}
var _cue_remaining: float = 0.0
var _stacked_top_rail: bool = false
var _last_action_accessibility_text: String = ""
var _top_layout_resolution_queued: bool = false
var text_entry_keyboard: TextEntryKeyboard = TextEntryKeyboard.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_setup_selectors()
	_connect_panes()
	chat_send.pressed.connect(_emit_chat_request)
	chat_body.text_submitted.connect(func(_body: String) -> void: _emit_chat_request())
	text_entry_keyboard.bind(chat_body)
	chat_scope.item_selected.connect(func(_index: int) -> void: _update_chat_controls())
	feedback_filter.item_selected.connect(func(_index: int) -> void: refresh_scrollback())
	resource_plate.minimum_size_changed.connect(_queue_top_layout_resolution)
	feedback_pane.minimum_size_changed.connect(_queue_top_layout_resolution)
	spell_palette.spell_primary_activated.connect(func(spell_id: String) -> void: spell_primary_activated.emit(spell_id))
	ranged_selector.item_selected.connect(_on_ranged_mode_selected)
	more_button.pressed.connect(func() -> void: context_requested.emit())
	help_button.pressed.connect(func() -> void: help_requested.emit())
	resized.connect(_on_hud_resized)
	top_rail.resized.connect(_synchronize_top_clearance)
	top_content.minimum_size_changed.connect(_queue_top_layout_resolution)
	_apply_original_style()
	call_deferred("apply_responsive_layout", size.x)
	_queue_top_layout_resolution()
	_update_chat_controls()
	_update_resource_bars({})
	set_process(true)


func _on_hud_resized() -> void:
	apply_responsive_layout(size.x)
	_queue_top_layout_resolution()


func apply_text_scale(percent: int) -> void:
	_stacked_top_rail = percent > 100
	top_content.columns = 1 if _stacked_top_rail else 3
	# The readiness line is never trimmed, at any scale. It carries the beat —
	# the wait in rounds, the frame's own times, and what is being prepared —
	# and an ellipsis after the second word is the whole statement gone. It has
	# the full width of the rail and wraps when it needs more; the rail is as
	# tall as what it has to show.
	readiness_status.text_overrun_behavior = TextServer.OVERRUN_NO_TRIMMING
	readiness_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	action_indicator.autowrap_mode = (
		TextServer.AUTOWRAP_WORD_SMART
		if _stacked_top_rail
		else TextServer.AUTOWRAP_OFF
	)
	action_indicator.text_overrun_behavior = (
		TextServer.OVERRUN_NO_TRIMMING
		if _stacked_top_rail
		else TextServer.OVERRUN_TRIM_ELLIPSIS
	)
	top_content.queue_sort()
	apply_responsive_layout(size.x)
	_queue_top_layout_resolution()


func top_rail_stacked() -> bool:
	return _stacked_top_rail


func top_rail_bottom() -> float:
	return top_rail.position.y + top_rail.size.y


func stacked_rail_height_is_bounded() -> bool:
	return not _stacked_top_rail or top_rail.size.y <= size.y * MAX_STACKED_RAIL_VIEWPORT_FRACTION


func _queue_top_layout_resolution() -> void:
	if _top_layout_resolution_queued:
		return
	_top_layout_resolution_queued = true
	call_deferred("_resolve_top_rail_layout")


func _resolve_top_rail_layout() -> void:
	_top_layout_resolution_queued = false
	if not is_node_ready():
		return
	# The rail follows its content at every text scale, not only when stacked.
	# It used to be pinned to the default height unless stacked, which was
	# survivable while nothing in it could need a second line — and stopped
	# being survivable when the beat moved in, because a line the rail has no
	# room for is a line the player does not get.
	var desired_height: float = maxf(
		DEFAULT_TOP_RAIL_HEIGHT, top_rail.get_combined_minimum_size().y
	)
	if not is_equal_approx(top_rail.size.y, desired_height):
		top_rail.offset_bottom = desired_height
		top_content.queue_sort()
	var resource_height: float = maxf(
		DEFAULT_RESOURCE_PLATE_HEIGHT,
		resource_plate.get_combined_minimum_size().y,
	)
	if not is_equal_approx(resource_plate.size.y, resource_height):
		resource_plate.offset_top = resource_plate.offset_bottom - resource_height
	_resolve_bottom_pane_clearance()
	_synchronize_top_clearance()


func _resolve_bottom_pane_clearance() -> void:
	if _last_compact_layout == null or bool(_last_compact_layout):
		feedback_pane.offset_right = DEFAULT_FEEDBACK_PANE_OFFSET_RIGHT
		return
	var resource_right: float = resource_plate.position.x + resource_plate.size.x
	var authored_right: float = size.x + DEFAULT_FEEDBACK_PANE_OFFSET_RIGHT
	var required_right: float = resource_right + BOTTOM_PANE_RESOURCE_GAP + feedback_pane.get_combined_minimum_size().x
	var bounded_right: float = minf(maxf(authored_right, required_right), size.x - HUD_EDGE_GAP)
	feedback_pane.offset_right = bounded_right - size.x


func _synchronize_top_clearance() -> void:
	if not is_node_ready():
		return
	if not _stacked_top_rail:
		cue_banner.offset_top = 0.0
		cue_banner.offset_bottom = DEFAULT_CUE_HEIGHT
		group_pane.offset_top = DEFAULT_SIDE_PANE_TOP
		focus_pane.offset_top = DEFAULT_SIDE_PANE_TOP
		_restore_side_launcher_offsets(group_launcher)
		_restore_side_launcher_offsets(focus_launcher)
		_restore_bottom_launcher_offsets(chat_launcher)
		_restore_bottom_launcher_offsets(feedback_launcher)
		top_clearance_changed.emit(DEFAULT_DOMAIN_TOP)
		return
	var clearance: float = ceil(top_rail_bottom() + STACKED_RAIL_GAP)
	var cue_anchor_y: float = size.y * cue_banner.anchor_top
	cue_banner.offset_top = clearance - cue_anchor_y
	cue_banner.offset_bottom = cue_banner.offset_top + DEFAULT_CUE_HEIGHT
	group_pane.offset_top = maxf(DEFAULT_SIDE_PANE_TOP, clearance)
	focus_pane.offset_top = maxf(DEFAULT_SIDE_PANE_TOP, clearance)
	_place_side_launcher_below_rail(group_launcher, clearance)
	_place_side_launcher_below_rail(focus_launcher, clearance)
	_place_bottom_launcher_above_resource(chat_launcher)
	_place_bottom_launcher_above_resource(feedback_launcher)
	top_clearance_changed.emit(clearance)


func _restore_side_launcher_offsets(launcher: Button) -> void:
	launcher.offset_top = DEFAULT_SIDE_LAUNCHER_OFFSET_TOP
	launcher.offset_bottom = DEFAULT_SIDE_LAUNCHER_OFFSET_BOTTOM


func _place_side_launcher_below_rail(launcher: Button, clearance: float) -> void:
	var anchor_y: float = size.y * launcher.anchor_top
	launcher.offset_top = clearance - anchor_y
	launcher.offset_bottom = launcher.offset_top + maxf(DEFAULT_SIDE_LAUNCHER_HEIGHT, launcher.get_combined_minimum_size().y)


func _restore_bottom_launcher_offsets(launcher: Button) -> void:
	launcher.offset_top = DEFAULT_BOTTOM_LAUNCHER_OFFSET_TOP
	launcher.offset_bottom = DEFAULT_BOTTOM_LAUNCHER_OFFSET_BOTTOM


func _place_bottom_launcher_above_resource(launcher: Button) -> void:
	var height: float = maxf(
		DEFAULT_SIDE_LAUNCHER_HEIGHT,
		launcher.get_combined_minimum_size().y,
	)
	var anchor_y: float = size.y * launcher.anchor_bottom
	var desired_bottom: float = resource_plate.position.y - BOTTOM_LAUNCHER_GAP
	launcher.offset_bottom = desired_bottom - anchor_y
	launcher.offset_top = launcher.offset_bottom - height


func configure_presenter(value: FeedbackPresenter) -> void:
	presenter = value if value != null else FeedbackPresenter.new()
	refresh_scrollback()


func reset_presentation_surface() -> void:
	_frame.clear()
	cooldown_meter.clear()
	_selected_actor_id = ""
	_cue_remaining = 0.0
	cue_banner.visible = false
	_update_resource_bars({})
	_rebuild_group()
	_rebuild_focus()
	_rebuild_page_recipients()
	_update_chat_controls()
	spell_palette.present_rows([])
	_update_ranged_selector("jumpkick", {})
	set_inspect_text("No authoritative frame.")
	refresh_scrollback()


func present_frame(frame: Dictionary) -> void:
	_frame = frame.duplicate(true)
	_update_resource_bars(_frame.get("character", {}).get("resources", {}))
	_rebuild_group()
	_rebuild_focus()
	_rebuild_page_recipients()
	_update_chat_controls()
	spell_palette.present_rows(_frame.get("spell_actions", []), spell_palette.selected_spell_id(), "", _command_pending)


func present_envelope(envelope: Dictionary, connection_id: String = "") -> Dictionary:
	var result: Dictionary = presenter.consume_server_envelope(envelope, connection_id)
	refresh_scrollback()
	_pump_cue_queue()
	return result


func present_observed_events(events: Array, identity_prefix: String = "") -> Array[Dictionary]:
	var result: Array[Dictionary] = presenter.consume_observed_events(events, identity_prefix)
	refresh_scrollback()
	_pump_cue_queue()
	return result


func set_command_pending(pending: bool) -> void:
	_command_pending = pending
	spell_palette.set_command_pending(pending)
	ranged_selector.disabled = pending


func set_spell_state(selected_spell_id: String, preparing_spell_id: String = "") -> void:
	spell_palette.present_rows(
		_frame.get("spell_actions", []),
		selected_spell_id,
		preparing_spell_id,
		_command_pending,
	)


func set_ranged_state(selected_mode: String, states: Dictionary = {}) -> void:
	_update_ranged_selector(selected_mode, states)


func set_inspect_text(value: String) -> void:
	inspect_panel.text = value


func set_selected_actor(actor_id: String) -> void:
	var valid_ids: Array[String] = []
	for row: Dictionary in nearest_actor_rows():
		valid_ids.append(str(row.get("actor_id", "")))
	_selected_actor_id = actor_id if actor_id in valid_ids else ""
	_rebuild_focus()


func selected_actor_id() -> String:
	return _selected_actor_id


func nearest_actor_rows() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	var observer_id: String = str(_frame.get("observer_actor_id", ""))
	var center: Dictionary = _frame.get("observation_center", {}).get("position", {})
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if actor.get("actor_id") == observer_id:
			continue
		if not PresentationState.actor_is_present(actor):
			continue
		var position: Dictionary = actor.get("position", {}).get("position", {})
		var row: Dictionary = actor.duplicate(true)
		row["distance"] = maxi(absi(int(position.get("x", 0)) - int(center.get("x", 0))), absi(int(position.get("y", 0)) - int(center.get("y", 0))))
		result.append(row)
	result.sort_custom(func(left: Dictionary, right: Dictionary) -> bool:
		if left["distance"] != right["distance"]:
			return int(left["distance"]) < int(right["distance"])
		return str(left.get("actor_id", "")) < str(right.get("actor_id", ""))
	)
	return result


func submit_chat(body: String, scope_kind: String, target_character_id: String = "") -> bool:
	var clean_body: String = body.strip_edges()
	if clean_body.is_empty() or not scope_kind in ["say", "shout", "group", "page"]:
		return false
	if scope_kind == "group" and _frame.get("social", {}).get("group") == null:
		return false
	var scope: Dictionary = {"kind": scope_kind}
	if scope_kind == "page":
		if target_character_id.is_empty() or not target_character_id in _visible_page_recipient_ids():
			return false
		scope["target_character_id"] = target_character_id
	social_message_requested.emit(scope, clean_body)
	return true


func pane_visible(pane_name: String) -> bool:
	var pane: Control = _pane(pane_name)
	return pane != null and pane.visible


func toggle_pane(pane_name: String, force_visible: Variant = null) -> void:
	var pane: Control = _pane(pane_name)
	var launcher: Button = _launcher(pane_name)
	if pane == null or launcher == null:
		return
	var show: bool = not pane.visible if force_visible == null else bool(force_visible)
	pane.visible = show
	launcher.visible = not show
	launcher.set_pressed_no_signal(show)
	launcher.accessibility_description = ("Collapse " if show else "Open ") + pane_name + " pane"
	if show:
		first_pane_focus(pane_name).grab_focus()
	else:
		launcher.grab_focus()


func cycle_output_size(pane_name: String) -> void:
	if not pane_name in ["chat", "feedback"]:
		return
	_output_expanded[pane_name] = not bool(_output_expanded[pane_name])
	if bool(_output_expanded[pane_name]):
		var side_pane: String = "group" if pane_name == "chat" else "focus"
		if pane_visible(side_pane):
			toggle_pane(side_pane, false)
	var pane: Control = _pane(pane_name)
	pane.offset_top = -390.0 if bool(_output_expanded[pane_name]) else -248.0
	var button: Button = chat_size_button if pane_name == "chat" else feedback_size_button
	button.text = "Compact" if bool(_output_expanded[pane_name]) else "Expand"
	button.accessibility_description = ("Use compact" if bool(_output_expanded[pane_name]) else "Use expanded") + " " + pane_name + " pane height"


func apply_responsive_layout(width: float) -> void:
	var compact: bool = width < LARGE_LAYOUT_THRESHOLD or _stacked_top_rail
	if _last_compact_layout != null and bool(_last_compact_layout) == compact:
		return
	var focus_owner: Control = get_viewport().gui_get_focus_owner()
	var focused_pane_name: String = _pane_name_containing(focus_owner)
	var focused_launcher_name: String = _launcher_name(focus_owner)
	_last_compact_layout = compact
	for pane_name: String in PANE_NAMES:
		var pane: Control = _pane(pane_name)
		var launcher: Button = _launcher(pane_name)
		pane.visible = not compact
		launcher.visible = compact
		launcher.set_pressed_no_signal(not compact)
		launcher.accessibility_description = ("Collapse " if not compact else "Open ") + pane_name + " pane"
	if not compact:
		if bool(_output_expanded["chat"]):
			toggle_pane("group", false)
		if bool(_output_expanded["feedback"]):
			toggle_pane("focus", false)
	if compact and not focused_pane_name.is_empty():
		_launcher(focused_pane_name).grab_focus()
	elif not compact and not focused_launcher_name.is_empty():
		first_pane_focus(focused_launcher_name).grab_focus()


func responsive_layout_is_compact() -> bool:
	return _last_compact_layout != null and bool(_last_compact_layout)


func _pane_name_containing(control: Control) -> String:
	if control == null:
		return ""
	for pane_name: String in PANE_NAMES:
		var pane: Control = _pane(pane_name)
		if control == pane or pane.is_ancestor_of(control):
			return pane_name
	return ""


func _launcher_name(control: Control) -> String:
	if control == null:
		return ""
	for pane_name: String in PANE_NAMES:
		if control == _launcher(pane_name):
			return pane_name
	return ""


func first_pane_focus(pane_name: String) -> Control:
	match pane_name:
		"group": return %GroupClose
		"focus": return %FocusClose
		"chat": return chat_log
		"feedback": return feedback_log
	return group_launcher


func first_interaction_focus() -> Control:
	for button: Button in _focus_buttons:
		if not button.disabled:
			return button
	if not stairs_up_button.disabled:
		return stairs_up_button
	if not stairs_down_button.disabled:
		return stairs_down_button
	return more_button


func group_text() -> String:
	var values: Array[String] = []
	for child: Node in group_list.get_children():
		if child is Label:
			values.append((child as Label).text)
	return "\n".join(values)


func focus_button_texts() -> Array[String]:
	var values: Array[String] = []
	for button: Button in _focus_buttons:
		values.append(button.text)
	return values


func focus_buttons() -> Array[Button]:
	return _focus_buttons


func refresh_scrollback() -> void:
	var chat_lines: Array[String] = []
	for entry: Dictionary in presenter.chat_entries:
		chat_lines.append(str(entry.get("text", "")))
	chat_log.text = "No messages this session." if chat_lines.is_empty() else "\n".join(chat_lines)
	var filter_name: String = ["all", "combat", "quest", "system"][feedback_filter.selected if is_instance_valid(feedback_filter) else 0]
	var feedback_lines: Array[String] = []
	for entry: Dictionary in presenter.feedback_entries:
		if filter_name == "all" or entry.get("category") == filter_name:
			feedback_lines.append(str(entry.get("text", "")))
	feedback_log.text = "No feedback this session." if feedback_lines.is_empty() else "\n".join(feedback_lines)


func advance_cue_time(delta: float) -> void:
	if _cue_remaining <= 0.0:
		_pump_cue_queue()
		return
	_cue_remaining = maxf(0.0, _cue_remaining - delta)
	if _cue_remaining <= 0.0:
		cue_banner.visible = false
		_pump_cue_queue()


# Proof/capture affordance only; live play continues to consume cue_queue FIFO.
func display_queued_cue(kind: String) -> bool:
	var entry: Dictionary = presenter.take_cue_of_kind(kind)
	if entry.is_empty():
		return false
	_display_cue_entry(entry)
	return true


func _process(delta: float) -> void:
	_synchronize_action_accessibility()
	advance_cue_time(delta)


func _synchronize_action_accessibility() -> void:
	if action_indicator.text == _last_action_accessibility_text:
		return
	_last_action_accessibility_text = action_indicator.text
	action_indicator.tooltip_text = action_indicator.text
	action_indicator.accessibility_description = action_indicator.text
	if _stacked_top_rail:
		_queue_top_layout_resolution()


func _pump_cue_queue() -> void:
	if _cue_remaining > 0.0 or presenter.cue_queue.is_empty():
		return
	var entry: Dictionary = presenter.cue_queue.pop_front()
	_display_cue_entry(entry)


func _display_cue_entry(entry: Dictionary) -> void:
	cue_label.text = str(entry.get("text", ""))
	cue_label.accessibility_description = "%s cue: %s" % [entry.get("category", "system"), entry.get("text", "")]
	cue_banner.visible = true
	_cue_remaining = CUE_DURATION_SECONDS


func _update_resource_bars(resources: Dictionary) -> void:
	_update_resource_bar(hp_bar, hp_label, "HP", resources.get("hp"), resources.get("max_hp"))
	_update_resource_bar(stamina_bar, stamina_label, "STAMINA", resources.get("stamina"), resources.get("max_stamina"))
	_update_resource_bar(mp_bar, mp_label, "MP", resources.get("mp"), resources.get("max_mp"))


func _update_resource_bar(bar: ProgressBar, label: Label, title: String, current_value: Variant, maximum_value: Variant) -> void:
	if current_value == null or maximum_value == null:
		bar.min_value = 0.0
		bar.max_value = 1.0
		bar.value = 0.0
		label.text = title + " —/—"
		bar.accessibility_description = title + ": no authoritative character frame"
		return
	var current: int = int(current_value)
	var maximum: int = int(maximum_value)
	bar.min_value = 0.0
	bar.max_value = maxf(1.0, float(maximum))
	bar.value = clampf(float(current), 0.0, bar.max_value)
	label.text = "%s %d/%d" % [title, current, maximum]
	bar.accessibility_description = "%s %d of %d" % [title, current, maximum]


func _rebuild_group() -> void:
	_free_children(group_list)
	var social: Dictionary = _frame.get("social", {})
	var group_value: Variant = social.get("group")
	if not group_value is Dictionary:
		_add_plain_label(group_list, "Not currently grouped.")
		return
	var group: Dictionary = group_value as Dictionary
	var visible_names: Dictionary = {}
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if actor.get("character_id") != null:
			visible_names[str(actor.get("character_id"))] = str(actor.get("name", ""))
	var members: Array = group.get("members", []).duplicate(true)
	members.sort_custom(func(left: Dictionary, right: Dictionary) -> bool: return CanonicalDecimal.less(str(left.get("joined_order", "0")), str(right.get("joined_order", "0"))))
	for value: Variant in members:
		var member: Dictionary = value as Dictionary
		var character_id: String = str(member.get("character_id", ""))
		var name: String = str(visible_names.get(character_id, "Member …" + character_id.right(8)))
		var leader: String = "◆ LEADER · " if character_id == group.get("leader_character_id") else "◇ MEMBER · "
		var presence: String = "● Connected" if bool(member.get("connected", false)) else "○ Disconnected"
		if member.get("absent_since") != null:
			presence += " since T" + str(member.get("absent_since"))
		_add_plain_label(group_list, leader + name + "\n" + presence)


func _rebuild_focus() -> void:
	_free_buttons(_focus_buttons)
	var rows: Array[Dictionary] = nearest_actor_rows()
	var valid_ids: Array[String] = []
	for actor: Dictionary in rows:
		valid_ids.append(str(actor.get("actor_id", "")))
	if not _selected_actor_id in valid_ids:
		_selected_actor_id = ""
	if rows.is_empty():
		var empty: Button = Button.new()
		empty.text = "No other visible actors"
		empty.disabled = true
		focus_list.add_child(empty)
		_focus_buttons.append(empty)
		return
	for actor: Dictionary in rows:
		var actor_id: String = str(actor.get("actor_id", ""))
		var kind: String = str(actor.get("kind", "unknown"))
		var kind_token: String = {"monster": "MON", "npc": "NPC", "player": "PLY"}.get(kind, "ACT")
		var selected: String = "◆ " if actor_id == _selected_actor_id else ""
		var hp_text: String = "HP %s/%s" % [actor.get("hp", "?"), actor.get("max_hp", "?")]
		var button: Button = Button.new()
		button.text = "%s[%s] %s · d%s\n%s · %s · %s" % [selected, kind_token, actor.get("name", actor_id), actor.get("distance", "?"), hp_text, _words(actor.get("life_state", "unknown")), _words(actor.get("attack_safety", "unknown"))]
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.accessibility_description = "%s %s at distance %s; %s; %s; attack safety %s" % [kind, actor.get("name", actor_id), actor.get("distance", "?"), hp_text, _words(actor.get("life_state", "unknown")), _words(actor.get("attack_safety", "unknown"))]
		button.pressed.connect(_select_actor.bind(actor_id))
		focus_list.add_child(button)
		_focus_buttons.append(button)


func _select_actor(actor_id: String) -> void:
	_selected_actor_id = actor_id
	_rebuild_focus()
	actor_selected.emit(actor_id)


func _rebuild_page_recipients() -> void:
	page_recipient.clear()
	for actor: Dictionary in nearest_actor_rows():
		if actor.get("kind") == "player" and actor.get("character_id") != null:
			page_recipient.add_item("%s · %s" % [actor.get("name", "Player"), str(actor.get("character_id")).right(8)])
			page_recipient.set_item_metadata(page_recipient.item_count - 1, str(actor.get("character_id")))
	page_recipient.disabled = page_recipient.item_count == 0


func _visible_page_recipient_ids() -> Array[String]:
	var ids: Array[String] = []
	for index: int in range(page_recipient.item_count):
		ids.append(str(page_recipient.get_item_metadata(index)))
	return ids


func _emit_chat_request() -> void:
	var scope_kind: String = str(chat_scope.get_item_metadata(chat_scope.selected))
	var target: String = ""
	if scope_kind == "page" and page_recipient.selected >= 0:
		target = str(page_recipient.get_item_metadata(page_recipient.selected))
	if submit_chat(chat_body.text, scope_kind, target):
		text_entry_keyboard.hide()
		chat_body.clear()


func _update_chat_controls() -> void:
	if not is_instance_valid(chat_scope):
		return
	var grouped: bool = _frame.get("social", {}).get("group") is Dictionary
	for index: int in range(chat_scope.item_count):
		if chat_scope.get_item_metadata(index) == "group":
			chat_scope.set_item_disabled(index, not grouped)
	var scope_kind: String = str(chat_scope.get_item_metadata(chat_scope.selected))
	page_recipient.visible = scope_kind == "page"
	chat_send.disabled = scope_kind == "group" and not grouped or scope_kind == "page" and page_recipient.item_count == 0
	chat_send.accessibility_description = "Send " + scope_kind + " message" if not chat_send.disabled else "Send unavailable for current " + scope_kind + " scope"


func _setup_selectors() -> void:
	for row: Dictionary in [
		{"label": "Leap / jumpkick", "mode": "jumpkick"},
		{"label": "Throw held weapon", "mode": "throw"},
		{"label": "Shoot bow", "mode": "shoot"},
	]:
		ranged_selector.add_item(str(row["label"]))
		ranged_selector.set_item_metadata(ranged_selector.item_count - 1, row["mode"])
	for scope_name: String in ["say", "shout", "group", "page"]:
		chat_scope.add_item(scope_name.capitalize())
		chat_scope.set_item_metadata(chat_scope.item_count - 1, scope_name)
	for filter_name: String in ["All", "Combat", "Quest", "System"]:
		feedback_filter.add_item(filter_name)


func _update_ranged_selector(selected_mode: String, states: Dictionary) -> void:
	for index: int in range(ranged_selector.item_count):
		var mode: String = str(ranged_selector.get_item_metadata(index))
		var base_label: String = {
			"jumpkick": "Leap / jumpkick",
			"throw": "Throw held weapon",
			"shoot": "Shoot bow",
		}.get(mode, mode)
		var state: Variant = states.get(mode)
		var suffix: String = ""
		if state is Dictionary:
			suffix = " · Ready" if bool(state.get("enabled", false)) else " · " + _words(state.get("blocked_reason", "Unavailable")).capitalize()
		ranged_selector.set_item_text(index, base_label + suffix)
		if mode == selected_mode:
			ranged_selector.select(index)
	ranged_selector.tooltip_text = "Ranged preference: " + {
		"jumpkick": "Leap / jumpkick",
		"throw": "Throw held weapon",
		"shoot": "Shoot bow",
	}.get(selected_mode, "Leap / jumpkick")
	ranged_selector.accessibility_description = ranged_selector.tooltip_text


func _on_ranged_mode_selected(index: int) -> void:
	if index < 0 or index >= ranged_selector.item_count:
		return
	ranged_mode_selected.emit(str(ranged_selector.get_item_metadata(index)))


func _connect_panes() -> void:
	group_launcher.toggled.connect(func(pressed: bool) -> void: toggle_pane("group", pressed))
	focus_launcher.toggled.connect(func(pressed: bool) -> void: toggle_pane("focus", pressed))
	chat_launcher.toggled.connect(func(pressed: bool) -> void: toggle_pane("chat", pressed))
	feedback_launcher.toggled.connect(func(pressed: bool) -> void: toggle_pane("feedback", pressed))
	(%GroupClose as Button).pressed.connect(func() -> void: toggle_pane("group", false))
	(%FocusClose as Button).pressed.connect(func() -> void: toggle_pane("focus", false))
	(%ChatClose as Button).pressed.connect(func() -> void: toggle_pane("chat", false))
	(%FeedbackClose as Button).pressed.connect(func() -> void: toggle_pane("feedback", false))
	chat_size_button.pressed.connect(cycle_output_size.bind("chat"))
	feedback_size_button.pressed.connect(cycle_output_size.bind("feedback"))


func _pane(pane_name: String) -> Control:
	return {"group": group_pane, "focus": focus_pane, "chat": chat_pane, "feedback": feedback_pane}.get(pane_name)


func _launcher(pane_name: String) -> Button:
	return {"group": group_launcher, "focus": focus_launcher, "chat": chat_launcher, "feedback": feedback_launcher}.get(pane_name)


func _free_buttons(buttons: Array[Button]) -> void:
	for button: Button in buttons:
		if is_instance_valid(button):
			button.get_parent().remove_child(button)
			button.free()
	buttons.clear()


func _free_children(parent: Control) -> void:
	for child: Node in parent.get_children():
		parent.remove_child(child)
		child.free()


func _add_plain_label(parent: Control, text_value: String) -> void:
	var label: Label = Label.new()
	label.text = text_value
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	parent.add_child(label)


func _words(value: Variant) -> String:
	return str(value).replace("_", " ")


func _apply_original_style() -> void:
	for panel: PanelContainer in [%TopRail, %CueBanner, %ResourcePlate, group_pane, focus_pane, chat_pane, feedback_pane]:
		panel.add_theme_stylebox_override(
			"panel",
			_panel_style(
				Color(0.075, 0.105, 0.12, 0.93),
				Color(0.58, 0.42, 0.23, 1.0),
				2,
			),
		)
	for pair: Array in [[hp_bar, Color(0.48, 0.08, 0.08, 1.0)], [stamina_bar, Color(0.62, 0.34, 0.07, 1.0)], [mp_bar, Color(0.08, 0.22, 0.48, 1.0)]]:
		var bar: ProgressBar = pair[0] as ProgressBar
		bar.add_theme_stylebox_override(
			"background",
			_panel_style(
				Color(0.04, 0.055, 0.065, 0.97),
				Color(0.31, 0.24, 0.16, 1.0),
				1,
			),
		)
		bar.add_theme_stylebox_override(
			"fill",
			_panel_style(
				pair[1] as Color,
				Color(0.76, 0.55, 0.26, 1.0),
				1,
			),
		)


func _panel_style(background: Color, border: Color, width: int) -> StyleBoxFlat:
	var style: StyleBoxFlat = StyleBoxFlat.new()
	style.bg_color = background
	style.border_color = border
	style.set_border_width_all(width)
	style.corner_radius_top_left = 3
	style.corner_radius_top_right = 3
	style.corner_radius_bottom_left = 3
	style.corner_radius_bottom_right = 3
	style.shadow_color = Color(0.01, 0.015, 0.02, 0.55)
	style.shadow_size = 4
	style.content_margin_left = 8.0
	style.content_margin_top = 6.0
	style.content_margin_right = 8.0
	style.content_margin_bottom = 6.0
	return style
