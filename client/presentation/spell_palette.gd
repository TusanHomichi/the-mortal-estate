class_name SpellPalette
extends Control

signal spell_primary_activated(spell_id: String)

const DEFAULT_GRID_COLUMNS: int = 3

@export var single_row: bool = false
@export var empty_text: String = "No ready spells in the current authoritative frame."

@onready var spell_grid: GridContainer = %SpellGrid
@onready var empty_label: Label = %EmptyLabel

var _rows: Array[Dictionary] = []
var _buttons: Array[Button] = []
var _selected_spell_id: String = ""
var _preparing_spell_id: String = ""
var _command_pending: bool = false
var _base_minimum_height: float = 54.0
var _button_signal_depth: int = 0
var _rebuild_queued: bool = false


func present_rows(
	rows: Array,
	selected_spell_id: String = "",
	preparing_spell_id: String = "",
	command_pending: bool = false,
) -> void:
	_rows.clear()
	var seen: Dictionary = {}
	for value: Variant in rows:
		if value is not Dictionary:
			continue
		var row: Dictionary = (value as Dictionary).duplicate(true)
		var spell_id: String = str(row.get("spell_id", ""))
		if spell_id.is_empty() or seen.has(spell_id):
			continue
		seen[spell_id] = true
		_rows.append(row)
	_selected_spell_id = selected_spell_id if seen.has(selected_spell_id) else ""
	_preparing_spell_id = preparing_spell_id if seen.has(preparing_spell_id) else ""
	_command_pending = command_pending
	if is_node_ready():
		_request_rebuild()


func set_command_pending(pending: bool) -> void:
	_command_pending = pending
	for button: Button in _buttons:
		button.disabled = pending


func select_spell(spell_id: String) -> bool:
	if not spell_id in spell_ids():
		return false
	_selected_spell_id = spell_id
	if is_node_ready():
		_request_rebuild()
	return true


func selected_spell_id() -> String:
	return _selected_spell_id


func spell_ids() -> Array[String]:
	var ids: Array[String] = []
	for row: Dictionary in _rows:
		ids.append(str(row.get("spell_id", "")))
	return ids


func ready_control_count() -> int:
	return _buttons.size()


func buttons() -> Array[Button]:
	return _buttons


func row_for_spell(spell_id: String) -> Dictionary:
	for row: Dictionary in _rows:
		if row.get("spell_id") == spell_id:
			return row.duplicate(true)
	return {}


func focus_spell(spell_id: String) -> bool:
	for button: Button in _buttons:
		if button.get_meta("spell_id", "") == spell_id:
			button.grab_focus()
			return true
	return false


func _ready() -> void:
	_base_minimum_height = maxf(54.0, custom_minimum_size.y)
	empty_label.text = empty_text
	_request_rebuild()


func _notification(what: int) -> void:
	if what == NOTIFICATION_THEME_CHANGED and is_node_ready():
		call_deferred("_synchronize_minimum_height")


func _request_rebuild() -> void:
	if _button_signal_depth > 0 or _rebuild_queued:
		_queue_rebuild()
		return
	_rebuild()


func _queue_rebuild() -> void:
	if _rebuild_queued:
		return
	_rebuild_queued = true
	call_deferred("_run_queued_rebuild")


func _run_queued_rebuild() -> void:
	if not _rebuild_queued:
		return
	if not is_node_ready():
		return
	_rebuild_queued = false
	_rebuild()


func _rebuild() -> void:
	if _button_signal_depth > 0:
		_queue_rebuild()
		return
	var focused_spell_id: String = ""
	var focus_owner: Control = get_viewport().gui_get_focus_owner()
	if focus_owner != null and focus_owner in _buttons:
		focused_spell_id = str(focus_owner.get_meta("spell_id", ""))
	for button: Button in _buttons:
		if is_instance_valid(button):
			spell_grid.remove_child(button)
			button.free()
	_buttons.clear()
	spell_grid.columns = (
		maxi(1, _rows.size())
		if single_row
		else DEFAULT_GRID_COLUMNS
	)
	empty_label.visible = _rows.is_empty()
	for row: Dictionary in _rows:
		var spell_id: String = str(row.get("spell_id", ""))
		var button: Button = Button.new()
		button.name = "Spell_" + spell_id.validate_node_name()
		button.set_meta("spell_id", spell_id)
		button.custom_minimum_size = Vector2(104.0, 42.0)
		button.disabled = _command_pending
		button.text = _button_text(row)
		button.tooltip_text = _spell_description(row)
		button.accessibility_description = button.tooltip_text
		button.pressed.connect(_emit_spell_primary.bind(spell_id))
		spell_grid.add_child(button)
		_buttons.append(button)
	_synchronize_minimum_height()
	if not focused_spell_id.is_empty():
		focus_spell(focused_spell_id)


func _emit_spell_primary(spell_id: String) -> void:
	_button_signal_depth += 1
	spell_primary_activated.emit(spell_id)
	_button_signal_depth -= 1


func _synchronize_minimum_height() -> void:
	if not is_node_ready():
		return
	var required_height: float = _base_minimum_height
	if not _buttons.is_empty():
		var row_height: float = 0.0
		for button: Button in _buttons:
			row_height = maxf(row_height, button.get_combined_minimum_size().y)
		var row_count: int = ceili(float(_buttons.size()) / float(maxi(1, spell_grid.columns)))
		required_height = maxf(
			required_height,
			row_height * row_count
			+ float(maxi(0, row_count - 1))
			* float(spell_grid.get_theme_constant("v_separation")),
		)
	if not is_equal_approx(custom_minimum_size.y, required_height):
		custom_minimum_size.y = required_height


func _button_text(row: Dictionary) -> String:
	var spell_id: String = str(row.get("spell_id", ""))
	var marker: String = "◆ " if spell_id == _selected_spell_id else ""
	if spell_id == _preparing_spell_id:
		marker = "◈ "
	var state: Dictionary = _activation_state(row)
	var state_text: String = "Ready" if bool(state.get("enabled", false)) else _words(state.get("blocked_reason", "Unavailable"))
	return "%s%s\n%s" % [marker, str(row.get("spell_name", spell_id)), state_text]


func _spell_description(row: Dictionary) -> String:
	var state: Dictionary = _activation_state(row)
	var readiness: String = "Ready" if bool(state.get("enabled", false)) else "Unavailable: " + _words(state.get("blocked_reason", "server disabled"))
	return "%s; %s; target %s; MP %s; stamina %s" % [
		str(row.get("spell_name", row.get("spell_id", "Spell"))),
		readiness,
		"none" if row.get("target_kind") == null else _words(row.get("target_kind")),
		"none" if row.get("mp_cost") == null else str(row.get("mp_cost")),
		"none" if row.get("stamina_cost") == null else str(row.get("stamina_cost")),
	]


func _activation_state(row: Dictionary) -> Dictionary:
	return row.get("cast", {}) if row.get("casting_method") == "direct" else row.get("warm", {})


func _words(value: Variant) -> String:
	return str(value).replace("_", " ").capitalize()
