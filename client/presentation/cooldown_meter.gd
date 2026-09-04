class_name CooldownMeter
extends Control

## Displays the controlled character's own action cooldown and spell preparation.
const DEFAULT_MINIMUM_SIZE: Vector2 = Vector2(60.0, 18.0)

const SEGMENT_GAP: int = 3
const MINIMUM_SEGMENT_WIDTH: int = 4
const BORDER_WIDTH: float = 1.0
const READY_BORDER_WIDTH: float = 2.0

## The most beats drawn as segments. A longer wait keeps counting in words and
## stops adding rectangles, so the meter stays readable rather than becoming a
## bar chart of a hundred slivers.
const MAXIMUM_SEGMENTS: int = 8

## The preparation band's height, in pixels rather than as a fraction: a band
## drawn as a share of a short control comes out too thin to read at exactly the
## sizes the HUD uses. The beat row takes whatever is left.
const BAND_HEIGHT: float = 6.0

const TRACK: Color = Color(0.10, 0.11, 0.13)
const BORDER: Color = Color(0.62, 0.66, 0.74, 0.85)
const BEAT_FILL: Color = Color(0.55, 0.80, 1.0, 0.90)
const READY_FILL: Color = Color(0.35, 0.95, 0.55, 0.90)
const PREPARED_FILL: Color = Color(1.0, 0.78, 0.35, 0.90)
const UNMEASURED_HATCH: Color = Color(0.72, 0.74, 0.80, 0.45)

var _state: Dictionary = {}
var _described: String = ""


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	focus_mode = Control.FOCUS_NONE
	if custom_minimum_size == Vector2.ZERO:
		custom_minimum_size = DEFAULT_MINIMUM_SIZE
	_refresh()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


## Installs one account of the beat. An empty state is absence of authority.
func present_cooldown(state: Dictionary) -> void:
	_state = state.duplicate(true)
	_refresh()


func clear() -> void:
	_state.clear()
	_refresh()


# -- what the tests and the accessibility description read --------------------


## The segments the meter is showing, in draw order. Each is the beat it stands
## for and how far that beat has filled: `0.0` for a beat still entirely ahead,
## a fraction for the beat now elapsing, and `1.0` only for a beat the authority
## has already closed.
func segments() -> Array[Dictionary]:
	if not bool(_state.get("has_authority", false)):
		return []
	var drawn: int = 1
	var fill: float = float(_state.get("fill", 0.0)) if bool(_state.get("known_duration", false)) else 0.0
	var rows: Array[Dictionary] = []
	for index: int in drawn:
		rows.append({
			"kind": "ready" if int(_state.get("remaining_msec", 0)) == 0 else "beat",
			"fill": fill if index == 0 else 0.0,
			"known_duration": bool(_state.get("known_duration", false)),
		})
	return rows


## The beat in words. The HUD's readiness line is this string, so the meter and
## the sentence beside it can never disagree about what is being shown.
func meter_text() -> String:
	if not bool(_state.get("has_authority", false)):
		return "◇ No authoritative frame"
	var line: String = "◆ Ready" if bool(_state.get("can_act", false)) else "◇ Action cooldown"
	var prepared: Dictionary = _state.get("prepared", {})
	if not prepared.is_empty():
		line += " · preparing %s" % str(prepared.get("label", "action"))
	return line


func prepared_band() -> Dictionary:
	return (_state.get("prepared", {}) as Dictionary).duplicate(true)


## Where the two rows are drawn. The band is only present while something is
## prepared, and it never takes the beat row's legibility with it: the beat is
## what this control exists to show, so the band is given a fixed height and the
## beat keeps the rest.
func layout_rows() -> Dictionary:
	var band: Dictionary = prepared_band()
	var rows: Dictionary = {"beat": _beat_area(not band.is_empty())}
	if not band.is_empty():
		rows["band"] = _band_area()
	return rows


# -- drawing ------------------------------------------------------------------


