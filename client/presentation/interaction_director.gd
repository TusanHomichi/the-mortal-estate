class_name InteractionDirector
extends RefCounted

signal selection_changed(target: Dictionary)
## Tiles carry no repeat flag: the world shell's visible movement draft is the
## player's live intent, so a later activation on that same square confirms it
## without a double-activation deadline.
signal tile_activated(target: Dictionary, activation_msec: int)
signal ground_target_pinned(target: Dictionary)
signal intent_requested(intent: Dictionary, label: String)
signal unsafe_intent_requested(intent: Dictionary, label: String)
signal rejected(message: String)
signal drag_started(target: Dictionary)
signal drag_finished
signal spell_selection_changed(spell_id: String)
signal spell_prepare_started(spell_id: String)
signal spell_target_ready(spell_id: String)

const DRAG_THRESHOLD_PIXELS: float = 8.0
const CLOSE_MODES: Array[String] = ["fight", "poke"]
const RANGED_MODES: Array[String] = ["jumpkick", "throw", "shoot"]
const UNSUPPORTED_CLOSE_REASONS: Array[String] = ["physical_mode_not_supported", "right_hand_not_weapon"]
const DISTANCE_REASONS: Array[String] = ["not_engaged", "out_of_range"]
const TRANSIENT_CLOSE_REASONS: Array[String] = [
	"not_ready",
	"insufficient_stamina",
	"suppressed_by_status",
	"blocked_by_sight",
	"protected_target_requires_confirmation",
]
const INCOMPLETE_MESSAGE: String = "Authoritative actions incomplete for this frame"
const EXAMINATION_ONLY_MESSAGE: String = "Move onto this square to manipulate ground state"

var _frame: Dictionary = {}
var _generation: int = -1
var _input_enabled: bool = false
var _command_pending: bool = false
var _selected_target: Dictionary = {}
var _last_activation: Dictionary = {}
var _press: Dictionary = {}
var _drag: Dictionary = {}
var _ranged_mode: String = "jumpkick"
var _not_ready_buffer: Dictionary = {}
var _bow_chain: Dictionary = {}
var _selected_spell_id: String = ""
var _spell_prepare: Dictionary = {}
var profile: CombatFeelProfile


func _init(profile_value: CombatFeelProfile = null) -> void:
	profile = profile_value if profile_value != null else preload("res://presentation/combat_feel_profile.tres")


func present_frame(frame: Dictionary, generation: int) -> void:
	var prior_frame: Dictionary = _frame
	_frame = frame.duplicate(true)
	_generation = generation
	if frame.is_empty():
		_cancel_drag()
	_validate_selection()
	_validate_spell_selection()
	_advance_not_ready_buffer()
	_advance_bow_chain()
	_advance_spell_prepare(prior_frame)


func set_input_enabled(enabled: bool) -> void:
	_input_enabled = enabled
	if not enabled:
		cancel_local_state("Input disabled")


func set_command_pending(pending: bool) -> void:
	_command_pending = pending
	if pending:
		_last_activation.clear()
		_cancel_drag()


func reconsider_current_frame() -> void:
	_advance_not_ready_buffer()
	_advance_bow_chain()
	_advance_spell_prepare({})


func set_ranged_mode(mode: String) -> bool:
	if mode not in RANGED_MODES:
		return false
	if mode != _ranged_mode:
		_ranged_mode = mode
		_not_ready_buffer.clear()
		_bow_chain.clear()
	return true


func ranged_mode() -> String:
	return _ranged_mode


func selected_target() -> Dictionary:
	return _selected_target.duplicate(true)


func selected_actor_id() -> String:
	if _selected_target.get("kind") != "actor":
		return ""
	return str(_selected_target.get("source_identity", ""))


func selected_spell_id() -> String:
	return _selected_spell_id


func preparing_spell_id() -> String:
	return str(_spell_prepare.get("spell_id", ""))


func spell_target_is_ready() -> bool:
	return bool(_spell_prepare.get("target_ready", false))


