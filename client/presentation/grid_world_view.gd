class_name GridWorldView
extends WorldViewSeam

## The world drawn as a square lattice of flat colours.
##
## This is **not the game's renderer and not art direction.** It is the cheapest
## honest presenter that satisfies the seam: one rectangle per visible square,
## one marker per addressable thing standing on one, the reach grid and the
## movement draft outlined over the top, and a banner that says so. The pixel
## renderer substitutes for it behind [WorldViewSeam] later, and nothing outside
## this file learns that it happened.
##
## Its targeting is real, and that is the whole reason it exists. Every drawn
## thing is a [WorldTargets] target occupying an exact screen rectangle, so
## [method semantic_target_for_display_position] answers by reading the same
## rectangles the frame was drawn from — never by guessing at the nearest
## anchor. That exactness is what lets [CaptureEmitter] write an identity
## sidecar and a per-pixel identity raster that match the picture by
## construction rather than by agreement.
##
## Two consequences follow and both are deliberate:
##
## [br]- **Only addressable things are drawn as markers.** Ground state further
##   than one square from the controlled character is not a [WorldTargets]
##   target at all, so drawing a marker for it would invite the owner to point
##   at something that resolves to the square underneath. It is counted in the
##   status line instead.
## [br]- **Colour carries no authority.** Hues are spread evenly across the
##   distinct terrain ids present in the current frame so that neighbouring
##   classes stay tellable apart. A class can therefore change colour between
##   frames. Identity comes from the sidecar and the target list; never from a
##   pixel's colour.

## The lattice never draws squares smaller than this, so a marker is always at
## least a few pixels of real estate rather than a rounding artefact.
const MINIMUM_PITCH: int = 8

## Bands reserved at the top and bottom of the control for the banner, the
## status line, and the movement-draft line. The lattice is laid out inside
## what is left, so a label never covers an addressable rectangle.
const HEADER_HEIGHT: int = 44
const FOOTER_HEIGHT: int = 28

## Where an occupant marker sits inside its square, as a fraction of the pitch.
## Occupants share the band side by side rather than stacking, so no two
## addressable rectangles ever overlap and the identity raster needs no
## tie-break rule.
const OCCUPANT_BAND_TOP: float = 0.22
const OCCUPANT_BAND_HEIGHT: float = 0.56
const OCCUPANT_INSET: float = 0.08

const GRID_STATE_ON: String = "on"
const GRID_STATE_OFF: String = "off"

## The furthest a step is animated rather than snapped. A square or two is a
## step; anything wider is a transition, a resynchronisation, or an actor that
## left and came back, and sliding a marker across the picture would present it
## as a walk it never took.
const MAXIMUM_ANIMATED_SQUARES: int = 2

const LAYER_SQUARES: String = "squares"
const LAYER_OCCUPANTS: String = "occupants"

const BACKDROP: Color = Color(0.055, 0.06, 0.075)
const NO_AUTHORITY: Color = Color(0.10, 0.105, 0.125)
const GRID_LINE: Color = Color(1.0, 1.0, 1.0, 0.07)
const IMPASSABLE_HATCH: Color = Color(0.0, 0.0, 0.0, 0.45)
const REACH_RING: Color = Color(0.55, 0.80, 1.0, 0.75)
const DRAFT_LINE: Color = Color(1.0, 0.78, 0.35, 0.95)
const PREVIEW_LINE: Color = Color(0.55, 1.0, 0.65, 0.95)
const CONTROLLED_OUTLINE: Color = Color(1.0, 1.0, 1.0, 0.95)
const FEEDBACK_FLASH: Color = Color(1.0, 0.86, 0.62, 0.35)

## One flat colour per addressable kind. Gold and loose items are deliberately
## far apart in hue: they are the two the owner most often has to tell apart.
const OCCUPANT_COLOURS: Dictionary = {
	"actor": Color(0.90, 0.35, 0.30),
	"corpse": Color(0.72, 0.70, 0.62),
	"ground_item": Color(0.40, 0.78, 0.95),
	"gold_pile": Color(0.98, 0.84, 0.18),
}
const CONTROLLED_COLOUR: Color = Color(0.35, 0.95, 0.55)

@onready var banner_label: Label = %WorldBanner
@onready var status_label: Label = %WorldStatus
@onready var interaction_label: Label = %WorldInteraction

