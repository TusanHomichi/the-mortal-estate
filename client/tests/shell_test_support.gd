extends RefCounted

## Shared scaffolding for the world-shell suites.
##
## `test_input_actions.gd` grew past a thousand lines carrying three unrelated
## subjects and one pile of helpers. The subjects now live in three suites —
## bindings and accessibility, pointer movement, and world-shell actions — and
## everything they share lives here, as static functions with no suite state of
## their own.
##
## Nothing here asserts. A helper that asserted would put a failure in a file
## whose name says nothing about what failed.

const SCREEN_PATHS: Array[String] = [
	"res://scenes/LoginScreen.tscn",
	"res://scenes/CharacterSelectScreen.tscn",
	"res://scenes/WorldShellScreen.tscn",
]


static func add_screen(path: String) -> Control:
	var screen: Control = (load(path) as PackedScene).instantiate() as Control
	add_to_tree(screen)
	return screen


static func add_to_tree(node: Node) -> void:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	tree.root.size = Vector2i(1280, 720)
	if node is Control:
		var control: Control = node as Control
		control.set_anchors_preset(Control.PRESET_TOP_LEFT)
		control.position = Vector2.ZERO
		control.size = Vector2(1280, 720)
	tree.root.add_child(node)


## The width a label's text would take if nothing wrapped it. `get_minimum_size`
## cannot answer this for a wrapping label — it reports the narrowest column the
## text could be squeezed into — so the string is measured against the font the
## label is actually drawing with.
static func unwrapped_text_width(label: Label) -> float:
	var font: Font = label.get_theme_font("font")
	var font_size: int = label.get_theme_font_size("font_size")
	if font == null:
		return 0.0
	return font.get_string_size(
		label.text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size
	).x


## The longest state the readiness line has: a multi-beat wait, no measured fill,
## both of the frame's own times, and a spell warming in the preparation band.
static func longest_beat_frame() -> Dictionary:
	var frame: Dictionary = frame_with_options([])
	frame["logical_time"] = "12"
	frame["ready_at"] = "15"
	frame["can_act"] = false
	frame["warmed_spell"] = {
		"spell_id": "greater_conflagration",
		"warmed_at": "12",
		"ready_at": "15",
		"status": "warming",
	}
	return frame


static func frame_with_options(options: Array) -> Dictionary:
	var frame := {
		"observer_actor_id": "player",
		"observation_center": {"realm": "synthetic", "level": "surface", "position": {"x": 0, "y": 0}},
		"observation_radius": 7,
		"tiles": [{"position": {"x": 0, "y": 0}, "terrain_id": "town_floor", "passable": true}],
		"actors": [],
		"corpses": [],
		"ground_items": [],
		"gold_piles": [],
		"action_options": options,
	}
	frame["static_scene_context"] = static_context_for_tiles(frame["tiles"])
	return frame


static func expected_default_binding_signatures() -> Dictionary:
	return {
		"tme_move_north": ["key:87:0:0:0:0", "key:4194320:0:0:0:0", "key:4194446:0:0:0:0", "joy:11"],
		"tme_move_northeast": ["key:69:0:0:0:0", "key:4194447:0:0:0:0"],
		"tme_move_east": ["key:68:0:0:0:0", "key:4194321:0:0:0:0", "key:4194444:0:0:0:0", "joy:14"],
		"tme_move_southeast": ["key:67:0:0:0:0", "key:4194441:0:0:0:0"],
		"tme_move_south": ["key:83:0:0:0:0", "key:4194322:0:0:0:0", "key:4194440:0:0:0:0", "joy:12"],
		"tme_move_southwest": ["key:90:0:0:0:0", "key:4194439:0:0:0:0"],
		"tme_move_west": ["key:65:0:0:0:0", "key:4194319:0:0:0:0", "key:4194442:0:0:0:0", "joy:13"],
		"tme_move_northwest": ["key:81:0:0:0:0", "key:4194445:0:0:0:0"],
		"tme_ui_accept": ["key:4194309:0:0:0:0", "key:32:0:0:0:0", "joy:0"],
		"tme_ui_cancel": ["key:4194305:0:0:0:0", "joy:1"],
		"tme_focus_next": ["key:4194306:0:0:0:0", "joy:10"],
		"tme_focus_previous": ["key:4194306:1:0:0:0", "joy:9"],
		"tme_world_primary": ["mouse:1"],
		"tme_world_secondary": ["mouse:2"],
		"tme_context_palette": ["key:4194306:0:0:0:0"],
		"tme_help": ["key:4194332:0:0:0:0"],
		"tme_screenshot": ["key:4194343:0:0:0:0"],
		"tme_grid_toggle": ["key:71:0:0:0:0"],
		"tme_stairs_up": ["key:4194323:0:0:0:0"],
		"tme_stairs_down": ["key:4194324:0:0:0:0"],
		"tme_reconnect": ["key:4194336:0:0:0:0"],
		"tme_logout": ["key:76:0:1:0:0"],
		"tme_text_scale_increase": ["key:61:0:1:0:0"],
		"tme_text_scale_decrease": ["key:45:0:1:0:0"],
		"tme_text_scale_reset": ["key:48:0:1:0:0"],
	}


