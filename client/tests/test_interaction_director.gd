extends RefCounted

var _support: TestSupport

func test_same_identity_window_is_inclusive_first_activation_is_immediate_and_drag_starts_at_eight_pixels() -> void:
	var director: InteractionDirector = InteractionDirector.new()
	director.set_input_enabled(true)
	director.present_frame(_frame([]), 3)
	var selections: Array[String] = []
	var pins: Array[String] = []
	var drags: Array[String] = []
	director.selection_changed.connect(func(target: Dictionary) -> void: selections.append(str(target.get("identity", ""))))
	director.ground_target_pinned.connect(func(target: Dictionary) -> void: pins.append(str(target.get("identity", ""))))
	director.drag_started.connect(func(target: Dictionary) -> void: drags.append(str(target.get("identity", ""))))
	var item: Dictionary = _semantic_target("ground_item", "item:1", 3)
	_support.expect(director.begin_primary_press(item, Vector2(10, 10), 1000), "first activation is accepted immediately")
	_support.expect_equal(selections, ["ground_item:item:1"], "first activation selects without waiting")
	_support.expect_equal(pins, ["ground_item:item:1"], "ground focus pins on first activation")
	_support.expect(not director.update_pointer(Vector2(17.99, 10)), "motion below eight pixels remains an activation")
	_support.expect(director.update_pointer(Vector2(18, 10)), "motion at eight pixels converts to drag")
	_support.expect_equal(drags, ["ground_item:item:1"], "drag carries stable item identity")
	_support.expect(director.not_ready_buffer().is_empty(), "drag creates no gameplay buffer")
	director.end_primary_press()
	_support.expect(director.drag_state().is_empty(), "release away from an exact destination cancels the drag")


func test_frame_generation_between_activations_preserves_attack_ground_and_spell_intent() -> void:
	var fight: Dictionary = _physical("fight", "enemy", true, null)
	var poke: Dictionary = _physical("poke", "enemy", false, "physical_mode_not_supported")
	var attack: InteractionDirector = _director([fight, poke])
	var attack_intents: Array[Dictionary] = []
	attack.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: attack_intents.append(intent.duplicate(true)))
	attack.activate_semantic_target(_semantic_target("actor", "enemy", 1), 1000)
	attack.present_frame(_frame([fight, poke]), 2)
	attack.activate_semantic_target(_semantic_target("actor", "enemy", 2), 1250)
	_support.expect_equal(attack_intents, [fight["intent"]], "an intervening frame does not erase a same-actor attack repeat")

	var item_option: Dictionary = {
		"id": "move_item:item:1",
		"label": "Stow item",
		"enabled": true,
		"blocked_reason": null,
		"intent": {"kind": "move_item", "item_instance_id": "item:1", "destination": {"kind": "carried", "position": "sack_item_1"}},
	}
	var ground: InteractionDirector = _director([item_option])
	var ground_intents: Array[Dictionary] = []
	ground.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: ground_intents.append(intent.duplicate(true)))
	ground.activate_semantic_target(_semantic_target("ground_item", "item:1", 1), 2000)
	ground.present_frame(_frame([item_option]), 2)
	ground.activate_semantic_target(_semantic_target("ground_item", "item:1", 2), 2250)
	_support.expect_equal(ground_intents, [item_option["intent"]], "an intervening frame does not erase a same-item ground repeat")

	var spell: InteractionDirector = InteractionDirector.new()
	spell.set_input_enabled(true)
	spell.present_frame(_frame([], [_spell("spark", "direct")]), 1)
	spell.activate_spell("spark", 3000)
	spell.present_frame(_frame([], [_spell("spark", "direct")]), 2)
	spell.activate_spell("spark", 3250)
	_support.expect_equal(spell.preparing_spell_id(), "spark", "an intervening frame does not erase a same-spell repeat")


