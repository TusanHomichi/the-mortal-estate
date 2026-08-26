class_name CharacterSelectScreen
extends Control

signal character_selected(character_id: String)
signal logout_requested

@onready var mark_status: Label = %MarkStatus
@onready var character_list: VBoxContainer = %CharacterList
@onready var selection_status: Label = %SelectionStatus
@onready var continue_button: Button = %ContinueButton
@onready var logout_button: Button = %LogoutButton

var _character_buttons: Array[Button] = []
var _selected_character_id: String = ""


func _ready() -> void:
	continue_button.pressed.connect(_on_continue_pressed)
	logout_button.pressed.connect(func() -> void: logout_requested.emit())


func render_bootstrap(bootstrap: Dictionary) -> void:
	_clear_character_buttons()
	_selected_character_id = ""
	selection_status.text = "Tap a character, then tap Continue."
	continue_button.text = "Select a Character to Continue"
	continue_button.disabled = true
	var mark_state: Dictionary = bootstrap.get("player_kill_marks", {})
	var mark_count: int = int(mark_state.get("active_count", 0))
	var lock_text: String = "LOCKED" if mark_state.get("gameplay_locked", false) else "Available"
	mark_status.text = "◆ Player-kill marks: %d active | Gameplay: %s" % [mark_count, lock_text]
	for character_value: Variant in bootstrap.get("characters", []):
		var character: Dictionary = character_value as Dictionary
		var button: Button = Button.new()
		button.name = "CharacterSlot" + str(character.get("slot", 0))
		var display_name: String = str(character.get("display_name", ""))
		var base_label: String = "Slot %d — %s" % [int(character.get("slot", 0)), display_name]
		button.text = base_label
		button.tooltip_text = "Select this server-owned character slot"
		button.custom_minimum_size = Vector2(0.0, 56.0)
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.toggle_mode = true
		button.set_meta("character_id", str(character.get("character_id", "")))
		button.set_meta("display_name", display_name)
		button.set_meta("base_label", base_label)
		button.pressed.connect(_on_character_pressed.bind(button))
		character_list.add_child(button)
		_character_buttons.append(button)
	_rebuild_focus_order()


func first_focus_control() -> Control:
	if not _character_buttons.is_empty():
		return _character_buttons[0]
	return logout_button


func apply_text_scale(percent: int) -> void:
	var scaled_theme: Theme = Theme.new()
	scaled_theme.default_font_size = 16 * percent / 100
	theme = scaled_theme


func _clear_character_buttons() -> void:
	for button: Button in _character_buttons:
		button.queue_free()
	_character_buttons.clear()


func _rebuild_focus_order() -> void:
	var focus_controls: Array[Control] = []
	for button: Button in _character_buttons:
		focus_controls.append(button)
	focus_controls.append(continue_button)
	focus_controls.append(logout_button)
	for index: int in range(focus_controls.size()):
		var current: Control = focus_controls[index]
		var previous: Control = focus_controls[(index - 1 + focus_controls.size()) % focus_controls.size()]
		var next: Control = focus_controls[(index + 1) % focus_controls.size()]
		current.focus_neighbor_top = current.get_path_to(previous)
		current.focus_neighbor_bottom = current.get_path_to(next)
	if not focus_controls.is_empty():
		focus_controls[0].call_deferred("grab_focus")


func selected_character_id() -> String:
	return _selected_character_id


func _on_character_pressed(button: Button) -> void:
	var character_id: String = str(button.get_meta("character_id", ""))
	if character_id.is_empty():
		return
	_selected_character_id = character_id
	var display_name: String = str(button.get_meta("display_name", ""))
	for candidate: Button in _character_buttons:
		var selected: bool = candidate == button
		candidate.button_pressed = selected
		candidate.text = ("◆ Selected — " if selected else "") + str(candidate.get_meta("base_label", ""))
	selection_status.text = "Selected: %s. Tap Continue to enter the world." % display_name
	continue_button.text = "Continue with %s" % display_name
	continue_button.disabled = false
	continue_button.grab_focus()


func _on_continue_pressed() -> void:
	if _selected_character_id.is_empty() or continue_button.disabled:
		return
	character_selected.emit(_selected_character_id)