static func event_signatures(events: Array[InputEvent]) -> Array[String]:
	var signatures: Array[String] = []
	for event: InputEvent in events:
		if event is InputEventKey:
			var key: InputEventKey = event as InputEventKey
			signatures.append("key:%d:%d:%d:%d:%d" % [key.physical_keycode, int(key.shift_pressed), int(key.ctrl_pressed), int(key.alt_pressed), int(key.meta_pressed)])
		elif event is InputEventMouseButton:
			signatures.append("mouse:" + str((event as InputEventMouseButton).button_index))
		elif event is InputEventJoypadButton:
			signatures.append("joy:" + str((event as InputEventJoypadButton).button_index))
	return signatures


static func sorted_keys(value: Dictionary) -> Array[String]:
	var keys: Array[String] = []
	for key: Variant in value.keys():
		keys.append(str(key))
	keys.sort()
	return keys


static func dictionary_has_key_fragment(value: Variant, fragment: String) -> bool:
	if value is Dictionary:
		for key: Variant in (value as Dictionary).keys():
			if str(key).to_lower().contains(fragment):
				return true
			if dictionary_has_key_fragment((value as Dictionary)[key], fragment):
				return true
	elif value is Array:
		for child: Variant in value:
			if dictionary_has_key_fragment(child, fragment):
				return true
	return false


static func write_json(path: String, value: Dictionary) -> bool:
	var file: FileAccess = FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(value))
	return file.get_error() == OK


static func remove_file(path: String) -> void:
	if FileAccess.file_exists(path):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(path))


