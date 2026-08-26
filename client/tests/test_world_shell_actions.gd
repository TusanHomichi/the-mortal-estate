extends RefCounted

## Acting on the world through the shell: ground state, inspection, exact
## server-offered actions, unsafe confirmation, and the domain surfaces.
##
## The rule every test here holds to is that the client submits an intent the
## server offered, or it submits nothing. Nothing is composed locally.

const ShellSupport: Script = preload("res://tests/shell_test_support.gd")

var _support: TestSupport


func test_ground_item_drag_survives_replacement_frame_and_fails_closed_if_item_vanishes() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.fk_ground_reach_frame(), 60)
	var own_item: Dictionary = ShellSupport.ground_target(screen, "ground_item", "item:own")
	_support.expect(screen.interaction_director.begin_primary_press(own_item, Vector2(10, 10), 1000), "the own-square item press begins")
	_support.expect(screen.interaction_director.update_pointer(Vector2(18, 10)), "eight pixels turns the press into a drag")

	screen.present_frame(ShellSupport.fk_ground_reach_frame(), 61)
	_support.expect(not screen.interaction_director.drag_state().is_empty(), "a routine authoritative replacement frame preserves the live drag")
	_support.expect(not screen.interaction_director.drag_state().has("generation"), "drag state carries stable intent rather than a frame-generation currency")
	_support.expect(screen.interaction_director.complete_item_drag("sack_item_1"), "release validates and submits against the current frame")
	_support.expect_equal(intents, [{"kind": "move_item", "item_instance_id": "item:own", "destination": {"kind": "carried", "position": "sack_item_1"}}], "the cross-frame drag submits the exact current action")

	screen.present_frame(ShellSupport.fk_ground_reach_frame(), 62)
	own_item = ShellSupport.ground_target(screen, "ground_item", "item:own")
	screen.interaction_director.begin_primary_press(own_item, Vector2(10, 10), 2000)
	screen.interaction_director.update_pointer(Vector2(18, 10))
	var vanished_frame: Dictionary = ShellSupport.fk_ground_reach_frame()
	vanished_frame["ground_items"] = (vanished_frame["ground_items"] as Array).filter(func(item: Dictionary) -> bool: return item.get("item_instance_id") != "item:own")
	vanished_frame["action_options"] = []
	screen.present_frame(vanished_frame, 63)
	_support.expect(not screen.interaction_director.complete_item_drag("sack_item_1"), "a vanished item fails closed on release against the current frame")
	_support.expect("No exact action in this frame" in screen.action_indicator.text, "the vanished item produces the existing rejection feedback instead of dying silently")
	_support.expect_equal(intents.size(), 1, "the vanished item emits no second move")
	screen.free()


func test_new_frame_reconciles_pointer_preview_and_cancel_invalidates_it() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.begin_pointer_draft(Vector2i(1, 0))
	screen.apply_path_preview_result(ShellSupport.preview_result(["east"]))
	screen.present_frame(ShellSupport.world_frame([]), 2, true, true)
	_support.expect(screen.has_current_preview() and screen.draft_path() == ["east"], "generation-only replacement preserves an authority-current preview")
	screen.present_frame(ShellSupport.world_frame([]), 3, true, false)
	_support.expect(not screen.has_current_preview() and screen.draft_path() == ["east"], "authority revision replacement keeps target intent but requests a fresh preview")
	_support.expect_equal(previews, [["east"], ["east"]], "authority change replaces the stale preview request exactly once")
	screen.clear_pointer_draft()
	_support.expect(screen.draft_path().is_empty(), "explicit cancel drops draft")
	screen.free()