var _frame: Dictionary = {}
var _frame_generation: int = -1
var _targets: Array[Dictionary] = []
var _screen_targets: Array[Dictionary] = []
var _layout: Dictionary = {}
var _palette: Dictionary = {}
var _interaction_text: String = ""
var _presented_feedback: Array[String] = []
var _flash_kinds: Array[String] = []
var _grid_preference: String = GRID_STATE_ON
var _grid_transient_active: bool = false
var _draft: Dictionary = {}
var _previous_squares: Dictionary = {}
var _previous_center: Vector2i = Vector2i.ZERO
var _previous_generation: int = -1
var _step_squares: Dictionary = {}
var _motion: float = 0.0
var _motion_quantum: int = 0


func _ready() -> void:
	_refresh()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_rebuild_screen_targets()
		queue_redraw()


# -- the seam ----------------------------------------------------------------


func present_frame(frame: Dictionary, frame_generation: int) -> void:
	_frame = frame.duplicate(true)
	_frame_generation = frame_generation
	_targets = WorldTargets.build(_frame, frame_generation)
	_palette = _build_palette(_frame)
	_flash_kinds.clear()
	if frame.is_empty():
		_interaction_text = ""
		_draft.clear()
	_note_step(frame_generation)
	_rebuild_screen_targets()
	_refresh()


## Installs the action interval this view is drawing inside of, as [ActionCooldown] accounts for
## it. Motion is the action interval run backwards: a fill of nothing means the step has not
## started and the picture still shows where everything was, and a full action interval
## means the step has landed. Nothing here decides when a action interval ends — the fill
## does, and the fill comes from the observer's reported deadline.
func present_cooldown(state: Dictionary) -> void:
	var motion: float = 0.0
	if _stepping() and bool(state.get("known_duration", false)):
		motion = clampf(1.0 - float(state.get("fill", 0.0)), 0.0, 1.0)
	# Rebuilt only when the step has actually moved a whole pixel. A fill
	# advances every drawn frame and the lattice does not, so quantising here is
	# what keeps a action interval's worth of motion from being a action interval's worth of layout.
	var quantum: int = int(round(motion * float(_layout.get("pitch", 0))))
	if is_equal_approx(motion, _motion) or (quantum == _motion_quantum and not _screen_targets.is_empty()):
		_motion = motion
		return
	_motion = motion
	_motion_quantum = quantum
	_rebuild_screen_targets()
	queue_redraw()


func clear() -> void:
	_frame.clear()
	_frame_generation = -1
	_targets.clear()
	_screen_targets.clear()
	_layout.clear()
	_palette.clear()
	_interaction_text = ""
	_draft.clear()
	_presented_feedback.clear()
	_flash_kinds.clear()
	_forget_step()
	_refresh()


func observation_center() -> Vector2i:
	return WorldTargets.observation_center(_frame)


func semantic_target_for_coordinate(coordinate: Vector2i) -> Dictionary:
	return WorldTargets.tile_at(_targets, coordinate)


## The target under a pointer position, read off the drawn rectangles.
##
## The list is walked from the last drawn to the first, so the answer is the
## topmost thing actually under the pointer — the same rule the identity raster
## encodes, because the raster is filled in that same order.
func semantic_target_for_display_position(display_position: Vector2) -> Dictionary:
	for index: int in range(_screen_targets.size() - 1, -1, -1):
		var record: Dictionary = _screen_targets[index]
		if (record["rect"] as Rect2i).has_point(Vector2i(floori(display_position.x), floori(display_position.y))):
			return (record["target"] as Dictionary).duplicate(true)
	return {}


func pointer_surface() -> Control:
	return self


func show_pending(start: Vector2i, path: Array[String]) -> void:
	_draft = {"kind": "pending", "start": start, "path": path.duplicate()}
	_interaction_text = "Movement draft from %d,%d · %s · awaiting authoritative preview" % [
		start.x,
		start.y,
		" → ".join(path),
	]
	_refresh()


func show_preview(preview: Dictionary) -> void:
	_draft = {"kind": "preview", "preview": preview.duplicate(true)}
	_interaction_text = "Authoritative preview · %s pace · %s of %d steps accepted · %s" % [
		str(preview.get("pace", "path")),
		str(preview.get("accepted_steps", "0")),
		(preview.get("requested_path", []) as Array).size(),
		str(preview.get("stop_reason", "full_path_accepted")).replace("_", " "),
	]
	_refresh()


