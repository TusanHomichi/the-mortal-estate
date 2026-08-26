class_name CombatFeelDirector
extends RefCounted

signal deferred_feedback_ready(entry: Dictionary)
signal visual_flash_requested(kind: String)
signal spell_chant_complete

var profile: CombatFeelProfile
var audio_player: AudioCuePlayer
var _active: Dictionary = {}
var _scheduled: Array[Dictionary] = []
var _held_feedback: Array[Dictionary] = []
var _seen_identities: Dictionary = {}
var _clock_override_msec: int = -1

## The local millisecond the beat now elapsing is expected to end at, as
## [PulseClock] measured it, or -1 while no beat has been observed.
##
## The profile's payoff windows are the *minimum* time a cue may be held for, so
## that a result never lands before the swing that caused it. They were also,
## until this was here, the *only* thing bounding it — which left an unstated
## assumption that a beat is comfortably longer than a payoff window. This makes
## the bound explicit and derives it from the pulse instead: a cue belongs to the
## beat its action resolved on, so it is shown inside that beat or at the end of
## it, never spilled into the next one. Where no beat has been observed the
## profile's windows stand alone, exactly as before.
var _beat_deadline_msec: int = -1


func _init(
	profile_value: CombatFeelProfile = null,
	audio_player_value: AudioCuePlayer = null,
) -> void:
	profile = profile_value if profile_value != null else preload("res://presentation/combat_feel_profile.tres")
	audio_player = audio_player_value


func begin_spell_prepare(spell_id: String, now_msec: int = -1) -> void:
	var now: int = _now(now_msec)
	_active = {
		"family": "spell",
		"spell_id": spell_id,
		"started_at": now,
		"payoff_at": _inside_beat(now + profile.spell_minimum_payoff_msec, now),
	}
	_play("spell_chant", "prepare:" + spell_id)
	_scheduled.append({"at": _inside_beat(now + profile.spell_chant_msec, now), "kind": "chant_complete"})


func set_test_clock(now_msec: int) -> void:
	_clock_override_msec = now_msec


func set_beat_deadline(deadline_msec: int) -> void:
	_beat_deadline_msec = deadline_msec


## A moment brought inside the beat it belongs to. Never brought earlier than
## `now`: a beat that has already ended does not make a cue retroactive, it
## makes it due immediately.
func _inside_beat(target_msec: int, now_msec: int) -> int:
	if _beat_deadline_msec < 0:
		return target_msec
	return maxi(now_msec, mini(target_msec, _beat_deadline_msec))


func note_command_installed(intent: Dictionary, now_msec: int = -1) -> void:
	var now: int = _now(now_msec)
	var kind: String = str(intent.get("kind", ""))
	if kind == "physical_attack":
		var mode: String = str(intent.get("mode", ""))
		var ranged: bool = mode in ["jumpkick", "throw", "shoot"]
		_active = {
			"family": "physical",
			"mode": mode,
			"target_actor_id": str(intent.get("target_actor_id", "")),
			"started_at": now,
			"payoff_at": _inside_beat(now + (profile.ranged_minimum_payoff_msec if ranged else profile.melee_minimum_payoff_msec), now),
		}
		_scheduled.append({
			"at": _inside_beat(now + (profile.ranged_release_msec if ranged else profile.melee_wind_up_msec), now),
			"kind": "release",
			"role": "bow_release" if mode == "shoot" else "combat_swing",
			"identity": "release:%s:%s" % [mode, intent.get("target_actor_id", "")],
		})
	elif kind in ["cast_spell", "cast_warmed_spell"]:
		_active = {
			"family": "spell",
			"spell_id": str(intent.get("spell_id", _active.get("spell_id", ""))),
			"started_at": now,
			"payoff_at": _inside_beat(now + profile.spell_minimum_payoff_msec, now),
		}
		_scheduled.append({
			"at": _inside_beat(now + profile.spell_release_msec, now),
			"kind": "release",
			"role": "spell_release",
			"identity": "spell-release:" + str(_active.get("spell_id", "")),
		})
	elif kind == "move_item":
		_active = {
			"family": "loot",
			"item_instance_id": str(intent.get("item_instance_id", "")),
			"destination": (intent.get("destination", {}) as Dictionary).duplicate(true),
			"result_accepted": false,
			"result_identity": "",
			"started_at": now,
		}


func note_command_result(intent_kind: String, accepted: bool, identity: String = "") -> void:
	if accepted:
		if intent_kind == "move_item" and _active.get("family") == "loot":
			_active["result_accepted"] = true
			_active["result_identity"] = identity
		return
	_scheduled.clear()
	_held_feedback.clear()
	_play("ui_reject", identity)
	_active.clear()