func test_right_click_inspect_and_typed_stairs_use_exact_server_options() -> void:
	var options: Array = [
		{"id": "inspect", "label": "Inspect", "enabled": true, "blocked_reason": null, "intent": {"kind": "inspect"}},
		{"id": "stairs_up", "label": "Stairs Up", "enabled": true, "blocked_reason": null, "intent": {"kind": "traverse", "traversal": "stairs_up"}},
		{"id": "stairs_down", "label": "Stairs Down", "enabled": false, "blocked_reason": "wrong_stair_direction", "intent": {"kind": "traverse", "traversal": "stairs_down"}},
	]
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame(options), 4)
	_support.expect(screen.request_inspect(), "right-click route submits enabled inspect option")
	_support.expect(screen.submit_stairs("stairs_up"), "stairs-up route submits enabled typed option")
	_support.expect(not screen.submit_stairs("stairs_down"), "disabled stairs-down option emits nothing")
	_support.expect_equal(intents, [{"kind": "inspect"}, {"kind": "traverse", "traversal": "stairs_up"}], "special controls preserve exact projected intents")
	_support.expect("wrong_stair_direction" in screen.stairs_down_button.tooltip_text and "Unavailable" in screen.stairs_down_button.accessibility_description, "disabled stair control retains the server reason in pointer and accessibility text")
	var inspected: Dictionary = {
		"kind": "inspected", "tile": "Stone", "tile_move_cost": 1,
		"exits": [{"direction": "east", "status": {"kind": "door", "open": false}}],
		"nearby_actors": [{"direction": "north", "actor": "Mireling", "kind": "monster", "hp": 4}],
		"ground_items": [{"direction": null, "name": "Ground Item", "quantity": 1}],
	}
	_support.expect(screen.present_observed_events([inspected]), "privacy-safe inspected event renders")
	_support.expect("Stone · move cost 1" in screen.inspect_panel.text and "Door closed".to_lower() in screen.inspect_panel.text.to_lower(), "inspect panel renders finite safe tile and door facts")
	_support.expect("Mireling · monster · HP 4" in screen.inspect_panel.text and "Ground Item ×1" in screen.inspect_panel.text, "inspect panel renders only supplied safe actor/item rows")
	screen.present_frame(ShellSupport.world_frame([options[1], options[2]]), 5)
	_support.expect("Stone · move cost 1" in screen.inspect_panel.text and "Ground Item ×1" in screen.inspect_panel.text, "ordinary replacement frames preserve the latest inspect result")
	var replacement_inspect: Dictionary = inspected.duplicate(true)
	replacement_inspect["tile"] = "Slate"
	replacement_inspect["ground_items"] = []
	_support.expect(screen.present_observed_events([replacement_inspect]), "a later privacy-safe inspected event renders")
	_support.expect("Slate · move cost 1" in screen.inspect_panel.text and "Ground Item" not in screen.inspect_panel.text, "the next inspect result replaces the prior result")
	_support.expect_equal(screen.first_focus_control(), screen.stairs_up_button, "typed stairs remain reachable from the action-options keyboard route")
	screen.free()


func test_inspect_result_resets_only_at_world_lifecycle_boundaries() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var inspected: Dictionary = {"kind": "inspected", "tile": "Granite", "tile_move_cost": 2, "exits": [], "nearby_actors": [], "ground_items": []}
	_support.expect(screen.present_observed_events([inspected]), "inspect result renders before lifecycle transitions")
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.set_connection_state("ONLINE", true)
	_support.expect("Granite · move cost 2" in screen.inspect_panel.text, "ordinary frames and same-session connection sync preserve inspect text")
	screen.set_connection_state("RETRYABLE_ERROR", false)
	_support.expect("Granite · move cost 2" in screen.inspect_panel.text, "non-lifecycle error presentation does not clear inspect text")

	screen.hide()
	screen.show()
	_support.expect_equal(screen.inspect_panel.text, WorldShellScreen.INSPECT_PLACEHOLDER, "world-screen entry restores the inspect placeholder")
	for boundary: String in ["RECONCILING", "LOGGING_OUT"]:
		screen.set_connection_state("ONLINE", true)
		screen.present_observed_events([inspected])
		screen.set_connection_state(boundary, false)
		_support.expect_equal(screen.inspect_panel.text, WorldShellScreen.INSPECT_PLACEHOLDER, boundary + " restores the inspect placeholder")

	screen.present_observed_events([inspected])
	screen.refresh_discarded_presentation()
	_support.expect_equal(screen.inspect_panel.text, WorldShellScreen.INSPECT_PLACEHOLDER, "successful logout discard retains the inspect placeholder")
	screen.free()


func test_readiness_replaces_from_frames_and_never_ticks_locally() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var waiting: Dictionary = ShellSupport.world_frame([])
	waiting["logical_time"] = "12"
	waiting["ready_at"] = "15"
	waiting["can_act"] = false
	screen.present_frame(waiting, 1)
	var exact_waiting_text: String = screen.readiness_status.text
	_support.expect_equal(
		exact_waiting_text,
		"◇ Ready in 3 beats · beat unmeasured · world T12 · ready T15",
		"waiting display uses exact frame facts, and claims no fill it has not measured",
	)
	screen._process(999.0)
	_support.expect_equal(
		screen.readiness_status.text,
		exact_waiting_text,
		"wall-clock processing does not advance readiness, the wait, or a fill with no measured beat behind it",
	)
	var ready: Dictionary = waiting.duplicate(true)
	ready["logical_time"] = "15"
	ready["can_act"] = true
	screen.present_frame(ready, 2)
	_support.expect_equal(
		screen.readiness_status.text,
		"◆ Ready · beat unmeasured · world T15 · ready T15",
		"new full frame atomically replaces readiness",
	)
	screen.free()