func clear_interaction() -> void:
	_draft.clear()
	_interaction_text = ""
	_refresh()


func set_reach_grid_transient_active(active: bool) -> void:
	if _grid_transient_active == active:
		return
	_grid_transient_active = active
	queue_redraw()


func toggle_grid() -> void:
	_grid_preference = GRID_STATE_OFF if _grid_preference == GRID_STATE_ON else GRID_STATE_ON
	queue_redraw()


func grid_control_state() -> String:
	return _grid_preference


## Flashes the controlled character's square and reports what was presented.
## The shell records the receipt without knowing how the cue was drawn.
func present_feedback(kind: String) -> Dictionary:
	if kind.is_empty():
		return {}
	_presented_feedback.append(kind)
	_flash_kinds.append(kind)
	queue_redraw()
	return {"presented": "lattice_flash", "kind": kind}


# -- what a capture and the tests read ---------------------------------------


func targets() -> Array[Dictionary]:
	return _targets.duplicate(true)


## The authoritative frame currently presented, and the generation it arrived
## with. A capture records both so that what it shows can be checked against
## what the server said.
func frame() -> Dictionary:
	return _frame.duplicate(true)


func frame_generation() -> int:
	return _frame_generation


## Every drawn addressable thing with the exact rectangle it occupies, in draw
## order. This is the presenter's own semantic-target list in the sense spec
## §5.2 means: identity, kind, coordinate, presentation layer, screen anchor,
## and screen hit shape, all in this control's local space.
func screen_targets() -> Array[Dictionary]:
	var copy: Array[Dictionary] = []
	for record: Dictionary in _screen_targets:
		copy.append({
			"index": record["index"],
			"target": (record["target"] as Dictionary).duplicate(true),
			"rect": record["rect"],
			"anchor": record["anchor"],
			"layer": record["layer"],
			"controlled": record["controlled"],
		})
	return copy


## The lattice's placement: the pitch, the origin square (0,0) would be drawn
## at, and the frame's square bounds. A consumer converts between squares and
## pixels with these three facts and nothing else.
##
## The lattice itself never moves within a action interval. Only the markers standing on it
## do, and each one's displacement is carried on its own screen rectangle rather
## than on the camera, so this placement stays exactly as true mid-step as it is
## between steps — which is what a capture's sidecar depends on.
func layout() -> Dictionary:
	return _layout.duplicate(true)


## The steps this view is currently animating, as identity to squares-remaining.
## Empty between action intervals, and empty for a frame that snapped rather than stepped.
func animated_steps() -> Dictionary:
	if not _stepping():
		return {}
	var rows: Dictionary = {}
	for identity: String in _step_squares:
		rows[identity] = _step_squares[identity]
	return rows


## How much of the current step is still to be presented, `1.0` at the start of
## a action interval and `0.0` once it has landed.
func step_motion() -> float:
	return _motion


func presented_feedback() -> Array[String]:
	return _presented_feedback.duplicate()


func reach_grid_visible() -> bool:
	return _grid_preference == GRID_STATE_ON or _grid_transient_active


func palette() -> Dictionary:
	return _palette.duplicate()


func status_text() -> String:
	if _frame.is_empty():
		return "No authoritative frame. Nothing is being presented."
	var center: Vector2i = observation_center()
	var beyond: int = _ground_rows_beyond_reach()
	var line: String = (
		"Observation centre %d,%d · frame generation %d · logical time %s · %d squares · %d actors"
		% [
			center.x,
			center.y,
			_frame_generation,
			str(_frame.get("logical_time", "?")),
			(_frame.get("tiles", []) as Array).size(),
			(_frame.get("actors", []) as Array).size(),
		]
	)
	line += " · " + _controlled_line()
	if beyond > 0:
		line += " · %d ground row(s) beyond reach, not addressable" % beyond
	return line