func note_authoritative_frame(frame: Dictionary) -> void:
	if _active.get("family") != "loot" or not bool(_active.get("result_accepted", false)):
		return
	var expected_item: String = str(_active.get("item_instance_id", ""))
	var expected_destination: Dictionary = _active.get("destination", {})
	if expected_item.is_empty() or expected_destination.get("kind") != "carried":
		_active.clear()
		return
	for value: Variant in frame.get("carried", {}).get("items", []):
		var row: Dictionary = value as Dictionary
		var item: Dictionary = row.get("item", {})
		if item.get("item_instance_id") == expected_item and row.get("position") == expected_destination.get("position"):
			_play("loot_stow", str(_active.get("result_identity", "")))
			_active.clear()
			return


func gate_feedback_entry(entry: Dictionary, cue: Dictionary, event_identity: String) -> bool:
	if not event_identity.is_empty() and _seen_identities.has(event_identity):
		return false
	if not event_identity.is_empty():
		_seen_identities[event_identity] = true
	var role: String = _role_for_cue(cue)
	var correlated: bool = _cue_correlates(cue)
	var now: int = _now(-1)
	var payoff_at: int = _inside_beat(int(_active.get("payoff_at", 0)), now)
	if correlated and now < payoff_at:
		_held_feedback.append({
			"at": payoff_at,
			"entry": entry.duplicate(true),
			"role": role,
			"identity": event_identity,
		})
		return false
	_present_payoff(entry, role, event_identity)
	return true


func advance(now_msec: int = -1) -> void:
	var now: int = _now(now_msec)
	var remaining_scheduled: Array[Dictionary] = []
	for scheduled: Dictionary in _scheduled:
		if int(scheduled.get("at", 0)) > now:
			remaining_scheduled.append(scheduled)
			continue
		if scheduled.get("kind") == "chant_complete":
			spell_chant_complete.emit()
		elif scheduled.get("kind") == "release":
			_play(str(scheduled.get("role", "")), str(scheduled.get("identity", "")))
	_scheduled = remaining_scheduled
	var remaining_feedback: Array[Dictionary] = []
	for held: Dictionary in _held_feedback:
		if int(held.get("at", 0)) > now:
			remaining_feedback.append(held)
			continue
		_play(str(held.get("role", "")), str(held.get("identity", "")))
		visual_flash_requested.emit(str(held.get("entry", {}).get("kind", "payoff")))
		deferred_feedback_ready.emit((held["entry"] as Dictionary).duplicate(true))
	_held_feedback = remaining_feedback
	if not _active.is_empty() and now >= _inside_beat(int(_active.get("started_at", now)) + profile.visual_tail_cap_msec, now):
		_active.clear()


func discard() -> void:
	_beat_deadline_msec = -1
	_active.clear()
	_scheduled.clear()
	_held_feedback.clear()
	_seen_identities.clear()
	if audio_player != null:
		audio_player.discard()


func held_feedback_count() -> int:
	return _held_feedback.size()


func _present_payoff(entry: Dictionary, role: String, identity: String) -> void:
	_play(role, identity)
	if not role.is_empty():
		visual_flash_requested.emit(str(entry.get("kind", "payoff")))


func _cue_correlates(cue: Dictionary) -> bool:
	if _active.get("family") == "physical" and cue.get("kind") in ["physical_combat", "weapon_fumbled", "defeat"]:
		if cue.get("kind") == "physical_combat":
			return cue.get("mode") == _active.get("mode") and cue.get("target", {}).get("actor_id") == _active.get("target_actor_id")
		return true
	if _active.get("family") == "spell" and cue.get("kind") in ["spell_impact", "spell_lifecycle", "defeat"]:
		return cue.get("kind") == "defeat" or cue.get("spell_id") == _active.get("spell_id")
	return false


func _role_for_cue(cue: Dictionary) -> String:
	match str(cue.get("kind", "")):
		"physical_combat":
			return "combat_body_impact" if cue.get("outcome", {}).get("kind") == "hit" else "combat_dry_result"
		"weapon_fumbled":
			return "combat_dry_result"
		"spell_impact":
			return "spell_impact"
	return ""


func _play(role: String, identity: String) -> void:
	if audio_player != null and not role.is_empty():
		audio_player.play_role(role, identity)


func _now(value: int) -> int:
	if value >= 0:
		return value
	if _clock_override_msec >= 0:
		return _clock_override_msec
	return Time.get_ticks_msec()
