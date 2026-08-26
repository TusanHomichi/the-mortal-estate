class_name HelpOverlay
extends PanelContainer

signal close_requested

@onready var help_text: RichTextLabel = %HelpText
@onready var close_button: Button = %HelpClose

var _origin: Control


func _ready() -> void:
	close_button.pressed.connect(close_help)
	refresh_bindings()


func open_help(origin: Control = null) -> void:
	_origin = origin
	refresh_bindings()
	visible = true
	close_button.grab_focus()


func close_help() -> void:
	visible = false
	if is_instance_valid(_origin) and _origin.is_visible_in_tree():
		_origin.grab_focus()
	close_requested.emit()


func refresh_bindings() -> void:
	if not is_node_ready():
		return
	var lines: Array[String] = [
		"POINTER & ACTIVATION",
		"%s — focus/select; repeat on the same target to activate" % _binding_text("tme_world_primary"),
		"%s — open target context" % _binding_text("tme_world_secondary"),
		"Drag a loose item at least 8 UI pixels to a specific carried destination.",
		"",
		"DISCOVERY",
		"%s — contextual exact actions" % _binding_text("tme_context_palette"),
		"%s — this gesture help" % _binding_text("tme_help"),
		"%s / %s — keyboard focus" % [_binding_text("tme_focus_next"), _binding_text("tme_focus_previous")],
		"%s — activate focused control" % _binding_text("tme_ui_accept"),
		"%s — cancel current local gesture or close overlay" % _binding_text("tme_ui_cancel"),
		"",
		"MOVEMENT",
		"Movement bindings queue one to three exact directions. A tile activation previews; repeat activation commits the current server preview.",
		"%s / %s — stairs" % [_binding_text("tme_stairs_up"), _binding_text("tme_stairs_down")],
		"%s — toggle the walkable-cell grid" % _binding_text("tme_grid_toggle"),
		"",
		"COMBAT & SPELLS",
		"Activate an actor twice to use the sole exact close mode or your selected ranged mode.",
		"Activate a spell once to select it; activate the same spell again to Prepare.",
		"Blocked reasons and outcomes always come from the authoritative frame.",
	]
	help_text.text = "\n".join(lines)


func _binding_text(action_name: String) -> String:
	var names: Array[String] = []
	for event: InputEvent in InputMap.action_get_events(action_name):
		names.append(event.as_text())
	return " / ".join(names) if not names.is_empty() else action_name