func test_ground_reach_preserves_examination_and_refuses_adjacent_manipulation() -> void:
	var options: Array = [
		{
			"id": "move_item:item:1",
			"label": "Stow item",
			"enabled": true,
			"blocked_reason": null,
			"intent": {
				"kind": "move_item",
				"item_instance_id": "item:1",
				"destination": {"kind": "carried", "position": "sack_item_1"},
			},
		},
		{
			"id": "search_corpse:1",
			"label": "Search corpse",
			"enabled": true,
			"blocked_reason": null,
			"intent": {"kind": "search_corpse", "corpse_id": "corpse:1"},
		},
	]
	var director: InteractionDirector = _director(options)
	var intents: Array[Dictionary] = []
	var pins: Array[String] = []
	var rejections: Array[String] = []
	var drags: Array[String] = []
	director.intent_requested.connect(func(intent: Dictionary, _label: String) -> void:
		intents.append(intent.duplicate(true))
	)
	director.ground_target_pinned.connect(func(target: Dictionary) -> void:
		pins.append(str(target.get("identity", "")))
	)
	director.rejected.connect(func(message: String) -> void: rejections.append(message))
	director.drag_started.connect(func(target: Dictionary) -> void:
		drags.append(str(target.get("identity", "")))
	)

	var own_item: Dictionary = _semantic_target(
		"ground_item",
		"item:1",
		1,
		WorldTargets.GROUND_REACH_MANIPULATE,
	)
	director.activate_semantic_target(own_item, 1000)
	director.activate_semantic_target(own_item, 1300)
	var own_corpse: Dictionary = _semantic_target(
		"corpse",
		"corpse:1",
		1,
		WorldTargets.GROUND_REACH_MANIPULATE,
	)
	director.activate_semantic_target(own_corpse, 2000)
	director.activate_semantic_target(own_corpse, 2300)
	_support.expect_equal(
		intents.map(func(intent: Dictionary) -> String: return str(intent.get("kind", ""))),
		["move_item", "search_corpse"],
		"own-square repeated activation retains exact item and corpse intent routes",
	)

	var adjacent_item: Dictionary = _semantic_target(
		"ground_item",
		"item:adjacent",
		1,
		WorldTargets.GROUND_REACH_EXAMINE,
	)
	_support.expect(
		director.begin_primary_press(adjacent_item, Vector2(10, 10), 3000),
		"adjacent item first activation remains selectable for examination",
	)
	_support.expect(
		not director.update_pointer(Vector2(18, 10)) and director.drag_state().is_empty(),
		"adjacent item press never arms or begins drag",
	)
	for target: Dictionary in [
		adjacent_item,
		_semantic_target("corpse", "corpse:adjacent", 1, WorldTargets.GROUND_REACH_EXAMINE),
		_semantic_target("gold_pile", "gold:adjacent", 1, WorldTargets.GROUND_REACH_EXAMINE),
	]:
		director.activate_semantic_target(target, 4000)
		director.activate_semantic_target(target, 4300)
	_support.expect_equal(
		rejections,
		[
			InteractionDirector.EXAMINATION_ONLY_MESSAGE,
			InteractionDirector.EXAMINATION_ONLY_MESSAGE,
			InteractionDirector.EXAMINATION_ONLY_MESSAGE,
		],
		"adjacent item, corpse, and gold repeated activation refuse explicitly",
	)
	_support.expect_equal(
		intents.size(),
		2,
		"adjacent examination emits no manipulation intent",
	)
	_support.expect(drags.is_empty(), "adjacent examination emits no drag")
	_support.expect(
		pins.has("ground_item:item:adjacent")
		and pins.has("corpse:corpse:adjacent")
		and pins.has("gold_pile:gold:adjacent"),
		"all adjacent ground kinds still pin for examination",
	)


func test_close_mode_selection_distance_routing_transient_stop_and_dual_fail_closed() -> void:
	var fight: Dictionary = _physical("fight", "enemy", true, null)
	var poke_unsupported: Dictionary = _physical("poke", "enemy", false, "physical_mode_not_supported")
	var director: InteractionDirector = _director([fight, poke_unsupported])
	var intents: Array[Dictionary] = []
	var rejected: Array[String] = []
	director.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: intents.append(intent.duplicate(true)))
	director.rejected.connect(func(message: String) -> void: rejected.append(message))
	_activate_actor_twice(director, 1)
	_support.expect_equal(intents, [fight["intent"]], "sole supported enabled close mode copies its exact intent")

	var transient: InteractionDirector = _director([
		_physical("fight", "enemy", false, "insufficient_stamina"),
		poke_unsupported,
		_physical("throw", "enemy", true, null),
	])
	var transient_intents: Array[Dictionary] = []
	var transient_rejections: Array[String] = []
	transient.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: transient_intents.append(intent))
	transient.rejected.connect(func(message: String) -> void: transient_rejections.append(message))
	transient.set_ranged_mode("throw")
	_activate_actor_twice(transient, 1)
	_support.expect(transient_intents.is_empty() and "Insufficient Stamina" in transient_rejections, "transient close blocker never falls through to ranged")

	var distant: InteractionDirector = _director([
		_physical("fight", "enemy", false, "out_of_range"),
		poke_unsupported,
		_physical("throw", "enemy", true, null),
	])
	var distant_intents: Array[Dictionary] = []
	distant.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: distant_intents.append(intent))
	distant.set_ranged_mode("throw")
	_activate_actor_twice(distant, 1)
	_support.expect_equal(distant_intents[0]["mode"], "throw", "exact distance blocker routes only to the selected ranged mode")

	var dual: InteractionDirector = _director([
		_physical("fight", "enemy", true, null),
		_physical("poke", "enemy", false, "not_ready"),
	])
	var dual_rejections: Array[String] = []
	var dual_intents: Array[Dictionary] = []
	dual.rejected.connect(func(message: String) -> void: dual_rejections.append(message))
	dual.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: dual_intents.append(intent))
	_activate_actor_twice(dual, 1)
	_support.expect(dual_intents.is_empty() and dual_rejections.any(func(message: String) -> bool: return "Both Fight and Poke" in message), "dual plausible close modes fail closed")