func prepared_spell_row() -> Dictionary:
	return _spell_row(str(_spell_prepare.get("spell_id", "")))


func not_ready_buffer() -> Dictionary:
	return _not_ready_buffer.duplicate(true)


func bow_chain() -> Dictionary:
	return _bow_chain.duplicate(true)


func drag_state() -> Dictionary:
	return _drag.duplicate(true)


func activate_semantic_target(target: Dictionary, now_msec: int = -1) -> bool:
	if not _can_interact() or not _target_is_current(target):
		return false
	var now: int = Time.get_ticks_msec() if now_msec < 0 else now_msec
	var identity: String = str(target.get("identity", ""))
	var elapsed_since_activation: int = now - int(
		_last_activation.get("at_msec", -profile.double_activation_window_msec - 1)
	)
	var repeated: bool = (
		not _last_activation.is_empty()
		and _last_activation.get("identity") == identity
		and elapsed_since_activation >= 0
		and elapsed_since_activation <= profile.double_activation_window_msec
	)
	if not _selected_target.is_empty() and _selected_target.get("identity") != identity:
		_not_ready_buffer.clear()
		_bow_chain.clear()
	_selected_target = target.duplicate(true)
	selection_changed.emit(_selected_target.duplicate(true))
	if repeated:
		_last_activation.clear()
	else:
		_last_activation = {"identity": identity, "at_msec": now}
	match str(target.get("kind", "")):
		"actor":
			if repeated:
				_engage_actor(str(target.get("source_identity", "")))
		"ground_item":
			ground_target_pinned.emit(target.duplicate(true))
			if repeated:
				if _ground_target_manipulation_reachable(target):
					_submit_ground_item(str(target.get("source_identity", "")), "")
				else:
					rejected.emit(EXAMINATION_ONLY_MESSAGE)
		"corpse":
			ground_target_pinned.emit(target.duplicate(true))
			if repeated:
				if _ground_target_manipulation_reachable(target):
					_submit_corpse_search(str(target.get("source_identity", "")))
				else:
					rejected.emit(EXAMINATION_ONLY_MESSAGE)
		"gold_pile":
			ground_target_pinned.emit(target.duplicate(true))
			if repeated and not _ground_target_manipulation_reachable(target):
				rejected.emit(EXAMINATION_ONLY_MESSAGE)
		"tile":
			ground_target_pinned.emit(target.duplicate(true))
			tile_activated.emit(target.duplicate(true), now)
		_:
			return false
	return true


func begin_primary_press(target: Dictionary, display_position: Vector2, now_msec: int = -1) -> bool:
	if not activate_semantic_target(target, now_msec):
		return false
	if (
		target.get("kind") == "ground_item"
		and not _ground_target_manipulation_reachable(target)
	):
		_press.clear()
		return true
	_press = {
		"target": target.duplicate(true),
		"position": display_position,
		"generation": _generation,
	}
	return true


func update_pointer(display_position: Vector2) -> bool:
	if _press.is_empty() or not _drag.is_empty():
		return false
	var target: Dictionary = _press.get("target", {})
	if (
		target.get("kind") != "ground_item"
		or not _ground_target_manipulation_reachable(target)
	):
		return false
	if (display_position as Vector2).distance_to(_press["position"]) < DRAG_THRESHOLD_PIXELS:
		return false
	_drag = {
		"target": target.duplicate(true),
	}
	_last_activation.clear()
	drag_started.emit(target.duplicate(true))
	return true


func end_primary_press() -> void:
	if not _drag.is_empty():
		_cancel_drag()
	else:
		_press.clear()


func complete_item_drag(destination_position: String) -> bool:
	if _drag.is_empty():
		_cancel_drag()
		return false
	var target: Dictionary = _drag.get("target", {})
	if not _ground_target_manipulation_reachable(target):
		_cancel_drag()
		rejected.emit(EXAMINATION_ONLY_MESSAGE)
		return false
	var item_instance_id: String = str(target.get("source_identity", ""))
	var submitted: bool = _submit_ground_item(item_instance_id, destination_position)
	_cancel_drag()
	return submitted


