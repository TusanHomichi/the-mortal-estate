class_name PulseClock
extends RefCounted

## The authoritative beat, made presentable — and never made authoritative here.
##
## Ruling D5 gives this world one gameplay pulse, and
## [url]../../docs/boundary-map.md[/url] §2.1 says where it lives: what a beat
## [i]means[/i] is `tme-rules`' logical round, and [i]when[/i] a beat is struck is
## `GAMEPLAY_PULSE` in `crates/tme-server/src/scheduler.rs` — "one value in one
## place". The same section says what a client may not do: no second gameplay
## clock, and [b]no presentation layer may infer readiness from elapsed
## seconds[/b]. It also says what a client [i]may[/i] do: presentation may remain
## fluid between authoritative beats.
##
## This class is exactly that permission, held to exactly that limit.
##
## [b]Every authoritative answer comes from the frame.[/b] [method is_ready] is
## the frame's `can_act` and nothing else. [method beats_until_ready] is the
## whole-round distance between the frame's own `logical_time` and its own
## `ready_at`, on the digits. Neither consults a clock. If frames stop arriving,
## both keep saying what the last frame said, because that is still the last
## thing the authority said.
##
## [b]Only the fill is local, and it is measured, not declared.[/b] The wire
## carries no cadence — there is no pulse field on the welcome or the frame —
## so this does not get to know 3.0 seconds, and deliberately does not restate
## it. It [i]observes[/i] the beat: the wall-clock interval between the arrival
## of round T and the arrival of round T+1 is the span, and the fill is how far
## into that span the current beat has got. That keeps the cadence a single
## value in the single place that owns it, and has the meter track the beat the
## server is actually striking rather than the beat it was promised.
##
## Three consequences follow, all deliberate:
##
## [br]- [b]Before a span is observed, there is no fill.[/b] The first beat of a
##   session reports `measured == false` and a fill of zero, and the meter says
##   so. A guessed first beat would be a cadence assertion by another name.
## [br]- [b]The fill never runs past full.[/b] It is clamped, never
##   extrapolated, so a stalled connection freezes the meter at the end of the
##   beat that did arrive instead of inventing beats that did not.
## [br]- [b]A beat that skips a round is not measured.[/b] Two rounds of logical
##   time in one interval means an update went unseen; timing that interval
##   would record a beat twice as long as the one being struck.

## Bounds on an interval this will accept as a beat. Wide on purpose: the point
## is to exclude nonsense — a duplicated arrival, a resumed process, a machine
## suspended mid-session — not to encode an expected cadence. A tighter window
## would be this file quietly holding an opinion about the pulse, which is the
## one thing it must not do.
const MINIMUM_SPAN_MSEC: int = 250
const MAXIMUM_SPAN_MSEC: int = 20000

## What [method beats_until_ready] will count to before it stops counting. A
## wait longer than this is presented as "more beats than this meter draws".
const MAXIMUM_COUNTED_BEATS: int = 99

var _logical_time: String = ""
var _ready_at: String = ""
var _can_act: bool = false
var _anchor_msec: int = -1
var _span_msec: int = 0
var _prepared: Dictionary = {}
var _clock_override_msec: int = -1


## Installs one authoritative frame and returns whether it advanced the beat.
##
## An empty frame is absence of authority, not a beat: it clears everything,
## because a client that lost its frame has nothing true left to say about the
## pulse. A frame whose logical time is missing, non-canonical, or [i]earlier[/i]
## than the one already held is refused outright — authority does not run
## backwards, and a meter that smoothed over that would be presenting a
## contradiction as a picture.
func note_frame(frame: Dictionary, now_msec: int = -1) -> bool:
	if frame.is_empty():
		clear()
		return false
	var logical_time: String = str(frame.get("logical_time", ""))
	var ready_at: String = str(frame.get("ready_at", ""))
	if not CanonicalDecimal.is_canonical(logical_time) or not CanonicalDecimal.is_canonical(ready_at):
		clear()
		return false
	if not _logical_time.is_empty() and CanonicalDecimal.less(logical_time, _logical_time):
		clear()
		return false

	var now: int = _now(now_msec)
	var advanced: bool = logical_time != _logical_time
	if advanced:
		var rounds: int = CanonicalDecimal.rounds_between(_logical_time, logical_time)
		_span_msec = _measured_span(rounds, now)
		_anchor_msec = now
	_logical_time = logical_time
	_ready_at = ready_at
	_can_act = bool(frame.get("can_act", false))
	_note_warmed_spell(frame)
	return advanced