# -- drawing -----------------------------------------------------------------


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), BACKDROP, true)
	if _layout.is_empty():
		draw_rect(_lattice_area(), NO_AUTHORITY, true)
		return

	var pitch: int = int(_layout["pitch"])
	for record: Dictionary in _screen_targets:
		if str(record["layer"]) != LAYER_SQUARES:
			continue
		var rect: Rect2i = record["rect"]
		var row: Dictionary = (record["target"] as Dictionary)["source"]
		draw_rect(Rect2(rect), _terrain_colour(row), true)
		if not bool(row.get("passable", true)):
			_draw_impassable(rect)

	if pitch >= 12:
		_draw_grid_lines(pitch)
	if reach_grid_visible():
		_draw_reach_ring(pitch)
	_draw_draft(pitch)

	for record: Dictionary in _screen_targets:
		if str(record["layer"]) != LAYER_OCCUPANTS:
			continue
		var target: Dictionary = record["target"]
		var rect: Rect2i = record["rect"]
		var controlled: bool = bool(record["controlled"])
		draw_rect(
			Rect2(rect),
			CONTROLLED_COLOUR if controlled else OCCUPANT_COLOURS.get(target["kind"], Color.MAGENTA),
			true,
		)
		if controlled:
			draw_rect(Rect2(rect), CONTROLLED_OUTLINE, false, 2.0)

	if not _flash_kinds.is_empty():
		# The flash persists until the next authoritative frame rather than for
		# one redraw: a capture taken after a cue must show what the cue showed,
		# and a view that cleared itself on the next repaint would not.
		var square: Variant = WorldTargets.controlled_player_coordinate(_frame)
		if square != null:
			draw_rect(Rect2(_square_rect(square as Vector2i)), FEEDBACK_FLASH, true)


func _draw_impassable(rect: Rect2i) -> void:
	var origin: Vector2 = Vector2(rect.position)
	var extent: Vector2 = Vector2(rect.size)
	draw_line(origin + Vector2(2, 2), origin + extent - Vector2(2, 2), IMPASSABLE_HATCH, 1.0)
	draw_line(
		origin + Vector2(extent.x - 2, 2), origin + Vector2(2, extent.y - 2), IMPASSABLE_HATCH, 1.0
	)


func _draw_grid_lines(pitch: int) -> void:
	var bounds: Dictionary = _layout["bounds"]
	var left: int = int(bounds["min_x"])
	var top: int = int(bounds["min_y"])
	var columns: int = int(bounds["columns"])
	var rows: int = int(bounds["rows"])
	var start: Vector2i = _square_origin(Vector2i(left, top))
	for column: int in columns + 1:
		var x: float = float(start.x + column * pitch) + 0.5
		draw_line(Vector2(x, start.y), Vector2(x, start.y + rows * pitch), GRID_LINE, 1.0)
	for row: int in rows + 1:
		var y: float = float(start.y + row * pitch) + 0.5
		draw_line(Vector2(start.x, y), Vector2(start.x + columns * pitch, y), GRID_LINE, 1.0)


func _draw_reach_ring(_pitch: int) -> void:
	var square: Variant = WorldTargets.controlled_player_coordinate(_frame)
	if square == null:
		return
	var centre: Vector2i = square as Vector2i
	for dy: int in [-1, 0, 1]:
		for dx: int in [-1, 0, 1]:
			if dx == 0 and dy == 0:
				continue
			var neighbour: Vector2i = centre + Vector2i(dx, dy)
			if not _square_is_in_frame(neighbour):
				continue
			draw_rect(Rect2(_square_rect(neighbour)), REACH_RING, false, 1.0)


func _draw_draft(_pitch: int) -> void:
	if _draft.is_empty():
		return
	if _draft.get("kind") == "pending":
		var start: Vector2i = _draft["start"]
		_outline_path(start, _draft["path"], DRAFT_LINE)
		return
	var preview: Dictionary = _draft.get("preview", {})
	var steps: Variant = preview.get("steps")
	if steps is Array and not (steps as Array).is_empty():
		# The authoritative account of where each step landed. `attempted` is the
		# square the step reached for, which is the square worth outlining even
		# when the outcome was refused: it is what the owner asked for.
		for value: Variant in steps as Array:
			if value is not Dictionary:
				continue
			var square: Variant = WorldTargets.coordinate_or_null((value as Dictionary).get("attempted"))
			if square == null or not _square_is_in_frame(square as Vector2i):
				continue
			draw_rect(Rect2(_square_rect(square as Vector2i)), PREVIEW_LINE, false, 2.0)
		return
	_outline_path(
		observation_center(), preview.get("requested_path", []) as Array, PREVIEW_LINE
	)


func _outline_path(start: Vector2i, path: Array, colour: Color) -> void:
	var current: Vector2i = start
	for value: Variant in path:
		current += ClientReachability.STEP_BY_DIRECTION.get(str(value), Vector2i.ZERO)
		if not _square_is_in_frame(current):
			continue
		draw_rect(Rect2(_square_rect(current)), colour, false, 2.0)