func cancel_local_state(reason: String = "") -> void:
	_last_activation.clear()
	_press.clear()
	_cancel_drag()
	_not_ready_buffer.clear()
	_bow_chain.clear()
	_cancel_spell_prepare()
	if not _selected_target.is_empty():
		_selected_target.clear()
		selection_changed.emit({})
	if not _selected_spell_id.is_empty():
		_selected_spell_id = ""
		spell_selection_changed.emit("")
	if not reason.is_empty():
		rejected.emit(reason)


func note_unrelated_command() -> void:
	_not_ready_buffer.clear()
	_bow_chain.clear()


func note_command_result(intent_kind: String, accepted: bool) -> void:
	if intent_kind == "nock" and not _bow_chain.is_empty():
		if accepted:
			_bow_chain["nock_result_accepted"] = true
		else:
			_bow_chain.clear()
	if not accepted:
		_not_ready_buffer.clear()
		_cancel_spell_prepare()


func activate_spell(spell_id: String, now_msec: int = -1) -> bool:
	if not _can_interact():
		return false
	var row: Dictionary = _spell_row(spell_id)
	if row.is_empty():
		rejected.emit("Spell is not present in the current authoritative frame")
		return false
	var now: int = Time.get_ticks_msec() if now_msec < 0 else now_msec
	var elapsed_since_activation: int = now - int(
		_last_activation.get("at_msec", -profile.double_activation_window_msec - 1)
	)
	var repeated: bool = (
		_selected_spell_id == spell_id
		and _last_activation.get("identity") == "spell:" + spell_id
		and elapsed_since_activation >= 0
		and elapsed_since_activation <= profile.double_activation_window_msec
	)
	if not _selected_spell_id.is_empty() and _selected_spell_id != spell_id:
		_cancel_spell_prepare()
	_selected_spell_id = spell_id
	spell_selection_changed.emit(spell_id)
	if not repeated:
		_last_activation = {"identity": "spell:" + spell_id, "at_msec": now}
		return true
	_last_activation.clear()
	return _begin_spell_prepare(row)


func mark_spell_chant_complete() -> void:
	if _spell_prepare.is_empty():
		return
	_spell_prepare["chant_complete"] = true
	_maybe_open_spell_target()


func cancel_spell_selection() -> void:
	_selected_spell_id = ""
	_cancel_spell_prepare()
	spell_selection_changed.emit("")


func submit_prepared_spell_target(intent: Dictionary, label: String) -> bool:
	if _spell_prepare.is_empty() or not bool(_spell_prepare.get("target_ready", false)):
		return false
	var spell_id: String = str(_spell_prepare.get("spell_id", ""))
	if not _intent_matches_prepared_spell(intent, spell_id):
		rejected.emit("Prepared spell target no longer matches the authoritative row")
		return false
	_emit_exact_intent(intent, label)
	_cancel_spell_prepare()
	return true


func engagement_states(actor_id: String) -> Dictionary:
	var states: Dictionary = {}
	for mode: String in RANGED_MODES:
		var option: Dictionary = _physical_option(actor_id, mode)
		states[mode] = {
			"enabled": _option_enabled(option),
			"blocked_reason": option.get("blocked_reason", "no_exact_action"),
		}
	return states