## The readiness line changes on the beat, not inside it.
##
## A label's text drives its minimum size, and its minimum size drives the top
## rail's layout. When this string carried a live fill percentage it changed on
## every drawn frame, so the rail re-resolved its whole layout on every drawn
## frame — and a real live proof caught the stall as a 536 ms gap between two
## authoritative rounds. The fill belongs to the meter, which redraws freely
## because redrawing costs no layout.
func test_the_readiness_line_changes_on_the_beat_not_inside_it() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var waiting: Dictionary = ShellSupport.world_frame([])
	waiting["logical_time"] = "20"
	waiting["ready_at"] = "23"
	waiting["can_act"] = false
	screen.pulse_clock.set_test_clock(1000)
	screen.present_frame(waiting, 1)
	var second: Dictionary = waiting.duplicate(true)
	second["logical_time"] = "21"
	screen.pulse_clock.set_test_clock(4000)
	screen.present_frame(second, 2)
	_support.expect(screen.pulse_clock.has_measured_span(), "two rounds give the meter a span to fill across")

	var settled: String = screen.readiness_status.text
	var fills: Array[float] = []
	for step: int in 10:
		screen.pulse_clock.set_test_clock(4000 + step * 250)
		screen._process(0.016)
		fills.append(float(screen.pulse_meter.segments()[0]["fill"]))
		_support.expect_equal(
			screen.readiness_status.text,
			settled,
			"the readiness line is unchanged %d ms into the beat" % (step * 250),
		)
	_support.expect(
		fills[-1] > fills[0] + 0.5,
		"while the meter's fill advanced across the same beat, %.2f to %.2f" % [fills[0], fills[-1]],
	)

	var third: Dictionary = waiting.duplicate(true)
	third["logical_time"] = "22"
	screen.pulse_clock.set_test_clock(7000)
	screen.present_frame(third, 3)
	screen._process(0.016)
	_support.expect(
		screen.readiness_status.text != settled,
		"and the authoritative beat is what changes it",
	)
	screen.free()


func test_server_action_option_submits_exact_intent() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var supplied_intent: Dictionary = {"kind": "cast_spell", "spell_id": "synthetic_spell", "target": {"kind": "none"}, "authorization": "safe"}
	var emitted: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: emitted.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.frame_with_options([{"id": "action:exact", "label": "Synthetic cast", "enabled": true, "blocked_reason": null, "intent": supplied_intent}]), 8)
	_support.expect(screen.submit_action_option(0), "enabled server option submits")
	_support.expect_equal(emitted, [supplied_intent], "server-supplied intent is emitted byte-shape-equivalent without legality rewrite")
	screen.set_command_pending(true)
	_support.expect(not screen.submit_action_option(0), "at-most-one pending guard blocks another option")
	screen.free()


func test_unsafe_confirmation_names_target_defaults_cancel_and_invalidates() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var unsafe_intent: Dictionary = {"kind": "physical_attack", "mode": "fight", "target_actor_id": "synthetic_actor", "authorization": "confirmed_unsafe"}
	var frame: Dictionary = ShellSupport.frame_with_options([{"id": "action:unsafe", "label": "Risky strike", "enabled": true, "blocked_reason": null, "intent": unsafe_intent}])
	frame["actors"] = [{"actor_id": "synthetic_actor", "name": "Training Golem", "position": {"realm": "synthetic", "level": "surface", "position": {"x": 1, "y": 0}}, "life_state": "alive"}]
	var emitted: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: emitted.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(frame, 12)
	_support.expect(screen.submit_action_option(0), "unsafe option opens confirmation")
	_support.expect("Risky strike" in screen.confirmation.prompt_text(), "dialog names the action")
	_support.expect("Training Golem (synthetic_actor)" in screen.confirmation.prompt_text(), "dialog names target and ID")
	_support.expect(screen.confirmation.cancel_has_default_focus(), "unsafe dialog defaults to Cancel")
	screen.present_frame(frame, 13)
	_support.expect(not screen.confirmation.visible, "authoritative frame generation change invalidates dialog")
	_support.expect(emitted.is_empty(), "invalidation submits nothing")
	_support.expect(screen.submit_action_option(0), "unsafe option may be deliberately reopened")
	_support.expect(screen.confirmation.confirm(13), "same-generation deliberate confirmation succeeds")
	_support.expect_equal(emitted, [unsafe_intent], "confirmed option emits the exact supplied intent")
	screen.free()


