class_name ActionCooldown
extends RefCounted

## Presentation of the observer's own server deadline, in milliseconds.
## Elapsed local time animates the bar; only a server frame can grant readiness.
var _logical_time: String = ""
var _ready_at: String = ""
var _can_act: bool = false
var _anchor_msec: int = -1
var _duration_msec: int = 0
var _remaining_msec: int = 0
var _prepared: Dictionary = {}
var _clock_override_msec: int = -1

func note_frame(frame: Dictionary, now_msec: int = -1) -> bool:
	if frame.is_empty():
		clear()
		return false
	var logical: String = str(frame.get("logical_time", ""))
	var ready: String = str(frame.get("ready_at", ""))
	if not CanonicalDecimal.is_canonical(logical) or not CanonicalDecimal.is_canonical(ready):
		clear()
		return false
	if not _logical_time.is_empty() and CanonicalDecimal.less(logical, _logical_time):
		clear()
		return false
	var changed: bool = logical != _logical_time or ready != _ready_at
	if changed:
		_remaining_msec = CanonicalDecimal.bounded_difference(logical, ready)
		if ready != _ready_at or _duration_msec == 0:
			_duration_msec = _remaining_msec
		_anchor_msec = _now(now_msec)
	_logical_time = logical
	_ready_at = ready
	_can_act = bool(frame.get("can_act", false))
	_note_warmed_spell(frame)
	return changed

func note_prepared_intent(intent: Dictionary) -> void:
	if intent.is_empty() or str(_prepared.get("kind", "")) == "warmed_spell":
		return
	_prepared = {"kind": "command", "label": str(intent.get("kind", "action")).replace("_", " "), "remaining_msec": 0, "status": "sent", "authoritative": false}

func clear_prepared_intent() -> void:
	if str(_prepared.get("kind", "")) == "command":
		_prepared.clear()

func clear() -> void:
	_logical_time = ""
	_ready_at = ""
	_can_act = false
	_anchor_msec = -1
	_duration_msec = 0
	_remaining_msec = 0
	_prepared.clear()

func set_test_clock(now_msec: int) -> void:
	_clock_override_msec = now_msec

func has_authority() -> bool:
	return not _logical_time.is_empty()

func has_duration() -> bool:
	return _duration_msec > 0

func duration_msec() -> int:
	return _duration_msec

func logical_time() -> String:
	return _logical_time

func ready_at() -> String:
	return _ready_at

func is_ready() -> bool:
	return has_authority() and _can_act

func remaining_msec() -> int:
	return _remaining_msec

func cooldown_fill(now_msec: int = -1) -> float:
	if is_ready():
		return 1.0
	if not has_duration():
		return 0.0
	var elapsed: int = maxi(0, _now(now_msec) - _anchor_msec)
	return clampf(1.0 - float(maxi(0, _remaining_msec - elapsed)) / float(_duration_msec), 0.0, 1.0)

func cooldown_deadline_msec() -> int:
	return _anchor_msec + _remaining_msec if has_duration() and not is_ready() else -1

func state(now_msec: int = -1) -> Dictionary:
	return {"has_authority": has_authority(), "logical_time": _logical_time, "ready_at": _ready_at, "can_act": is_ready(), "remaining_msec": remaining_msec(), "known_duration": has_duration() or is_ready(), "duration_msec": _duration_msec, "fill": cooldown_fill(now_msec), "cooldown_deadline_msec": cooldown_deadline_msec(), "prepared": _prepared.duplicate(true)}

func _note_warmed_spell(frame: Dictionary) -> void:
	var warmed: Variant = frame.get("warmed_spell")
	if warmed is not Dictionary:
		if str(_prepared.get("kind", "")) == "warmed_spell":
			_prepared.clear()
		return
	var row: Dictionary = warmed as Dictionary
	_prepared = {"kind": "warmed_spell", "label": str(row.get("spell_id", "spell")).replace("_", " "), "remaining_msec": CanonicalDecimal.bounded_difference(_logical_time, str(row.get("ready_at", ""))), "status": str(row.get("status", "warming")), "authoritative": true}

func _now(value: int) -> int:
	if value >= 0:
		return value
	return _clock_override_msec if _clock_override_msec >= 0 else Time.get_ticks_msec()
