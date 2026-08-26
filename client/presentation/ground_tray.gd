class_name GroundTray
extends PanelContainer

signal exact_intent_requested(intent: Dictionary, label: String, origin: Control)

const INCOMPLETE_MESSAGE: String = "Authoritative actions incomplete for this frame"

@onready var title_label: Label = %GroundTitle
@onready var status_label: Label = %GroundStatus
@onready var rows: VBoxContainer = %GroundRows
@onready var close_button: Button = %GroundClose

var _frame: Dictionary = {}
var _frame_generation: int = -1
var _coordinate: Vector2i = Vector2i.ZERO
var _pinned_identity: String = ""
var _buttons: Array[Button] = []


func _ready() -> void:
	close_button.pressed.connect(func() -> void: visible = false)


func pin_target(target: Dictionary) -> void:
	_pinned_identity = str(target.get("identity", ""))
	_coordinate = _coord(target.get("coordinate", {}))
	visible = true
	if is_node_ready():
		_rebuild()


func present_frame(frame: Dictionary, frame_generation: int) -> void:
	_frame = frame.duplicate(true)
	_frame_generation = frame_generation
	if is_node_ready() and visible:
		_rebuild()


func discard() -> void:
	_frame.clear()
	_frame_generation = -1
	_pinned_identity = ""
	visible = false
	if is_node_ready():
		_rebuild()


## The tray shows ground state anywhere it is pinned, but only the square the
## controlled character stands on can be acted on.
func is_view_only() -> bool:
	var player_coordinate_value: Variant = WorldTargets.controlled_player_coordinate(_frame)
	return (
		player_coordinate_value == null
		or _coordinate != (player_coordinate_value as Vector2i)
	)


func pinned_coordinate() -> Vector2i:
	return _coordinate


func projected_items() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	for value: Variant in _frame.get("ground_items", []):
		var item: Dictionary = value as Dictionary
		if _coord(item.get("location", {})) == _coordinate:
			result.append(item.duplicate(true))
	result.sort_custom(func(left: Dictionary, right: Dictionary) -> bool:
		return str(left.get("item_instance_id", "")) < str(right.get("item_instance_id", ""))
	)
	return result


func fan_items() -> Array[Dictionary]:
	var values: Array[Dictionary] = projected_items()
	return values.slice(0, mini(4, values.size()))


func overflow_count() -> int:
	return maxi(0, projected_items().size() - fan_items().size())


func exact_search_option(corpse_id: String) -> Dictionary:
	if is_view_only() or _actions_incomplete():
		return {}
	return _find_option(func(intent: Dictionary) -> bool:
		return intent.get("kind") == "search_corpse" and intent.get("corpse_id") == corpse_id
	)


func exact_item_destination_option(item_instance_id: String, destination_position: String = "") -> Dictionary:
	if is_view_only() or _actions_incomplete():
		return {}
	var matches: Array[Dictionary] = []
	for value: Variant in _frame.get("action_options", []):
		var option: Dictionary = value as Dictionary
		var intent: Variant = option.get("intent")
		if intent is not Dictionary or intent.get("kind") != "move_item":
			continue
		if intent.get("item_instance_id") != item_instance_id:
			continue
		var destination: Dictionary = intent.get("destination", {})
		if destination.get("kind") != "carried":
			continue
		var position: String = str(destination.get("position", ""))
		if not destination_position.is_empty() and position != destination_position:
			continue
		if bool(option.get("enabled", false)):
			matches.append(option.duplicate(true))
	if not destination_position.is_empty():
		return matches[0] if matches.size() == 1 else {}
	matches = matches.filter(func(option: Dictionary) -> bool:
		return str(option.get("intent", {}).get("destination", {}).get("position", "")).begins_with("sack_item_")
	)
	matches.sort_custom(func(left: Dictionary, right: Dictionary) -> bool:
		return _sack_index(left) < _sack_index(right)
	)
	return matches[0] if not matches.is_empty() else {}


func _rebuild() -> void:
	for button: Button in _buttons:
		if is_instance_valid(button):
			rows.remove_child(button)
			button.free()
	_buttons.clear()
	title_label.text = "◇ GROUND · %d,%d" % [_coordinate.x, _coordinate.y]
	status_label.text = "Stand on this tile to take or search" if is_view_only() else (
		INCOMPLETE_MESSAGE if _actions_incomplete() else "Pinned exact projected ground state"
	)
	var found: bool = false
	for value: Variant in _frame.get("corpses", []):
		var corpse: Dictionary = value as Dictionary
		if _coord(corpse.get("location", {})) != _coordinate:
			continue
		found = true
		var searched_text: String = "Searched" if bool(corpse.get("searched", false)) else "Contents not searched"
		_add_action_row(
			"%s corpse · %s" % [corpse.get("origin_name", "Unknown"), searched_text],
			exact_search_option(str(corpse.get("corpse_id", ""))),
			"Search corpse",
		)
	for item: Dictionary in projected_items():
		found = true
		var claim: String = "" if item.get("loot_claim") == null else " · claimed"
		_add_action_row(
			"%s ×%s · %s%s" % [item.get("name", "Unknown item"), item.get("quantity", 1), item.get("item_instance_id", "?"), claim],
			exact_item_destination_option(str(item.get("item_instance_id", ""))),
			"Stow in sack",
		)
	for value: Variant in _frame.get("gold_piles", []):
		var gold: Dictionary = value as Dictionary
		if _coord(gold.get("location", {})) != _coordinate:
			continue
		found = true
		_add_plain_row("%s gold · %s" % [gold.get("amount", "?"), gold.get("gold_pile_id", "?")])
	if not found:
		_add_plain_row("No projected corpse, loose item, or gold at this tile.")


func _add_action_row(text_value: String, option: Dictionary, verb: String) -> void:
	var button: Button = Button.new()
	button.text = text_value
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	if option.is_empty():
		button.disabled = true
		var reason: String = (
			INCOMPLETE_MESSAGE
			if _actions_incomplete()
			else "Stand on this tile to take or search"
			if is_view_only()
			else "No exact action in this frame"
		)
		button.text += "\n[%s]" % reason
		button.tooltip_text = reason
	else:
		button.text += "\n%s" % verb
		button.pressed.connect(_emit_exact.bind(option, verb, button))
	rows.add_child(button)
	_buttons.append(button)


func _add_plain_row(text_value: String) -> void:
	var button: Button = Button.new()
	button.text = text_value
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.disabled = true
	rows.add_child(button)
	_buttons.append(button)


func _emit_exact(option: Dictionary, label: String, origin: Control) -> void:
	if bool(option.get("enabled", false)) and option.get("intent") is Dictionary:
		exact_intent_requested.emit((option["intent"] as Dictionary).duplicate(true), label, origin)


func _find_option(predicate: Callable) -> Dictionary:
	var matches: Array[Dictionary] = []
	for value: Variant in _frame.get("action_options", []):
		var option: Dictionary = value as Dictionary
		var intent: Variant = option.get("intent")
		if bool(option.get("enabled", false)) and intent is Dictionary and predicate.call(intent):
			matches.append(option.duplicate(true))
	return matches[0] if matches.size() == 1 else {}


func _actions_incomplete() -> bool:
	return bool(_frame.get("action_options_truncated", false)) or bool(_frame.get("ground_items_truncated", false))


func _sack_index(option: Dictionary) -> int:
	var position: String = str(option.get("intent", {}).get("destination", {}).get("position", ""))
	return int(position.trim_prefix("sack_item_")) if position.begins_with("sack_item_") else 2147483647


func _coord(value: Variant) -> Vector2i:
	return WorldTargets.coordinate(value)