func test_opening_unsafe_confirmation_clears_an_armed_movement_draft() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var unsafe_intent: Dictionary = {"kind": "physical_attack", "mode": "fight", "target_actor_id": "synthetic_actor", "authorization": "confirmed_unsafe"}
	var frame: Dictionary = ShellSupport.world_frame([{"id": "action:unsafe", "label": "Risky strike", "enabled": true, "blocked_reason": null, "intent": unsafe_intent}])
	frame["actors"].append({"actor_id": "synthetic_actor", "name": "Training Golem", "position": {"realm": "synthetic", "level": "surface", "position": {"x": 1, "y": 0}}, "life_state": "alive"})
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(frame, 12)
	screen.activate_square(Vector2i(1, 0), 1000)
	screen.activate_square(Vector2i(1, 0), 1200)
	_support.expect(screen.pointer_commit_armed(), "movement is armed while its preview is pending")

	_support.expect(screen.submit_action_option(0), "the player's unsafe attack choice opens confirmation")
	_support.expect(screen.confirmation.visible, "the unsafe confirmation is visible")
	_support.expect(screen.draft_path().is_empty() and not screen.pointer_commit_armed(), "opening the unsafe dialog disarms and clears the abandoned movement draft")
	_support.expect(not screen.apply_path_preview_result(ShellSupport.preview_result(["east"]), 1300), "the abandoned movement preview cannot land behind the modal")
	_support.expect(intents.is_empty(), "opening the dialog submits neither movement nor attack")
	screen.free()


func test_domain_drawer_focus_frame_and_unsafe_target_route_integrate_with_world_shell() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var frame: Dictionary = ShellSupport.world_frame([])
	frame["character"] = {
		"identity": {"base_class_id": "fighter", "current_class_id": "fighter", "display_class": "Fighter", "nationality_id": "northreach", "sex_or_gender_display": null},
		"alignment": "neutral", "karma_points": 0,
		"attributes": {"strength": 10, "dexterity": 10, "constitution": 10, "intelligence": 10, "wisdom": 10, "charisma": 10},
		"resources": {"hp": 10, "max_hp": 10, "peak_hp": 10, "mp": 5, "max_mp": 5, "stamina": 10, "max_stamina": 10},
		"progression": {"level": 1, "experience": "0", "pending_target_level": null},
		"physical_attribute_adds": {"strength_adds": 0, "dexterity_adds": 0}, "promotion_history": [],
		"known_spells": [{"spell_id": "spark", "lane": "attack", "learned_at_level": 1}], "skill_ledger": [],
	}
	frame["actors"].append({"actor_id": "target", "name": "Target Golem", "kind": "monster", "character_id": null, "position": {"realm": "synthetic", "level": "surface", "position": {"x": 1, "y": 0}}, "life_state": "alive", "hp": 10, "max_hp": 10, "attack_safety": "open_hostile"})
	frame["carried"] = {"items": [], "gold": {"left_hand": "0", "right_hand": "0", "sack": "0"}}
	frame["burden"] = {"item_burden": "0", "coin_burden": "0", "total_burden": "0", "lightly_loaded_limit": "10", "moderately_loaded_limit": "20", "heavily_loaded_limit": "30", "tier": "lightly_loaded"}
	frame["warmed_spell"] = null
	frame["spell_actions"] = [{"spell_id": "spark", "spell_name": "Spark", "casting_method": "direct", "cast_class": "character", "target_kind": "actor", "mp_cost": 1, "stamina_cost": null, "hostile_act": true, "town_law_violation": false, "warm": {"enabled": true, "blocked_reason": null, "requires_target_selection": false, "intent": {"kind": "warm_spell", "spell_id": "spark"}}, "cast": {"enabled": true, "blocked_reason": null, "requires_target_selection": true, "intent": null}}]
	frame["social"] = {"character_id": "char:self", "group": null, "incoming_invitations": [], "outgoing_invitations": [], "following_character_id": null, "pages_enabled": true, "blocked_character_ids": []}
	frame["incoming_item_offers"] = []
	frame["outgoing_item_offers"] = []
	frame["services_here"] = []
	frame["npcs_here"] = []
	frame["quest_log"] = []
	var emitted: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: emitted.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(frame, 20)
	screen.domains_button.grab_focus()
	screen.open_domains()
	_support.expect(screen.domain_panel.visible and screen.get_viewport().gui_get_focus_owner() == screen.domain_panel.mode_selector, "domain drawer opens with deterministic mode focus")
	screen.domain_panel.select_mode("Magic")
	screen.domain_panel.set_actor_target("target")
	screen.domain_panel.set_authorization("confirmed_unsafe")
	await (Engine.get_main_loop() as SceneTree).process_frame
	var cast_button: Button = null
	for button: Button in screen.domain_panel._action_buttons:
		if "Cast at current target" in button.text:
			cast_button = button
	_support.expect(cast_button != null and not cast_button.disabled, "current projected target enables one authored finite cast attempt")
	cast_button.pressed.emit()
	_support.expect(screen.confirmation.visible and "Target Golem (target)" in screen.confirmation.prompt_text(), "unsafe domain intent reuses target-naming cancel-default confirmation")
	_support.expect(screen.confirmation.confirm(20), "same-frame unsafe domain intent confirms deliberately")
	_support.expect_equal(emitted, [{"kind": "cast_spell", "spell_id": "spark", "target": {"kind": "actor", "actor_id": "target"}, "authorization": "confirmed_unsafe"}], "world shell forwards the exact authored Protocol intent")
	screen.close_domains()
	_support.expect(not screen.domain_panel.visible and screen.get_viewport().gui_get_focus_owner() == screen.domains_button, "closing drawer restores opener focus")
	screen.free()