## The intent the shell has installed and is waiting on. It is the local half of
## the preparation band and is labelled as such: the client knows it sent this,
## which is not the same as the world having accepted it.
func note_prepared_intent(intent: Dictionary) -> void:
	var kind: String = str(intent.get("kind", ""))
	if kind.is_empty():
		return
	if str(_prepared.get("kind", "")) == "warmed_spell":
		return
	_prepared = {
		"kind": "command",
		"label": kind.replace("_", " "),
		"beats_remaining": 1,
		"status": "sent",
		"authoritative": false,
	}


func clear_prepared_intent() -> void:
	if str(_prepared.get("kind", "")) == "command":
		_prepared.clear()


func clear() -> void:
	_logical_time = ""
	_ready_at = ""
	_can_act = false
	_anchor_msec = -1
	_span_msec = 0
	_prepared.clear()


## Test seam. Live play never sets this; the suites do, because a pulse asserted
## against `Time.get_ticks_msec()` would be a test of the host's scheduler.
func set_test_clock(now_msec: int) -> void:
	_clock_override_msec = now_msec


func has_authority() -> bool:
	return not _logical_time.is_empty()


func has_measured_span() -> bool:
	return _span_msec > 0


func span_msec() -> int:
	return _span_msec


func logical_time() -> String:
	return _logical_time


func ready_at() -> String:
	return _ready_at


## Whether the observer may act, straight from the frame. Elapsed time is not
## consulted and must never be: readiness arrives in the frame
## ([url]../../docs/boundary-map.md[/url] §2.1, "Never").
func is_ready() -> bool:
	return has_authority() and _can_act


## Whole beats between the frame's own clock and the frame's own readiness.
## Zero whenever the observer is ready, including the ordinary case of having
## been ready for several rounds already.
func beats_until_ready() -> int:
	if not has_authority():
		return 0
	return mini(MAXIMUM_COUNTED_BEATS, CanonicalDecimal.rounds_between(_logical_time, _ready_at))


## How far into the current beat presentation has got, in `0.0 .. 1.0`.
##
## Clamped at both ends and never extrapolated: when the next beat is late the
## fill sits at full and waits, which is the truthful picture of a client that
## has not been told anything since.
func beat_fill(now_msec: int = -1) -> float:
	if not has_measured_span() or _anchor_msec < 0:
		return 0.0
	var elapsed: int = _now(now_msec) - _anchor_msec
	if elapsed <= 0:
		return 0.0
	return clampf(float(elapsed) / float(_span_msec), 0.0, 1.0)


## The local millisecond the current beat is expected to end at, or -1 while no
## span has been observed. This is a presentation deadline — the latest moment a
## cue may still be shown inside the beat it belongs to — and never a gameplay
## one.
func beat_deadline_msec() -> int:
	if not has_measured_span() or _anchor_msec < 0:
		return -1
	return _anchor_msec + _span_msec


## Everything a presentation surface needs, built once per displayed frame so
## that the meter, the world view, and the feedback director all read one
## account of the same beat.
func state(now_msec: int = -1) -> Dictionary:
	return {
		"has_authority": has_authority(),
		"logical_time": _logical_time,
		"ready_at": _ready_at,
		"can_act": is_ready(),
		"beats_until_ready": beats_until_ready(),
		"measured": has_measured_span(),
		"span_msec": _span_msec,
		"fill": beat_fill(now_msec),
		"beat_deadline_msec": beat_deadline_msec(),
		"prepared": _prepared.duplicate(true),
	}


## The interval just observed, or zero when it cannot be trusted as one beat.
func _measured_span(rounds: int, now_msec: int) -> int:
	if rounds != 1 or _anchor_msec < 0:
		return 0
	var interval: int = now_msec - _anchor_msec
	if interval < MINIMUM_SPAN_MSEC or interval > MAXIMUM_SPAN_MSEC:
		return 0
	return interval


## The authoritative half of the preparation band: a spell the world says is
## warming, with the round it lands on. It outranks a locally installed intent,
## because it is the same preparation seen from the side that decides it.
func _note_warmed_spell(frame: Dictionary) -> void:
	var warmed: Variant = frame.get("warmed_spell")
	if warmed is not Dictionary:
		if str(_prepared.get("kind", "")) == "warmed_spell":
			_prepared.clear()
		return
	var row: Dictionary = warmed as Dictionary
	var spell_ready_at: String = str(row.get("ready_at", ""))
	_prepared = {
		"kind": "warmed_spell",
		"label": str(row.get("spell_id", "spell")).replace("_", " "),
		"beats_remaining": mini(
			MAXIMUM_COUNTED_BEATS, CanonicalDecimal.rounds_between(_logical_time, spell_ready_at)
		),
		"status": str(row.get("status", "warming")),
		"authoritative": true,
	}


func _now(value: int) -> int:
	if value >= 0:
		return value
	if _clock_override_msec >= 0:
		return _clock_override_msec
	return Time.get_ticks_msec()