func _engage_actor(actor_id: String) -> void:
	if not _actor_is_visible_and_present(actor_id):
		rejected.emit("Target is no longer visible and living")
		return
	if bool(_frame.get("action_options_truncated", false)):
		rejected.emit(INCOMPLETE_MESSAGE)
		return
	var plausible_close: Array[Dictionary] = []
	for mode: String in CLOSE_MODES:
		var option: Dictionary = _physical_option(actor_id, mode)
		if option.is_empty():
			continue
		var reason: String = str(option.get("blocked_reason", ""))
		if reason not in UNSUPPORTED_CLOSE_REASONS:
			plausible_close.append({"mode": mode, "option": option})
	if plausible_close.size() > 1:
		rejected.emit("Both Fight and Poke are plausible; choose an exact action from Context")
		return
	if plausible_close.size() == 1:
		var close: Dictionary = plausible_close[0]
		var close_option: Dictionary = close["option"]
		if _option_enabled(close_option):
			_emit_option(close_option)
			return
		var close_reason: String = str(close_option.get("blocked_reason", "server disabled"))
		if close_reason == "not_ready":
			_buffer_not_ready(actor_id, str(close["mode"]), close_option)
			return
		if close_reason not in DISTANCE_REASONS:
			rejected.emit(_words(close_reason))
			return
	var ranged_option: Dictionary = _physical_option(actor_id, _ranged_mode)
	if ranged_option.is_empty():
		rejected.emit("No exact %s action in this frame" % _ranged_mode)
		return
	if _option_enabled(ranged_option):
		_emit_option(ranged_option)
		return
	var ranged_reason: String = str(ranged_option.get("blocked_reason", "server disabled"))
	if ranged_reason == "not_ready":
		_buffer_not_ready(actor_id, _ranged_mode, ranged_option)
		return
	if _ranged_mode == "shoot" and ranged_reason == "bow_not_nocked":
		_begin_bow_chain(actor_id, ranged_option)
		return
	rejected.emit(_words(ranged_reason))


func _buffer_not_ready(actor_id: String, mode: String, option: Dictionary) -> void:
	var intent: Variant = option.get("intent")
	if intent is Dictionary and intent.get("authorization") == "confirmed_unsafe":
		rejected.emit("Unsafe attacks are never buffered")
		return
	_not_ready_buffer = {
		"option_id": str(option.get("id", "")),
		"actor_id": actor_id,
		"mode": mode,
		"captured_ready_at": str(_frame.get("ready_at", "")),
		"frame_generation": _generation,
		"preference": _ranged_mode,
		"safety": _actor_safety(actor_id),
	}
	rejected.emit("Queued for readiness")


func _advance_not_ready_buffer() -> void:
	if _not_ready_buffer.is_empty():
		return
	var actor_id: String = str(_not_ready_buffer.get("actor_id", ""))
	var mode: String = str(_not_ready_buffer.get("mode", ""))
	if not _input_enabled or _command_pending or not _actor_is_visible_and_present(actor_id):
		_not_ready_buffer.clear()
		return
	if str(_not_ready_buffer.get("preference", "")) != _ranged_mode:
		_not_ready_buffer.clear()
		return
	if _actor_safety(actor_id) != _not_ready_buffer.get("safety"):
		_not_ready_buffer.clear()
		return
	var option: Dictionary = _physical_option(actor_id, mode)
	if str(option.get("id", "")) != _not_ready_buffer.get("option_id"):
		_not_ready_buffer.clear()
		return
	if _option_enabled(option):
		_not_ready_buffer.clear()
		_emit_option(option)
		return
	if CanonicalDecimal.at_least(str(_frame.get("logical_time", "0")), str(_not_ready_buffer.get("captured_ready_at", ""))):
		var reason: String = str(option.get("blocked_reason", "no exact action"))
		_not_ready_buffer.clear()
		rejected.emit(_words(reason))


func _begin_bow_chain(actor_id: String, shoot_option: Dictionary) -> void:
	var nock: Dictionary = _option_by_id("nock")
	if not _option_enabled(nock):
		rejected.emit(_words(nock.get("blocked_reason", "No exact enabled Nock action")))
		return
	_bow_chain = {
		"actor_id": actor_id,
		"mode": "shoot",
		"shoot_option_id": str(shoot_option.get("id", "")),
		"preference": _ranged_mode,
		"safety": _actor_safety(actor_id),
		"nock_result_accepted": false,
	}
	_emit_option(nock)


