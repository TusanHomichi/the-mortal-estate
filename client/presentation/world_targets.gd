class_name WorldTargets
extends RefCounted

## The renderer-neutral half of world pointing.
##
## Everything here is derived from an authoritative frame alone: which things in
## the world can be addressed, what identity each one has, and how close the
## controlled character stands to the ground state under it. No screen space, no
## camera, no scene graph — a view decides where a target is drawn and how a
## pixel resolves to one, and asks this owner what the target *is*.
##
## The interaction director and the ground tray depend on this owner rather than
## on any view, which is what lets a view be replaced without touching either.

const KIND_PRIORITY: Dictionary = {
	"tile": 0,
	"corpse": 1,
	"ground_item": 2,
	"gold_pile": 2,
	"actor": 3,
}
const GROUND_KINDS: Array[String] = ["corpse", "ground_item", "gold_pile"]
const GROUND_REACH_EXAMINE: String = "examine"
const GROUND_REACH_MANIPULATE: String = "manipulate"

## Frame collections that produce addressable targets, in `collection`,
## `kind`, `identity key`, `location key` order.
const SOURCES: Array[Array] = [
	["tiles", "tile", "", "position"],
	["actors", "actor", "actor_id", "position"],
	["corpses", "corpse", "corpse_id", "location"],
	["ground_items", "ground_item", "item_instance_id", "location"],
	["gold_piles", "gold_pile", "gold_pile_id", "location"],
]


## Builds every addressable target in one frame, sorted by identity so two
## builds of the same frame are byte-identical.
##
## Ground state further than one square from the controlled character is not
## addressable at all and never enters the result; ground state within reach
## carries its reach classification, so callers never re-derive distance.
static func build(frame: Dictionary, generation: int) -> Array[Dictionary]:
	var player_coordinate_value: Variant = controlled_player_coordinate(frame)
	var targets: Array[Dictionary] = []
	var seen: Dictionary = {}
	for source: Array in SOURCES:
		var collection: String = source[0]
		var kind: String = source[1]
		var identity_key: String = source[2]
		var location_key: String = source[3]
		for value: Variant in frame.get(collection, []):
			if value is not Dictionary:
				continue
			var row: Dictionary = value as Dictionary
			var coordinate_value: Variant = coordinate_or_null(row.get(location_key))
			if coordinate_value == null:
				continue
			var coordinate: Vector2i = coordinate_value as Vector2i
			var source_identity: String = (
				tile_identity(coordinate)
				if identity_key.is_empty()
				else str(row.get(identity_key, ""))
			)
			if source_identity.is_empty():
				continue
			var identity: String = semantic_identity(kind, source_identity)
			if seen.has(identity):
				continue
			var target: Dictionary = {
				"identity": identity,
				"kind": kind,
				"source_identity": source_identity,
				"source": row.duplicate(true),
				"coordinate": coordinate,
				"generation": generation,
			}
			if kind in GROUND_KINDS:
				if player_coordinate_value == null:
					continue
				var ring_distance: int = ground_ring_distance(
					player_coordinate_value as Vector2i,
					coordinate,
				)
				if ring_distance > 1:
					continue
				target["ground_reach"] = (
					GROUND_REACH_MANIPULATE if ring_distance == 0 else GROUND_REACH_EXAMINE
				)
				target["examine_reachable"] = true
				target["manipulation_reachable"] = ring_distance == 0
			seen[identity] = true
			targets.append(target)
	targets.sort_custom(func(left: Dictionary, right: Dictionary) -> bool:
		return str(left["identity"]) < str(right["identity"])
	)
	return targets


## Returns the one tile target at a coordinate, or an empty dictionary.
static func tile_at(targets: Array[Dictionary], coordinate: Vector2i) -> Dictionary:
	for target: Dictionary in targets:
		if target.get("kind") == "tile" and target.get("coordinate") == coordinate:
			return target.duplicate(true)
	return {}


## Returns the highest-priority target at a coordinate, or an empty dictionary.
## Priority is the same order a view draws in: an actor standing on a corpse on
## a tile resolves to the actor.
static func topmost_at(targets: Array[Dictionary], coordinate: Vector2i) -> Dictionary:
	var best: Dictionary = {}
	for target: Dictionary in targets:
		if target.get("coordinate") != coordinate:
			continue
		if best.is_empty() or int(KIND_PRIORITY[target["kind"]]) > int(KIND_PRIORITY[best["kind"]]):
			best = target
	return best.duplicate(true)


static func semantic_identity(kind: String, source_identity: String) -> String:
	if kind == "tile" and source_identity.begins_with("tile:"):
		return source_identity
	return "%s:%s" % [kind, source_identity]


static func tile_identity(coordinate: Vector2i) -> String:
	return "tile:%d:%d" % [coordinate.x, coordinate.y]


static func controlled_player_coordinate(frame: Dictionary) -> Variant:
	var observer_actor_id: String = str(frame.get("observer_actor_id", ""))
	if observer_actor_id.is_empty():
		return null
	for actor_value: Variant in frame.get("actors", []):
		if actor_value is not Dictionary:
			continue
		var actor: Dictionary = actor_value as Dictionary
		if str(actor.get("actor_id", "")) == observer_actor_id:
			return coordinate_or_null(actor.get("position"))
	return null


static func observation_center(frame: Dictionary) -> Vector2i:
	var coordinate_value: Variant = coordinate_or_null(frame.get("observation_center"))
	return Vector2i.ZERO if coordinate_value == null else coordinate_value as Vector2i


static func ground_ring_distance(
	player_coordinate: Vector2i,
	target_coordinate: Vector2i,
) -> int:
	return maxi(
		absi(target_coordinate.x - player_coordinate.x),
		absi(target_coordinate.y - player_coordinate.y),
	)


## Coerces a wire position, a nested position, or a vector into a square, or
## returns null. Non-integral and non-finite inputs are refused rather than
## rounded into a square the server never named.
static func coordinate_or_null(value: Variant) -> Variant:
	if value is Vector2i:
		return value
	if value is Vector2:
		var vector: Vector2 = value as Vector2
		if (
			not is_finite(vector.x)
			or not is_finite(vector.y)
			or not is_equal_approx(vector.x, roundf(vector.x))
			or not is_equal_approx(vector.y, roundf(vector.y))
		):
			return null
		return Vector2i(roundi(vector.x), roundi(vector.y))
	if value is not Dictionary:
		return null
	var position: Dictionary = value as Dictionary
	if position.get("position") is Dictionary:
		position = position["position"] as Dictionary
	var x_value: Variant = position.get("x")
	var y_value: Variant = position.get("y")
	if not _finite_integer(x_value) or not _finite_integer(y_value):
		return null
	return Vector2i(roundi(float(x_value)), roundi(float(y_value)))


## The lenient form, for presentation rows that have already been accepted by
## the codec and only need a square to compare against.
static func coordinate(value: Variant) -> Vector2i:
	var coordinate_value: Variant = coordinate_or_null(value)
	return Vector2i.ZERO if coordinate_value == null else coordinate_value as Vector2i


static func _finite_integer(value: Variant) -> bool:
	return (
		(value is int or value is float)
		and is_finite(float(value))
		and is_equal_approx(float(value), roundf(float(value)))
	)
