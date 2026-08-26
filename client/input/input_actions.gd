class_name InputActions
extends RefCounted

const MOVEMENT_BATCH_MSEC: int = 100
const MAX_MOVE_PATH_STEPS: int = 3

const ACTION_NAMES: Array[String] = [
	"tme_move_north",
	"tme_move_northeast",
	"tme_move_east",
	"tme_move_southeast",
	"tme_move_south",
	"tme_move_southwest",
	"tme_move_west",
	"tme_move_northwest",
	"tme_ui_accept",
	"tme_ui_cancel",
	"tme_focus_next",
	"tme_focus_previous",
	"tme_world_primary",
	"tme_world_secondary",
	"tme_context_palette",
	"tme_help",
	"tme_screenshot",
	"tme_grid_toggle",
	"tme_stairs_up",
	"tme_stairs_down",
	"tme_reconnect",
	"tme_logout",
	"tme_text_scale_increase",
	"tme_text_scale_decrease",
	"tme_text_scale_reset",
]

const MOVEMENT_DIRECTIONS: Dictionary = {
	"tme_move_north": "north",
	"tme_move_northeast": "northeast",
	"tme_move_east": "east",
	"tme_move_southeast": "southeast",
	"tme_move_south": "south",
	"tme_move_southwest": "southwest",
	"tme_move_west": "west",
	"tme_move_northwest": "northwest",
}


static func is_tme_action(action_name: String) -> bool:
	return action_name in ACTION_NAMES


static func action_names() -> PackedStringArray:
	return PackedStringArray(ACTION_NAMES)


static func direction_for_action(action_name: String) -> String:
	return str(MOVEMENT_DIRECTIONS.get(action_name, ""))


static func movement_action_names() -> PackedStringArray:
	return PackedStringArray(MOVEMENT_DIRECTIONS.keys())