# -- the step across the action interval -------------------------------------------------


func _stepping() -> bool:
	return not _step_squares.is_empty()


## Works out, from the frame just installed and the one before it, how far each
## marker has to travel and whether travelling is the honest way to show it.
##
## The lattice is screen-stable: it is laid out around the observation centre, so
## when the controlled character takes a step the *world* is what shifts through
## the picture. A marker's displacement is therefore two facts added together —
## how far the observer moved, and how far that marker moved — which is why the
## controlled character comes out at zero and stays put while everything else
## slides past it. That is the same step, seen from inside it.
##
## Four conditions all have to hold, and any one of them failing makes the frame
## a snap rather than a step:
##
## [br]- the frame is the immediate successor of the one before it, so the two
##   pictures are a action interval apart rather than either side of a discard;
## [br]- the marker was in the previous frame, so there is a place to come from;
## [br]- the displacement is a step and not a transition
##   ([constant MAXIMUM_ANIMATED_SQUARES]);
## [br]- the marker's whole rectangle stays inside the lattice for the whole
##   travel, checked in [method _rebuild_screen_targets], so nothing is ever
##   drawn over the banner or outside the picture a capture claims to be exact.
func _note_step(frame_generation: int) -> void:
	var current_center: Vector2i = WorldTargets.observation_center(_frame)
	var current_squares: Dictionary = {}
	for target: Dictionary in _targets:
		if str(target["kind"]) != "tile":
			current_squares[str(target["identity"])] = target["coordinate"]

	_step_squares.clear()
	_motion = 0.0
	_motion_quantum = 0
	if _previous_generation >= 0 and frame_generation == _previous_generation + 1:
		var observer_step: Vector2i = current_center - _previous_center
		for identity: String in current_squares:
			var previous: Variant = _previous_squares.get(identity)
			if previous == null:
				continue
			var travel: Vector2i = observer_step + ((previous as Vector2i) - (current_squares[identity] as Vector2i))
			if travel == Vector2i.ZERO or not _is_a_step(travel):
				continue
			_step_squares[identity] = travel

	# The step is recorded, not started. Motion only exists once a reported action interval
	# has been presented through [method present_cooldown], so a view nobody hands a
	# action interval to draws every frame exactly where authority put it.
	_previous_squares = current_squares
	_previous_center = current_center
	_previous_generation = frame_generation
	if _frame.is_empty():
		_forget_step()


func _forget_step() -> void:
	_previous_squares.clear()
	_previous_center = Vector2i.ZERO
	_previous_generation = -1
	_step_squares.clear()
	_motion = 0.0
	_motion_quantum = 0


func _is_a_step(travel: Vector2i) -> bool:
	return (
		absi(travel.x) <= MAXIMUM_ANIMATED_SQUARES and absi(travel.y) <= MAXIMUM_ANIMATED_SQUARES
	)


## Whether a marker travelling this far stays inside the lattice for the whole
## action interval. Only the two extremes need checking: the displacement is linear, so a
## rectangle inside the area at both ends of the travel is inside it throughout.
func _travel_stays_inside(landing: Rect2i, travel: Vector2i) -> bool:
	var area: Rect2i = Rect2i(_lattice_area())
	if not area.encloses(landing):
		return false
	var pitch: int = int(_layout["pitch"])
	var start: Rect2i = Rect2i(landing.position + travel * pitch, landing.size)
	return area.encloses(start)


## The pixels a marker with this much travel left to do is displaced by.
func _motion_offset(travel: Vector2i) -> Vector2i:
	if travel == Vector2i.ZERO or is_zero_approx(_motion) or _layout.is_empty():
		return Vector2i.ZERO
	var pitch: float = float(_layout["pitch"]) * _motion
	return Vector2i(int(round(float(travel.x) * pitch)), int(round(float(travel.y) * pitch)))


# -- layout ------------------------------------------------------------------


func _lattice_area() -> Rect2:
	return Rect2(
		Vector2(0.0, float(HEADER_HEIGHT)),
		Vector2(size.x, maxf(0.0, size.y - float(HEADER_HEIGHT + FOOTER_HEIGHT))),
	)