func test_not_ready_buffer_and_bow_chain_require_fresh_exact_frames() -> void:
	var waiting: InteractionDirector = _director([
		_physical("fight", "enemy", false, "not_ready"),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
	])
	var waiting_intents: Array[Dictionary] = []
	waiting.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: waiting_intents.append(intent.duplicate(true)))
	_activate_actor_twice(waiting, 1)
	_support.expect_equal(waiting.not_ready_buffer()["captured_ready_at"], "11", "buffer captures the exact authoritative readiness boundary")
	var ready_frame: Dictionary = _frame([
		_physical("fight", "enemy", true, null),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
	])
	ready_frame["logical_time"] = "11"
	waiting.present_frame(ready_frame, 2)
	_support.expect_equal(waiting_intents.size(), 1, "fresh complete frame releases exactly one buffered intent")
	_support.expect(waiting.not_ready_buffer().is_empty(), "released buffer is discarded")

	var shoot_wait: Dictionary = _physical("shoot", "enemy", false, "bow_not_nocked")
	var nock: Dictionary = {"id": "nock", "label": "Nock bow", "enabled": true, "blocked_reason": null, "intent": {"kind": "nock"}}
	var bow: InteractionDirector = _director([
		_physical("fight", "enemy", false, "out_of_range"),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
		shoot_wait,
		nock,
	])
	bow.set_ranged_mode("shoot")
	var bow_intents: Array[Dictionary] = []
	bow.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: bow_intents.append(intent.duplicate(true)))
	_activate_actor_twice(bow, 1)
	_support.expect_equal(bow_intents, [{"kind": "nock"}], "unnocked Shoot submits only exact Nock first")
	bow.note_command_result("nock", true)
	var nocked_frame: Dictionary = _frame([
		_physical("fight", "enemy", false, "out_of_range"),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
		_physical("shoot", "enemy", true, null),
		nock,
	])
	bow.set_command_pending(true)
	bow.present_frame(nocked_frame, 2)
	_support.expect_equal(bow_intents.size(), 1, "replacement frame waits while the Nock result is still pending")
	_support.expect(not bow.bow_chain().is_empty(), "pending replacement does not discard the accepted Nock chain")
	bow.set_command_pending(false)
	bow.reconsider_current_frame()
	_support.expect_equal(bow_intents.size(), 2, "accepted Nock plus replacement frame submits Shoot once")
	_support.expect_equal(bow_intents[1]["mode"], "shoot", "second command is the fresh exact Shoot intent")