func test_hud_spell_button_uses_production_director_domain_wiring_without_orphaning() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var frame: Dictionary = ShellSupport.world_frame([])
	frame["spell_actions"] = [{
		"spell_id": "spark",
		"spell_name": "Spark",
		"casting_method": "direct",
		"cast_class": "character",
		"target_kind": "actor",
		"mp_cost": 1,
		"stamina_cost": null,
		"hostile_act": true,
		"town_law_violation": false,
		"warm": {"enabled": false, "blocked_reason": "spell_casts_directly", "requires_target_selection": false, "intent": null},
		"cast": {"enabled": true, "blocked_reason": null, "requires_target_selection": true, "intent": null},
	}]
	frame["character"] = {"known_spells": [{"spell_id": "spark", "lane": "attack", "learned_at_level": 1}]}
	frame["social"] = {"character_id": "char:self"}
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(frame, 30)
	screen.domain_panel.select_mode("Magic")
	var old_domain_palette: SpellPalette = screen.domain_panel.content.get_node("MagicSpellPalette") as SpellPalette
	var old_domain_content_count: int = screen.domain_panel.content.get_child_count()
	var hud_button: Button = screen.hud.spell_palette.buttons()[0]
	hud_button.grab_focus()

	hud_button.pressed.emit()
	_support.expect_equal(screen.interaction_director.selected_spell_id(), "spark", "the real HUD button reaches InteractionDirector spell selection")
	_support.expect_equal(screen.hud.spell_palette.selected_spell_id(), "spark", "the production callback updates HUD spell state during the same gesture")
	_support.expect(is_instance_valid(hud_button) and hud_button.get_parent() == screen.hud.spell_palette.spell_grid, "the real HUD emitter remains parented until pressed emission returns")
	_support.expect(is_instance_valid(old_domain_palette) and old_domain_palette.get_parent() == screen.domain_panel.content, "DomainPanel leaves its old palette parented until the external HUD emission returns")
	_support.expect_equal(screen.domain_panel.content.get_child_count(), old_domain_content_count, "the Magic body remains complete during the production signal chain")

	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(not is_instance_valid(old_domain_palette), "the production DomainPanel rebuild runs on the real deferred frame")
	_support.expect_equal(screen.domain_panel.content.get_children().filter(func(child: Node) -> bool: return child is SpellPalette).size(), 1, "the production chain leaves exactly one parented DomainPanel palette")
	_support.expect_equal(screen.domain_panel.content.get_node("MagicSpellPalette").get_parent(), screen.domain_panel.content, "the replacement palette is not orphaned")
	var focus_owner: Control = screen.get_viewport().gui_get_focus_owner()
	_support.expect(focus_owner != null and focus_owner.get_meta("spell_id", "") == "spark", "the real deferred HUD rebuild restores spell focus")
	screen.free()