func _rebuild_screen_targets() -> void:
	_screen_targets.clear()
	_layout.clear()
	if _targets.is_empty():
		return
	var bounds: Dictionary = _square_bounds()
	if bounds.is_empty():
		return
	var area: Rect2 = _lattice_area()
	var columns: int = int(bounds["columns"])
	var rows: int = int(bounds["rows"])
	var pitch: int = maxi(
		MINIMUM_PITCH,
		mini(int(area.size.x) / maxi(1, columns), int(area.size.y) / maxi(1, rows)),
	)
	var origin: Vector2i = Vector2i(
		int(area.position.x) + (int(area.size.x) - columns * pitch) / 2 - int(bounds["min_x"]) * pitch,
		int(area.position.y) + (int(area.size.y) - rows * pitch) / 2 - int(bounds["min_y"]) * pitch,
	)
	_layout = {
		"pitch": pitch,
		"origin": origin,
		"bounds": bounds,
		"viewport": Vector2i(int(size.x), int(size.y)),
	}

	var ordered: Array[Dictionary] = _targets.duplicate()
	ordered.sort_custom(func(left: Dictionary, right: Dictionary) -> bool:
		var left_priority: int = int(WorldTargets.KIND_PRIORITY[left["kind"]])
		var right_priority: int = int(WorldTargets.KIND_PRIORITY[right["kind"]])
		if left_priority != right_priority:
			return left_priority < right_priority
		return str(left["identity"]) < str(right["identity"])
	)

	var occupants_by_square: Dictionary = {}
	for target: Dictionary in ordered:
		if str(target["kind"]) == "tile":
			continue
		var square: Vector2i = target["coordinate"]
		if not occupants_by_square.has(square):
			occupants_by_square[square] = 0
		occupants_by_square[square] = int(occupants_by_square[square]) + 1

	var observer: String = str(_frame.get("observer_actor_id", ""))
	var placed: Dictionary = {}
	for target: Dictionary in ordered:
		var square: Vector2i = target["coordinate"]
		var kind: String = str(target["kind"])
		if kind == "tile":
			_append_screen_target(target, _square_rect(square), LAYER_SQUARES, false)
			continue
		var total: int = int(occupants_by_square[square])
		var slot: int = int(placed.get(square, 0))
		placed[square] = slot + 1
		var controlled: bool = kind == "actor" and str(target["source_identity"]) == observer
		var rect: Rect2i = _occupant_rect(square, slot, total)
		var travel: Variant = _step_squares.get(str(target["identity"]))
		if travel != null:
			var displaced: Rect2i = Rect2i(rect.position + _motion_offset(travel as Vector2i), rect.size)
			# The whole travel is checked, not just where the marker is right
			# now: a step that would leave the lattice at any point in the action interval
			# is presented as a snap for the whole action interval rather than sliding out
			# of the picture partway through it.
			if _travel_stays_inside(rect, travel as Vector2i):
				rect = displaced
			else:
				_step_squares.erase(str(target["identity"]))
		_append_screen_target(target, rect, LAYER_OCCUPANTS, controlled)


## A target's anchor is a pixel that target actually owns, never merely a point
## inside its rectangle. A square is overlapped by whatever stands on it, so its
## anchor sits in the strip above the occupant band; an occupant owns its whole
## rectangle, so its anchor is the middle of it. That keeps one invariant true
## by construction: every anchor resolves to the target it belongs to.
func _append_screen_target(
	target: Dictionary, rect: Rect2i, layer: String, controlled: bool
) -> void:
	var anchor: Vector2i = (
		Vector2i(
			rect.position.x + rect.size.x / 2,
			rect.position.y + maxi(1, int(round(float(rect.size.y) * OCCUPANT_BAND_TOP * 0.5))),
		)
		if layer == LAYER_SQUARES
		else rect.position + rect.size / 2
	)
	_screen_targets.append({
		"index": _screen_targets.size() + 1,
		"target": target,
		"rect": rect,
		"anchor": anchor,
		"layer": layer,
		"controlled": controlled,
	})


func _square_bounds() -> Dictionary:
	var squares: Array[Vector2i] = []
	for target: Dictionary in _targets:
		if str(target["kind"]) == "tile":
			squares.append(target["coordinate"])
	if squares.is_empty():
		return {}
	var min_x: int = squares[0].x
	var max_x: int = squares[0].x
	var min_y: int = squares[0].y
	var max_y: int = squares[0].y
	for square: Vector2i in squares:
		min_x = mini(min_x, square.x)
		max_x = maxi(max_x, square.x)
		min_y = mini(min_y, square.y)
		max_y = maxi(max_y, square.y)
	return {
		"min_x": min_x,
		"min_y": min_y,
		"max_x": max_x,
		"max_y": max_y,
		"columns": max_x - min_x + 1,
		"rows": max_y - min_y + 1,
	}


