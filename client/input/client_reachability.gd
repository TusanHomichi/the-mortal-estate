class_name ClientReachability
extends RefCounted

## Client-side movement proposal facts derived only from the current projected
## frame. The server preview remains authoritative for every submitted path.

const STEP_BY_DIRECTION: Dictionary = {
	"north": Vector2i(0, -1),
	"northeast": Vector2i(1, -1),
	"east": Vector2i(1, 0),
	"southeast": Vector2i(1, 1),
	"south": Vector2i(0, 1),
	"southwest": Vector2i(-1, 1),
	"west": Vector2i(-1, 0),
	"northwest": Vector2i(-1, -1),
}

const PATH_DIRECTIONS: Array[Vector2i] = [
	Vector2i(0, -1),
	Vector2i(1, -1),
	Vector2i(1, 0),
	Vector2i(1, 1),
	Vector2i(0, 1),
	Vector2i(-1, 1),
	Vector2i(-1, 0),
	Vector2i(-1, -1),
]


static func frame_facts(frame: Dictionary) -> Dictionary:
	var center := _coord(frame.get("observation_center", {}))
	var walkable: Dictionary = {}
	var context: Dictionary = frame.get("static_scene_context", {})
	for value: Variant in context.get("walkable_mask", []):
		walkable[_coord(value)] = true

	# Observer tiles are a radius-boxed window. Presence supplies the brief's
	# visibility condition; only projected Boolean passability is a dynamic
	# override. LOS-redacted rows omit that field and therefore retain the
	# entity-free static mask rather than inventing client-side sight.
	var visible: Dictionary = {}
	for value: Variant in frame.get("tiles", []):
		if value is not Dictionary:
			continue
		var tile := value as Dictionary
		var coordinate := _coord(tile.get("position", {}))
		visible[coordinate] = true
		if tile.get("passable") is bool:
			if bool(tile["passable"]):
				walkable[coordinate] = true
			else:
				walkable.erase(coordinate)

	var flood := _flood(center, walkable, InputActions.MAX_MOVE_PATH_STEPS)
	var reach_cells: Array[Vector2i] = []
	var reach_lookup: Dictionary = {}
	for coordinate_value: Variant in (flood.get("depth", {}) as Dictionary).keys():
		var coordinate := coordinate_value as Vector2i
		# A rendered square must author a positive-length movement draft. The
		# occupied origin is depth zero, so it is not a destination square.
		if coordinate == center or not visible.has(coordinate):
			continue
		reach_cells.append(coordinate)
		reach_lookup[coordinate] = true
	reach_cells.sort_custom(func(left: Vector2i, right: Vector2i) -> bool:
		return left.y < right.y or (left.y == right.y and left.x < right.x)
	)
	return {
		"center": center,
		"walkable": walkable,
		"visible": visible,
		"came_from": flood.get("came_from", {}),
		"depth": flood.get("depth", {}),
		"reach_cells": reach_cells,
		"reach_lookup": reach_lookup,
	}


static func proposed_path(frame: Dictionary, target: Vector2i) -> Array[String]:
	var facts := frame_facts(frame)
	if not (facts.get("reach_lookup", {}) as Dictionary).has(target):
		return []
	var start: Vector2i = facts.get("center", Vector2i.ZERO)
	var walkable: Dictionary = facts.get("walkable", {})
	var direct := _direct_path(start, target, InputActions.MAX_MOVE_PATH_STEPS)
	if _path_is_open(start, direct, walkable):
		return direct
	return _route_directions(
		facts.get("came_from", {}) as Dictionary,
		start,
		target,
	)


static func _flood(start: Vector2i, walkable: Dictionary, budget: int) -> Dictionary:
	var came_from: Dictionary = {}
	var depth: Dictionary = {}
	if not walkable.has(start):
		return {"came_from": came_from, "depth": depth}
	came_from[start] = start
	depth[start] = 0
	var frontier: Array[Vector2i] = [start]
	while not frontier.is_empty():
		var current: Vector2i = frontier.pop_front()
		if int(depth[current]) >= budget:
			continue
		for step: Vector2i in PATH_DIRECTIONS:
			var next := current + step
			if came_from.has(next) or not _step_is_legal(current, step, walkable):
				continue
			came_from[next] = current
			depth[next] = int(depth[current]) + 1
			frontier.append(next)
	return {"came_from": came_from, "depth": depth}


static func _path_is_open(start: Vector2i, directions: Array[String], walkable: Dictionary) -> bool:
	if directions.is_empty():
		return false
	var current := start
	for direction: String in directions:
		var step: Vector2i = STEP_BY_DIRECTION.get(direction, Vector2i.ZERO)
		if not _step_is_legal(current, step, walkable):
			return false
		current += step
	return true


static func _step_is_legal(current: Vector2i, step: Vector2i, walkable: Dictionary) -> bool:
	if step == Vector2i.ZERO or not walkable.has(current + step):
		return false
	if step.x != 0 and step.y != 0:
		return (
			walkable.has(current + Vector2i(step.x, 0))
			and walkable.has(current + Vector2i(0, step.y))
		)
	return true


static func _route_directions(came_from: Dictionary, start: Vector2i, target: Vector2i) -> Array[String]:
	if not came_from.has(target):
		return []
	var path: Array[String] = []
	var current := target
	while current != start:
		var previous: Vector2i = came_from[current]
		path.push_front(_direction_for_step(current - previous))
		current = previous
	return path


static func _direct_path(start: Vector2i, target: Vector2i, budget: int) -> Array[String]:
	var path: Array[String] = []
	var current := start
	while current != target and path.size() < budget:
		var step := Vector2i(signi(target.x - current.x), signi(target.y - current.y))
		path.append(_direction_for_step(step))
		current += step
	return path if current == target else []


static func _direction_for_step(step: Vector2i) -> String:
	for direction: String in STEP_BY_DIRECTION:
		if STEP_BY_DIRECTION[direction] == step:
			return direction
	return ""


static func _coord(value: Variant) -> Vector2i:
	if value is Vector2i:
		return value as Vector2i
	if value is not Dictionary:
		return Vector2i.ZERO
	var position := value as Dictionary
	if position.get("position") is Dictionary:
		position = position["position"] as Dictionary
	return Vector2i(int(position.get("x", 0)), int(position.get("y", 0)))