func _advance_bow_chain() -> void:
	if _bow_chain.is_empty() or not bool(_bow_chain.get("nock_result_accepted", false)):
		return
	var actor_id: String = str(_bow_chain.get("actor_id", ""))
	if not _input_enabled or _ranged_mode != "shoot" or not _actor_is_visible_and_present(actor_id):
		_bow_chain.clear()
		return
	if _command_pending:
		return
	if _actor_safety(actor_id) != _bow_chain.get("safety"):
		_bow_chain.clear()
		return
	var shoot: Dictionary = _physical_option(actor_id, "shoot")
	if str(shoot.get("id", "")) != _bow_chain.get("shoot_option_id") or not _option_enabled(shoot):
		return
	_bow_chain.clear()
	_emit_option(shoot)


func _begin_spell_prepare(row: Dictionary) -> bool:
	var spell_id: String = str(row.get("spell_id", ""))
	var method: String = str(row.get("casting_method", ""))
	var state: Dictionary = row.get("cast", {}) if method == "direct" else row.get("warm", {})
	if not bool(state.get("enabled", false)):
		rejected.emit(_words(state.get("blocked_reason", "Spell unavailable")))
		return false
	_spell_prepare = {
		"spell_id": spell_id,
		"casting_method": method,
		"chant_complete": false,
		"target_ready": false,
	}
	spell_prepare_started.emit(spell_id)
	if method == "warm_then_cast":
		if state.get("intent") is not Dictionary:
			_cancel_spell_prepare()
			rejected.emit("Warm row has no exact intent")
			return false
		_emit_exact_intent(state["intent"], "Warm " + str(row.get("spell_name", spell_id)))
	elif method != "direct":
		_cancel_spell_prepare()
		rejected.emit("Unsupported projected casting method")
		return false
	return true


func _advance_spell_prepare(_prior_frame: Dictionary) -> void:
	if _spell_prepare.is_empty():
		return
	var row: Dictionary = _spell_row(str(_spell_prepare.get("spell_id", "")))
	if row.is_empty() or row.get("casting_method") != _spell_prepare.get("casting_method"):
		_cancel_spell_prepare()
		return
	if _spell_prepare.get("casting_method") == "direct" and not bool(row.get("cast", {}).get("enabled", false)):
		_cancel_spell_prepare()
		return
	if _spell_prepare.get("casting_method") == "warm_then_cast":
		var warmed: Variant = _frame.get("warmed_spell")
		if warmed is not Dictionary or warmed.get("spell_id") != _spell_prepare.get("spell_id"):
			return
		var cast: Dictionary = row.get("cast", {})
		if warmed.get("status") != "ready":
			return
		if not bool(cast.get("enabled", false)):
			_cancel_spell_prepare()
			return
	_maybe_open_spell_target()


func _maybe_open_spell_target() -> void:
	if _spell_prepare.is_empty() or not bool(_spell_prepare.get("chant_complete", false)):
		return
	var row: Dictionary = _spell_row(str(_spell_prepare.get("spell_id", "")))
	if row.is_empty():
		_cancel_spell_prepare()
		return
	if _spell_prepare.get("casting_method") == "warm_then_cast":
		var warmed: Variant = _frame.get("warmed_spell")
		if warmed is not Dictionary or warmed.get("spell_id") != _spell_prepare.get("spell_id") or warmed.get("status") != "ready":
			return
	var cast: Dictionary = row.get("cast", {})
	if not bool(cast.get("enabled", false)):
		return
	_spell_prepare["target_ready"] = true
	if not bool(cast.get("requires_target_selection", false)) and cast.get("intent") is Dictionary:
		_emit_exact_intent(cast["intent"], "Cast " + str(row.get("spell_name", row.get("spell_id", "spell"))))
		_cancel_spell_prepare()
		return
	spell_target_ready.emit(str(row.get("spell_id", "")))