func _draw() -> void:
	var rows: Array[Dictionary] = segments()
	var band: Dictionary = prepared_band()
	var beat_area: Rect2 = _beat_area(not band.is_empty())
	if rows.is_empty():
		_draw_track(beat_area)
		return

	var width: float = (beat_area.size.x - float(SEGMENT_GAP * (rows.size() - 1))) / float(rows.size())
	if width < float(MINIMUM_SEGMENT_WIDTH):
		width = float(MINIMUM_SEGMENT_WIDTH)
	for index: int in rows.size():
		var row: Dictionary = rows[index]
		var rect: Rect2 = Rect2(
			Vector2(beat_area.position.x + float(index) * (width + float(SEGMENT_GAP)), beat_area.position.y),
			Vector2(width, beat_area.size.y),
		)
		if rect.position.x + rect.size.x > beat_area.position.x + beat_area.size.x:
			break
		_draw_segment(rect, row)
	if not band.is_empty():
		_draw_band(_band_area(), band)


func _draw_segment(rect: Rect2, row: Dictionary) -> void:
	_draw_track(rect)
	var ready: bool = str(row.get("kind", "beat")) == "ready"
	if not bool(row.get("known_duration", false)):
		# No span observed yet, so there is no fill this control is entitled to
		# draw. Hatching says "unknown" in shape rather than leaving an empty
		# rectangle that reads as "nothing is happening".
		_draw_hatch(rect)
	else:
		var fill: float = clampf(float(row.get("fill", 0.0)), 0.0, 1.0)
		if fill > 0.0:
			draw_rect(
				Rect2(rect.position, Vector2(rect.size.x * fill, rect.size.y)),
				READY_FILL if ready else BEAT_FILL,
				true,
			)
	draw_rect(rect, BORDER, false, READY_BORDER_WIDTH if ready else BORDER_WIDTH)


func _draw_band(rect: Rect2, band: Dictionary) -> void:
	if rect.size.y <= 0.0:
		return
	_draw_track(rect)
	var remaining: int = maxi(0, int(band.get("remaining_msec", 0)))
	# The band lands on a beat: it shows what is left of the wait, so a spell
	# one beat from ready shows one segment's worth and empties onto the beat.
	var portion: float = 0.0 if remaining == 0 else 1.0
	draw_rect(Rect2(rect.position, Vector2(rect.size.x * portion, rect.size.y)), PREPARED_FILL, true)
	if not bool(band.get("authoritative", false)):
		# A locally installed intent is the client's own claim, not the world's.
		# It gets the same band with a hatch over it so the two are never read
		# as the same kind of fact.
		_draw_hatch(rect)
	draw_rect(rect, BORDER, false, BORDER_WIDTH)


func _draw_track(rect: Rect2) -> void:
	draw_rect(rect, TRACK, true)


func _draw_hatch(rect: Rect2) -> void:
	var step: float = 6.0
	var x: float = rect.position.x
	while x < rect.position.x + rect.size.x:
		var top: Vector2 = Vector2(minf(x + rect.size.y, rect.position.x + rect.size.x), rect.position.y)
		draw_line(Vector2(x, rect.position.y + rect.size.y), top, UNMEASURED_HATCH, 1.0)
		x += step


func _beat_area(with_band: bool) -> Rect2:
	if not with_band:
		return Rect2(Vector2.ZERO, size)
	return Rect2(
		Vector2.ZERO,
		Vector2(size.x, maxf(1.0, size.y - BAND_HEIGHT - float(SEGMENT_GAP))),
	)


func _band_area() -> Rect2:
	var height: float = minf(BAND_HEIGHT, size.y)
	return Rect2(Vector2(0.0, size.y - height), Vector2(size.x, height))


## Called every drawn frame, so it does the cheap thing: the accessibility
## description and the tooltip are only rewritten when the sentence they hold
## has actually changed, and the redraw is queued unconditionally because the
## fill has moved even when the words have not.
func _refresh() -> void:
	var description: String = meter_text()
	if description != _described:
		_described = description
		tooltip_text = description
		accessibility_description = description
	queue_redraw()