func _square_origin(square: Vector2i) -> Vector2i:
	var origin: Vector2i = _layout["origin"]
	var pitch: int = int(_layout["pitch"])
	return Vector2i(origin.x + square.x * pitch, origin.y + square.y * pitch)


func _square_rect(square: Vector2i) -> Rect2i:
	var pitch: int = int(_layout["pitch"])
	return Rect2i(_square_origin(square), Vector2i(pitch, pitch))


func _occupant_rect(square: Vector2i, slot: int, total: int) -> Rect2i:
	var pitch: int = int(_layout["pitch"])
	var origin: Vector2i = _square_origin(square)
	var inset: int = maxi(1, int(round(float(pitch) * OCCUPANT_INSET)))
	var band_top: int = int(round(float(pitch) * OCCUPANT_BAND_TOP))
	var band_height: int = maxi(2, int(round(float(pitch) * OCCUPANT_BAND_HEIGHT)))
	var available: int = maxi(total, pitch - inset * 2)
	var width: int = maxi(1, available / total)
	return Rect2i(
		Vector2i(origin.x + inset + slot * width, origin.y + band_top),
		Vector2i(width, band_height),
	)


func _square_is_in_frame(square: Vector2i) -> bool:
	return not WorldTargets.tile_at(_targets, square).is_empty()


# -- colour ------------------------------------------------------------------


func _build_palette(frame: Dictionary) -> Dictionary:
	var names: Array[String] = []
	for value: Variant in frame.get("tiles", []):
		if value is not Dictionary:
			continue
		var name: String = str((value as Dictionary).get("terrain_id", ""))
		if not name.is_empty() and name not in names:
			names.append(name)
	names.sort()
	var palette: Dictionary = {}
	for index: int in names.size():
		palette[names[index]] = float(index) / float(maxi(1, names.size()))
	return palette


func _terrain_colour(row: Dictionary) -> Color:
	var hue: float = float(_palette.get(str(row.get("terrain_id", "")), 0.0))
	if bool(row.get("passable", true)):
		return Color.from_hsv(hue, 0.34, 0.74)
	return Color.from_hsv(hue, 0.46, 0.32)


# -- text --------------------------------------------------------------------


func _controlled_line() -> String:
	var square: Variant = WorldTargets.controlled_player_coordinate(_frame)
	if square == null:
		return "No controlled character is present in this frame."
	var coordinate: Vector2i = square as Vector2i
	var observer: String = str(_frame.get("observer_actor_id", ""))
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if str(actor.get("actor_id", "")) != observer:
			continue
		return "You: %s at %d,%d · %s · HP %s/%s" % [
			str(actor.get("name", observer)),
			coordinate.x,
			coordinate.y,
			str(actor.get("life_state", "unknown")).replace("_", " "),
			str(actor.get("hp", "?")),
			str(actor.get("max_hp", "?")),
		]
	return "You: %s at %d,%d" % [observer, coordinate.x, coordinate.y]


func _ground_rows_beyond_reach() -> int:
	var addressable: int = 0
	for target: Dictionary in _targets:
		if str(target["kind"]) in WorldTargets.GROUND_KINDS:
			addressable += 1
	var present: int = 0
	for collection: String in ["corpses", "ground_items", "gold_piles"]:
		present += (_frame.get(collection, []) as Array).size()
	return maxi(0, present - addressable)


func _refresh() -> void:
	if not is_node_ready():
		return
	status_label.text = status_text()
	interaction_label.text = (
		"Movement: no draft" if _interaction_text.is_empty() else _interaction_text
	)
	queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var button: InputEventMouseButton = event as InputEventMouseButton
		var target: Dictionary = semantic_target_for_display_position(button.position)
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				semantic_primary_pressed.emit(target, button.position)
			else:
				semantic_primary_released.emit(target, button.position)
			accept_event()
		elif button.button_index == MOUSE_BUTTON_RIGHT and button.pressed:
			semantic_secondary_pressed.emit(target, button.position)
			accept_event()
	elif event is InputEventMouseMotion:
		var motion: InputEventMouseMotion = event as InputEventMouseMotion
		semantic_pointer_moved.emit(
			semantic_target_for_display_position(motion.position), motion.position
		)