func _submit_ground_item(item_instance_id: String, destination_position: String) -> bool:
	if _ground_actions_incomplete():
		rejected.emit(INCOMPLETE_MESSAGE)
		return false
	var matches: Array[Dictionary] = []
	for value: Variant in _frame.get("action_options", []):
		var option: Dictionary = value as Dictionary
		var intent: Variant = option.get("intent")
		if not _option_enabled(option) or intent is not Dictionary:
			continue
		if intent.get("kind") != "move_item" or intent.get("item_instance_id") != item_instance_id:
			continue
		var destination: Dictionary = intent.get("destination", {})
		if destination.get("kind") != "carried":
			continue
		var position: String = str(destination.get("position", ""))
		if not destination_position.is_empty() and destination_position != "sack" and position != destination_position:
			continue
		if (destination_position.is_empty() or destination_position == "sack") and not position.begins_with("sack_item_"):
			continue
		matches.append(option)
	if destination_position.is_empty() or destination_position == "sack":
		matches.sort_custom(func(left: Dictionary, right: Dictionary) -> bool:
			return _sack_index(left) < _sack_index(right)
		)
	if matches.size() != 1 and not (destination_position.is_empty() or destination_position == "sack"):
		rejected.emit("No exact action in this frame")
		return false
	if matches.is_empty():
		rejected.emit("No exact action in this frame")
		return false
	_emit_option(matches[0])
	return true


func _submit_corpse_search(corpse_id: String) -> bool:
	if _ground_actions_incomplete():
		rejected.emit(INCOMPLETE_MESSAGE)
		return false
	var option: Dictionary = _option_matching(func(intent: Dictionary) -> bool:
		return intent.get("kind") == "search_corpse" and intent.get("corpse_id") == corpse_id
	)
	if not _option_enabled(option):
		rejected.emit(_words(option.get("blocked_reason", "No exact action in this frame")))
		return false
	_emit_option(option)
	return true


func _emit_option(option: Dictionary) -> void:
	var intent: Variant = option.get("intent")
	if not _option_enabled(option) or intent is not Dictionary:
		rejected.emit(_words(option.get("blocked_reason", "No exact action in this frame")))
		return
	_emit_exact_intent(intent, str(option.get("label", "Action")))


func _emit_exact_intent(intent_value: Variant, label: String) -> void:
	if intent_value is not Dictionary:
		return
	var intent: Dictionary = (intent_value as Dictionary).duplicate(true)
	if intent.get("authorization") == "confirmed_unsafe":
		unsafe_intent_requested.emit(intent, label)
	else:
		intent_requested.emit(intent, label)


func _physical_option(actor_id: String, mode: String) -> Dictionary:
	return _option_by_id("physical_attack_%s_%s" % [mode, actor_id])


func _option_by_id(option_id: String) -> Dictionary:
	for value: Variant in _frame.get("action_options", []):
		var option: Dictionary = value as Dictionary
		if option.get("id") == option_id:
			return option.duplicate(true)
	return {}


func _option_matching(predicate: Callable) -> Dictionary:
	var matches: Array[Dictionary] = []
	for value: Variant in _frame.get("action_options", []):
		var option: Dictionary = value as Dictionary
		var intent: Variant = option.get("intent")
		if intent is Dictionary and predicate.call(intent):
			matches.append(option.duplicate(true))
	return matches[0] if matches.size() == 1 else {}


func _option_enabled(option: Dictionary) -> bool:
	return bool(option.get("enabled", false)) and option.get("intent") is Dictionary


func _spell_row(spell_id: String) -> Dictionary:
	for value: Variant in _frame.get("spell_actions", []):
		var row: Dictionary = value as Dictionary
		if row.get("spell_id") == spell_id:
			return row.duplicate(true)
	return {}


func _intent_matches_prepared_spell(intent: Dictionary, spell_id: String) -> bool:
	if _spell_prepare.get("casting_method") == "direct":
		return intent.get("kind") == "cast_spell" and intent.get("spell_id") == spell_id
	return intent.get("kind") == "cast_warmed_spell"


