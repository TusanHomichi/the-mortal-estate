class_name TmeConfirmationDialog
extends Control

signal accepted(payload: Dictionary)
signal dismissed

@onready var prompt_label: Label = %PromptLabel
@onready var cancel_button: Button = %CancelButton
@onready var confirm_button: Button = %ConfirmButton

var _origin: Control = null
var _generation: int = -1
var _payload: Dictionary = {}


func _ready() -> void:
	cancel_button.pressed.connect(cancel)
	confirm_button.pressed.connect(confirm)
	cancel_button.focus_neighbor_right = cancel_button.get_path_to(confirm_button)
	confirm_button.focus_neighbor_left = confirm_button.get_path_to(cancel_button)
	cancel_button.focus_neighbor_left = cancel_button.get_path_to(confirm_button)
	confirm_button.focus_neighbor_right = confirm_button.get_path_to(cancel_button)


func open_confirmation(action_label: String, target_label: String, frame_generation: int, focus_origin: Control, payload: Dictionary = {}) -> void:
	_origin = focus_origin
	_generation = frame_generation
	_payload = payload.duplicate(true)
	prompt_label.text = "%s — target: %s" % [action_label, target_label]
	visible = true
	cancel_button.grab_focus()


func confirm(current_generation: int = -1) -> bool:
	if not visible:
		return false
	if _generation >= 0 and current_generation >= 0 and current_generation != _generation:
		_close(false)
		return false
	var payload: Dictionary = _payload.duplicate(true)
	_close(true)
	accepted.emit(payload)
	return true


func cancel() -> void:
	if visible:
		_close(true)
		dismissed.emit()


func invalidate_if_generation_changed(current_generation: int) -> bool:
	if not visible or _generation < 0 or current_generation == _generation:
		return false
	_close(true)
	dismissed.emit()
	return true


func cancel_has_default_focus() -> bool:
	return get_viewport().gui_get_focus_owner() == cancel_button


func prompt_text() -> String:
	return prompt_label.text


func _close(restore_focus: bool) -> void:
	visible = false
	_generation = -1
	_payload.clear()
	if restore_focus and is_instance_valid(_origin) and _origin.is_visible_in_tree() and _origin.focus_mode != Control.FOCUS_NONE:
		_origin.grab_focus()