static func world_frame(options: Array) -> Dictionary:
	var tiles: Array = []
	for y: int in range(-7, 8):
		for x: int in range(-7, 8):
			tiles.append({"position": {"x": x, "y": y}, "terrain_id": "synthetic", "passable": true})
	var frame := {
		"logical_time": "10",
		"ready_at": "10",
		"can_act": true,
		"observer_actor_id": "player",
		"observation_center": {"realm": "synthetic", "level": "surface", "position": {"x": 0, "y": 0}},
		"observation_radius": 7,
		"tiles": tiles,
			"actors": [{"actor_id": "player", "name": "Player", "kind": "player", "character_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "position": {"realm": "synthetic", "level": "surface", "position": {"x": 0, "y": 0}}, "life_state": "alive", "hp": 10, "max_hp": 10, "attack_safety": "invalid"}],
		"corpses": [],
		"ground_items": [],
		"gold_piles": [],
		"action_options": options,
	}
	frame["static_scene_context"] = static_context_for_tiles(tiles)
	return frame


static func static_context_for_tiles(tiles: Array) -> Dictionary:
	var static_tiles: Array = []
	var walkable_mask: Array = []
	var minimum := Vector2i(2147483647, 2147483647)
	var maximum := Vector2i(-2147483648, -2147483648)
	for value: Variant in tiles:
		var tile: Dictionary = value as Dictionary
		var position: Dictionary = tile["position"]
		var coordinate := Vector2i(int(position["x"]), int(position["y"]))
		minimum = Vector2i(mini(minimum.x, coordinate.x), mini(minimum.y, coordinate.y))
		maximum = Vector2i(maxi(maximum.x, coordinate.x), maxi(maximum.y, coordinate.y))
		var walkable: bool = bool(tile.get("passable", true))
		static_tiles.append({
			"position": position.duplicate(true),
			"terrain_ids": [str(tile["terrain_id"])],
			"walkable": walkable,
		})
		if walkable:
			walkable_mask.append(position.duplicate(true))
	return {
		"contract_version": 1,
		"site": {"realm": "synthetic", "level": "surface"},
		"bounds": {
			"min": {"x": minimum.x, "y": minimum.y},
			"max": {"x": maximum.x, "y": maximum.y},
		},
		"content_digest": "a".repeat(64),
		"visual_manifest_digest": "b".repeat(64),
		"scene_role": "overworld",
		"presentation_mode": "overworld_town",
		"world_zoom": [156, 104],
		"tiles": static_tiles,
		"walkable_mask": walkable_mask,
		"static_props": [],
		"transition_apertures": [],
	}


## What the world view is currently presenting about movement: "none" for no
## draft, "pending" while authority has not answered, "preview" once it has.


## What the world view is currently presenting about movement: "none" for no
## draft, "pending" while authority has not answered, "preview" once it has.
static func draft_presentation(screen: WorldShellScreen) -> String:
	var text: String = (screen.world_view as GridWorldView).interaction_label.text
	if text.begins_with("Movement draft"):
		return "pending"
	if text.begins_with("Authoritative preview"):
		return "preview"
	return "none"


static func preview_result(path: Array) -> Dictionary:
	var steps: Array = []
	var current: Vector2i = Vector2i.ZERO
	for index: int in range(path.size()):
		var next: Vector2i = current + ClientReachability.STEP_BY_DIRECTION.get(path[index], Vector2i.ZERO)
		steps.append({
			"index": str(index), "direction": path[index],
			"from": {"position": {"x": current.x, "y": current.y}},
			"attempted": {"position": {"x": next.x, "y": next.y}},
			"opens_door": index == path.size() - 1 and path.size() > 1,
			"outcome": {"kind": "transitioned" if index == path.size() - 1 and path.size() > 1 else "moved"},
		})
		current = next
	return {"disposition": {"kind": "previewed"}, "preview": {"requested_path": path.duplicate(), "pace": ["walk", "run", "sprint"][path.size() - 1], "accepted_steps": str(path.size()), "steps": steps, "stop_reason": "full_path_accepted"}}


static func fk_ground_reach_frame() -> Dictionary:
	var frame: Dictionary = world_frame([
		{
			"id": "move_item:item:own",
			"label": "Stow own item",
			"enabled": true,
			"blocked_reason": null,
			"intent": {
				"kind": "move_item",
				"item_instance_id": "item:own",
				"destination": {"kind": "carried", "position": "sack_item_1"},
			},
		},
		{
			"id": "search_corpse:own",
			"label": "Search own corpse",
			"enabled": true,
			"blocked_reason": null,
			"intent": {"kind": "search_corpse", "corpse_id": "corpse:own"},
		},
	])
	for tile: Dictionary in frame["tiles"]:
		tile["terrain_id"] = "town_floor"
	var coordinates: Dictionary = {
		"own": Vector2i.ZERO,
		"adjacent": Vector2i(1, 0),
		"distant": Vector2i(2, 0),
	}
	for label: String in coordinates:
		var coordinate: Vector2i = coordinates[label]
		var location: Dictionary = {
			"realm": "synthetic",
			"level": "surface",
			"position": {"x": coordinate.x, "y": coordinate.y},
		}
		frame["ground_items"].append({
			"item_instance_id": "item:" + label,
			"name": label.capitalize() + " item",
			"quantity": 1,
			"location": location.duplicate(true),
		})
		frame["corpses"].append({
			"corpse_id": "corpse:" + label,
			"origin_name": label.capitalize(),
			"searched": false,
			"location": location.duplicate(true),
		})
		frame["gold_piles"].append({
			"gold_pile_id": "gold:" + label,
			"amount": "1",
			"location": location.duplicate(true),
		})
	frame["action_options_truncated"] = false
	frame["ground_items_truncated"] = false
	return frame


static func ground_target(
	screen: WorldShellScreen,
	kind: String,
	source_identity: String,
) -> Dictionary:
	var view: GridWorldView = screen.world_view as GridWorldView
	var matches: Array[Dictionary] = view.targets().filter(
		func(target: Dictionary) -> bool:
			return (
				target.get("kind") == kind
				and target.get("source_identity") == source_identity
			)
	)
	return matches[0] if matches.size() == 1 else {}


static func world_frame_with_blocked(blocked: Array) -> Dictionary:
	var frame := world_frame([])
	var tiles: Array = []
	for value: Variant in frame["tiles"]:
		var tile: Dictionary = value
		var position: Dictionary = tile["position"]
		tile["passable"] = Vector2i(int(position["x"]), int(position["y"])) not in blocked
		tiles.append(tile)
	frame["tiles"] = tiles
	frame["static_scene_context"] = static_context_for_tiles(tiles)
	return frame


static func route_end_if_legal(start: Vector2i, directions: Array[String], blocked: Array) -> Vector2i:
	var steps: Dictionary = {
		"north": Vector2i(0, -1),
		"northeast": Vector2i(1, -1),
		"east": Vector2i(1, 0),
		"southeast": Vector2i(1, 1),
		"south": Vector2i(0, 1),
		"southwest": Vector2i(-1, 1),
		"west": Vector2i(-1, 0),
		"northwest": Vector2i(-1, -1),
	}
	var current: Vector2i = start
	for direction: String in directions:
		var step: Vector2i = steps.get(direction, Vector2i.ZERO)
		if step == Vector2i.ZERO:
			return Vector2i(999, 999)
		if step.x != 0 and step.y != 0 and (current + Vector2i(step.x, 0) in blocked or current + Vector2i(0, step.y) in blocked):
			return Vector2i(999, 999)
		current += step
		if current in blocked:
			return Vector2i(999, 999)
	return current
