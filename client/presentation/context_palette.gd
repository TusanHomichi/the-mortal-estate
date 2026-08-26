class_name ContextPalette
extends PanelContainer

signal option_requested(option_index: int)
signal close_requested

@onready var rows: VBoxContainer = %ContextRows
@onready var status_label: Label = %ContextStatus
@onready var close_button: Button = %ContextClose

var _buttons: Array[Button] = []
var _options: Array[Dictionary] = []
var _command_pending: bool = false
var _origin: Control


func _ready() -> void:
	close_button.pressed.connect(close_palette)


func present_options(options: Array, command_pending: bool, truncated: bool) -> void:
	_options.clear()
	for value: Variant in options:
		if value is Dictionary:
			_options.append((value as Dictionary).duplicate(true))
	_command_pending = command_pending
	if is_node_ready():
		_rebuild(truncated)


func open_palette(origin: Control = null) -> void:
	_origin = origin
	visible = true
	var first: Control = first_focus_control()
	if first != null:
		first.grab_focus()


func close_palette() -> void:
	visible = false
	if is_instance_valid(_origin) and _origin.is_visible_in_tree():
		_origin.grab_focus()
	close_requested.emit()


func first_focus_control() -> Control:
	for button: Button in _buttons:
		if not button.disabled:
			return button
	return close_button


func buttons() -> Array[Button]:
	return _buttons


func _rebuild(truncated: bool) -> void:
	for button: Button in _buttons:
		if is_instance_valid(button):
			rows.remove_child(button)
			button.free()
	_buttons.clear()
	status_label.text = (
		"Authoritative actions incomplete for this frame."
		if truncated
		else "Exact actions from the current authoritative frame."
	)
	for index: int in range(_options.size()):
		var option: Dictionary = _options[index]
		if _is_dedicated_surface_option(option):
			continue
		var button: Button = Button.new()
		button.name = "ContextOption%d" % index
		button.set_meta("option_index", index)
		var enabled: bool = bool(option.get("enabled", false)) and option.get("intent") is Dictionary
		button.disabled = _command_pending or not enabled
		button.text = str(option.get("label", "Action"))
		if not enabled:
			button.text += " [Unavailable: %s]" % _words(option.get("blocked_reason", "server disabled"))
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.tooltip_text = button.text
		button.accessibility_description = button.text
		button.pressed.connect(_emit_option.bind(index))
		rows.add_child(button)
		_buttons.append(button)
	if _buttons.is_empty():
		var empty: Button = Button.new()
		empty.text = "No contextual actions in this frame"
		empty.disabled = true
		rows.add_child(empty)
		_buttons.append(empty)


func _emit_option(index: int) -> void:
	if index < 0 or index >= _options.size():
		return
	option_requested.emit(index)


func _is_dedicated_surface_option(option: Dictionary) -> bool:
	var intent: Variant = option.get("intent")
	if intent is not Dictionary:
		return false
	return intent.get("kind") in ["physical_attack", "warm_spell", "cast_spell", "cast_warmed_spell"]


func _words(value: Variant) -> String:
	return str(value).replace("_", " ")