func _validate_selection() -> void:
	if _selected_target.is_empty():
		return
	var kind: String = str(_selected_target.get("kind", ""))
	var source_identity: String = str(_selected_target.get("source_identity", ""))
	var exists: bool = false
	match kind:
		"actor":
			exists = _actor_is_visible_and_present(source_identity)
		"ground_item":
			exists = _row_exists("ground_items", "item_instance_id", source_identity)
		"corpse":
			exists = _row_exists("corpses", "corpse_id", source_identity)
		"gold_pile":
			exists = _row_exists("gold_piles", "gold_pile_id", source_identity)
		"tile":
			exists = _tile_exists(_selected_target.get("coordinate", Vector2i.ZERO))
	if not exists:
		if _last_activation.get("identity") == _selected_target.get("identity"):
			_last_activation.clear()
		_selected_target.clear()
		_not_ready_buffer.clear()
		_bow_chain.clear()
		selection_changed.emit({})


func _validate_spell_selection() -> void:
	if _selected_spell_id.is_empty() or not _spell_row(_selected_spell_id).is_empty():
		return
	if _last_activation.get("identity") == "spell:" + _selected_spell_id:
		_last_activation.clear()
	_selected_spell_id = ""
	_cancel_spell_prepare()
	spell_selection_changed.emit("")


func _actor_is_visible_and_present(actor_id: String) -> bool:
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if actor.get("actor_id") != actor_id:
			continue
		return PresentationState.actor_is_present(actor)
	return false


func _actor_safety(actor_id: String) -> String:
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if actor.get("actor_id") == actor_id:
			return str(actor.get("attack_safety", "unknown"))
	return "unknown"


func _row_exists(collection: String, key: String, identity: String) -> bool:
	for value: Variant in _frame.get(collection, []):
		if (value as Dictionary).get(key) == identity:
			return true
	return false


func _tile_exists(coordinate: Variant) -> bool:
	var expected: Vector2i = _coord(coordinate)
	for value: Variant in _frame.get("tiles", []):
		if _coord((value as Dictionary).get("position", {})) == expected:
			return true
	return false


func _target_is_current(target: Dictionary) -> bool:
	var current: bool = (
		not str(target.get("identity", "")).is_empty()
		and int(target.get("generation", -1)) == _generation
	)
	if not current:
		return false
	if target.get("kind") in WorldTargets.GROUND_KINDS:
		return (
			target.get("examine_reachable") == true
			and target.get("ground_reach")
			in [
				WorldTargets.GROUND_REACH_EXAMINE,
				WorldTargets.GROUND_REACH_MANIPULATE,
			]
			and target.get("manipulation_reachable")
			== (
				target.get("ground_reach")
				== WorldTargets.GROUND_REACH_MANIPULATE
			)
		)
	return true


func _ground_target_manipulation_reachable(target: Dictionary) -> bool:
	return (
		target.get("ground_reach")
		== WorldTargets.GROUND_REACH_MANIPULATE
		and target.get("manipulation_reachable") == true
	)


func _can_interact() -> bool:
	return _input_enabled and not _command_pending


func _ground_actions_incomplete() -> bool:
	return bool(_frame.get("action_options_truncated", false)) or bool(_frame.get("ground_items_truncated", false))


func _sack_index(option: Dictionary) -> int:
	var position: String = str(option.get("intent", {}).get("destination", {}).get("position", ""))
	return int(position.trim_prefix("sack_item_")) if position.begins_with("sack_item_") else 2147483647


func _cancel_drag() -> void:
	var had_drag: bool = not _drag.is_empty()
	_drag.clear()
	_press.clear()
	if had_drag:
		drag_finished.emit()


func _cancel_spell_prepare() -> void:
	_spell_prepare.clear()


func _coord(value: Variant) -> Vector2i:
	return WorldTargets.coordinate(value)


func _words(value: Variant) -> String:
	return str(value).replace("_", " ").capitalize()