func test_direct_and_warm_then_cast_spells_separate_select_prepare_authority_and_target() -> void:
	var direct_row: Dictionary = _spell("spark", "direct")
	var direct: InteractionDirector = InteractionDirector.new()
	direct.set_input_enabled(true)
	direct.present_frame(_frame([], [direct_row]), 1)
	var direct_intents: Array[Dictionary] = []
	var prepared: Array[String] = []
	var target_ready: Array[String] = []
	direct.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: direct_intents.append(intent.duplicate(true)))
	direct.spell_prepare_started.connect(func(spell_id: String) -> void: prepared.append(spell_id))
	direct.spell_target_ready.connect(func(spell_id: String) -> void: target_ready.append(spell_id))
	_support.expect(direct.activate_spell("spark", 1000), "first spell activation selects")
	_support.expect(direct.activate_spell("spark", 1420), "same spell at 420ms enters Prepare")
	_support.expect(direct_intents.is_empty(), "direct Prepare sends no command")
	_support.expect_equal(prepared, ["spark"], "direct Prepare starts chant presentation")
	direct.mark_spell_chant_complete()
	_support.expect_equal(target_ready, ["spark"], "direct target opens only after chant")
	var cast: Dictionary = {"kind": "cast_spell", "spell_id": "spark", "target": {"kind": "actor", "actor_id": "enemy"}, "authorization": "safe"}
	_support.expect(direct.submit_prepared_spell_target(cast, "Cast Spark"), "existing exact target construction submits")
	_support.expect_equal(direct_intents, [cast], "direct cast remains exact")

	var warm_row: Dictionary = _spell("ember", "warm_then_cast")
	var warm: InteractionDirector = InteractionDirector.new()
	warm.set_input_enabled(true)
	warm.present_frame(_frame([], [warm_row]), 1)
	var warm_intents: Array[Dictionary] = []
	var warm_targets: Array[String] = []
	warm.intent_requested.connect(func(intent: Dictionary, _label: String) -> void: warm_intents.append(intent.duplicate(true)))
	warm.spell_target_ready.connect(func(spell_id: String) -> void: warm_targets.append(spell_id))
	warm.activate_spell("ember", 2000)
	warm.activate_spell("ember", 2300)
	_support.expect_equal(warm_intents, [{"kind": "warm_spell", "spell_id": "ember"}], "warm-then-cast submits only exact Warm at Prepare")
	warm.mark_spell_chant_complete()
	_support.expect(warm_targets.is_empty(), "chant alone cannot open target before authoritative warmed readiness")
	var warmed_frame: Dictionary = _frame([], [warm_row])
	warmed_frame["warmed_spell"] = {"spell_id": "ember", "warmed_at": "10", "ready_at": "11", "status": "ready"}
	warm.present_frame(warmed_frame, 2)
	_support.expect_equal(warm_targets, ["ember"], "complete same-spell ready frame opens warmed target")
	var warmed_cast: Dictionary = {"kind": "cast_warmed_spell", "target": {"kind": "actor", "actor_id": "enemy"}, "authorization": "safe"}
	_support.expect(warm.submit_prepared_spell_target(warmed_cast, "Cast Ember"), "fresh warmed target submits")
	_support.expect_equal(warm_intents[1], warmed_cast, "Warm and Cast remain two exact commands")


func test_buffers_bow_drag_and_spell_prepare_cancel_on_every_authority_edge() -> void:
	var waiting: InteractionDirector = _director([
		_physical("fight", "enemy", false, "not_ready"),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
	])
	_activate_actor_twice(waiting, 1)
	_support.expect(not waiting.not_ready_buffer().is_empty(), "not-ready buffer begins from one exact row")
	waiting.activate_semantic_target(_semantic_target("ground_item", "item:1", 1), 1500)
	_support.expect(waiting.not_ready_buffer().is_empty(), "target change cancels the buffer")
	_activate_actor_twice(waiting, 1)
	waiting.note_unrelated_command()
	_support.expect(waiting.not_ready_buffer().is_empty(), "unrelated command cancels the buffer")
	_activate_actor_twice(waiting, 1)
	waiting.set_ranged_mode("throw")
	_support.expect(waiting.not_ready_buffer().is_empty(), "preference change cancels the buffer")
	waiting.set_ranged_mode("jumpkick")
	_activate_actor_twice(waiting, 1)
	waiting.set_input_enabled(false)
	_support.expect(waiting.not_ready_buffer().is_empty() and waiting.selected_target().is_empty(), "input/authority disable discards buffer and selection")

	var shoot_wait: Dictionary = _physical("shoot", "enemy", false, "bow_not_nocked")
	var nock: Dictionary = {"id": "nock", "label": "Nock bow", "enabled": true, "blocked_reason": null, "intent": {"kind": "nock"}}
	var bow: InteractionDirector = _director([
		_physical("fight", "enemy", false, "out_of_range"),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
		shoot_wait,
		nock,
	])
	bow.set_ranged_mode("shoot")
	_activate_actor_twice(bow, 1)
	_support.expect(not bow.bow_chain().is_empty(), "Nock-to-Shoot chain begins from exact current rows")
	bow.note_unrelated_command()
	_support.expect(bow.bow_chain().is_empty(), "unrelated command cancels the bow chain")
	_activate_actor_twice(bow, 1)
	bow.set_ranged_mode("throw")
	_support.expect(bow.bow_chain().is_empty(), "preference change cancels the bow chain")
	bow.set_ranged_mode("shoot")
	_activate_actor_twice(bow, 1)
	bow.note_command_result("nock", true)
	var unsafe_frame: Dictionary = _frame([
		_physical("fight", "enemy", false, "out_of_range"),
		_physical("poke", "enemy", false, "physical_mode_not_supported"),
		_physical("shoot", "enemy", true, null),
		nock,
	])
	unsafe_frame["actors"][1]["attack_safety"] = "protected"
	bow.present_frame(unsafe_frame, 2)
	_support.expect(bow.bow_chain().is_empty(), "safety change cancels stale bow consent")

	var direct: InteractionDirector = InteractionDirector.new()
	direct.set_input_enabled(true)
	direct.present_frame(_frame([], [_spell("spark", "direct"), _spell("ember", "direct")]), 1)
	direct.activate_spell("spark", 1000)
	direct.activate_spell("spark", 1200)
	_support.expect_equal(direct.preparing_spell_id(), "spark", "direct spell enters Prepare")
	direct.activate_spell("ember", 1300)
	_support.expect(direct.preparing_spell_id().is_empty(), "selecting another spell cancels Prepare")
	direct.activate_spell("ember", 1400)
	direct.activate_spell("ember", 1500)
	var reshaped: Dictionary = _frame([], [_spell("spark", "direct")])
	direct.present_frame(reshaped, 2)
	_support.expect(direct.preparing_spell_id().is_empty() and direct.selected_spell_id().is_empty(), "row-count replacement removes incompatible selection and Prepare safely")
	direct.activate_spell("spark", 2000)
	direct.activate_spell("spark", 2200)
	var blocked_row: Dictionary = _spell("spark", "direct")
	blocked_row["cast"]["enabled"] = false
	blocked_row["cast"]["blocked_reason"] = "not_ready"
	direct.present_frame(_frame([], [blocked_row]), 3)
	_support.expect(direct.preparing_spell_id().is_empty(), "replacement frame cannot let chant make a disabled direct cast legal")


func _director(options: Array) -> InteractionDirector:
	var director: InteractionDirector = InteractionDirector.new()
	director.set_input_enabled(true)
	director.present_frame(_frame(options), 1)
	return director


func _activate_actor_twice(director: InteractionDirector, generation: int) -> void:
	var target: Dictionary = _semantic_target("actor", "enemy", generation)
	director.activate_semantic_target(target, 1000)
	director.activate_semantic_target(target, 1300)


func _frame(options: Array, spells: Array = []) -> Dictionary:
	return {
		"logical_time": "10",
		"ready_at": "11",
		"actors": [
			{"actor_id": "player", "life_state": "alive", "attack_safety": "open_hostile"},
			{"actor_id": "enemy", "life_state": "alive", "attack_safety": "open_hostile"},
		],
		"tiles": [],
		"corpses": [{"corpse_id": "corpse:1"}],
		"ground_items": [
			{"item_instance_id": "item:1"},
			{"item_instance_id": "item:adjacent"},
		],
		"gold_piles": [{"gold_pile_id": "gold:adjacent"}],
		"action_options": options,
		"action_options_truncated": false,
		"ground_items_truncated": false,
		"spell_actions": spells,
		"warmed_spell": null,
	}


func _physical(mode: String, actor_id: String, enabled: bool, reason: Variant) -> Dictionary:
	return {
		"id": "physical_attack_%s_%s" % [mode, actor_id],
		"label": "%s Enemy" % mode.capitalize(),
		"enabled": enabled,
		"blocked_reason": reason,
		"intent": {
			"kind": "physical_attack",
			"mode": mode,
			"target_actor_id": actor_id,
			"authorization": "safe",
		},
	}


func _spell(spell_id: String, method: String) -> Dictionary:
	return {
		"spell_id": spell_id,
		"spell_name": spell_id.capitalize(),
		"casting_method": method,
		"cast_class": "character",
		"target_kind": "actor",
		"mp_cost": 1,
		"stamina_cost": null,
		"warm": {
			"enabled": method == "warm_then_cast",
			"blocked_reason": null if method == "warm_then_cast" else "spell_casts_directly",
			"requires_target_selection": false,
			"intent": {"kind": "warm_spell", "spell_id": spell_id} if method == "warm_then_cast" else null,
		},
		"cast": {"enabled": true, "blocked_reason": null, "requires_target_selection": true, "intent": null},
	}


func _semantic_target(
	kind: String,
	source_identity: String,
	generation: int,
	ground_reach: String = WorldTargets.GROUND_REACH_MANIPULATE,
) -> Dictionary:
	var target: Dictionary = {
		"identity": WorldTargets.semantic_identity(kind, source_identity),
		"kind": kind,
		"source_identity": source_identity,
		"coordinate": Vector2i.ZERO,
		"generation": generation,
	}
	if kind in WorldTargets.GROUND_KINDS:
		target["ground_reach"] = ground_reach
		target["examine_reachable"] = true
		target["manipulation_reachable"] = (
			ground_reach == WorldTargets.GROUND_REACH_MANIPULATE
		)
	return target
