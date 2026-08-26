extends RefCounted

var _support: TestSupport


func test_exact_resources_nearest_first_selection_and_group_privacy() -> void:
	var hud: FullHud = _add_hud(1920)
	var frame: Dictionary = _frame()
	frame["actors"].append({"actor_id": "actor-dead", "character_id": null, "name": "Fallen Mireling", "kind": "monster", "position": _position(1, 0), "life_state": "dead", "hp": 0, "max_hp": 7, "attack_safety": "invalid"})
	frame["actors"].append({"actor_id": "actor-ghost", "character_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc", "name": "Ghost Ally", "kind": "player", "position": _position(3, 3), "life_state": "ghost", "hp": 0, "max_hp": 10, "attack_safety": "invalid"})
	hud.present_frame(frame)
	_support.expect_equal(hud.hp_label.text, "HP 7/12", "HP uses exact complete-frame values")
	_support.expect_equal(hud.stamina_label.text, "STAMINA -1/10", "negative stamina remains honest in text while only drawing clamps")
	_support.expect_equal(hud.mp_label.text, "MP 15/10", "over-max MP remains honest in text while only drawing clamps")
	var rows: Array[Dictionary] = hud.nearest_actor_rows()
	_support.expect_equal([rows[0]["actor_id"], rows[1]["actor_id"], rows[2]["actor_id"]], ["actor-a", "actor-z", "actor-b"], "nearby actors sort by Chebyshev distance then stable actor ID")
	_support.expect_equal(rows.size(), 4, "dead rows leave Focus while a projected ghost player remains present")
	_support.expect_equal(rows[3]["actor_id"], "actor-ghost", "ghost player remains in the nearest projected actor rows")
	_support.expect("cccccccc-cccc-4ccc-8ccc-cccccccccccc" in hud.call("_visible_page_recipient_ids"), "paging can still address a projected ghost player")
	_support.expect("[NPC] Quartermaster" in hud.focus_button_texts()[0] and "d1" in hud.focus_button_texts()[0], "focus row has non-color kind and exact distance")
	_support.expect("◆ LEADER · Wayfarer" in hud.group_text() and "Visible Ally" in hud.group_text(), "visible character identity safely supplies current member names")
	_support.expect("Member …dddddddd" in hud.group_text() and not "HP" in hud.group_text(), "unseen group member uses abbreviated stable ID and invents no resources")
	hud.set_selected_actor("actor-z")
	_support.expect_equal(hud.selected_actor_id(), "actor-z", "current visible actor can be selected locally")
	frame["actors"] = [frame["actors"][0], frame["actors"][1], frame["actors"][3]]
	hud.present_frame(frame)
	_support.expect_equal(hud.selected_actor_id(), "", "replacement frame clears a disappeared local selection")
	hud.free()


func test_bottom_plate_uses_dynamic_spells_ranged_selector_more_and_help_without_action_mirrors() -> void:
	var hud: FullHud = _add_hud(1920)
	var frame: Dictionary = _frame()
	frame["spell_actions"] = [
		_spell_row("spark", true, null),
		_spell_row("ward", false, "not_ready"),
		_spell_row("light", true, null),
	]
	hud.present_frame(frame)
	hud.set_spell_state("ward", "ward")
	_support.expect_equal(hud.spell_palette.ready_control_count(), 3, "bottom plate mirrors exactly the projected spell rows")
	_support.expect(hud.spell_palette.single_row and hud.spell_palette.spell_grid.columns == 3, "bottom plate lays every current row into one horizontally scrollable strip")
	_support.expect("◈ Ward" in hud.spell_palette.buttons()[1].text and "Not Ready" in hud.spell_palette.buttons()[1].text, "selected Prepare state and exact blocked reason are visible")
	var ranged: Array[String] = []
	hud.ranged_mode_selected.connect(func(mode: String) -> void: ranged.append(mode))
	hud.set_ranged_state("shoot", {"shoot": {"enabled": false, "blocked_reason": "bow_not_nocked"}})
	_support.expect("Bow Not Nocked" in hud.ranged_selector.get_item_text(2), "ranged selector exposes the exact current reason")
	hud.ranged_selector.select(1)
	hud.ranged_selector.item_selected.emit(1)
	_support.expect_equal(ranged, ["throw"], "ranged selector emits only its finite exact mode")
	var launches: Array[String] = []
	hud.context_requested.connect(func() -> void: launches.append("context"))
	hud.help_requested.connect(func() -> void: launches.append("help"))
	hud.more_button.pressed.emit()
	hud.help_button.pressed.emit()
	_support.expect_equal(launches, ["context", "help"], "More and Help are dedicated discovery controls")
	hud.set_command_pending(true)
	_support.expect(hud.spell_palette.buttons().all(func(button: Button) -> bool: return button.disabled) and hud.ranged_selector.disabled, "pending command blocks interactive combat controls")
	hud.free()


func test_dynamic_spell_controls_remain_focusable_at_enlarged_scale() -> void:
	var hud: FullHud = _add_hud(1280)
	var scaled_theme: Theme = Theme.new()
	scaled_theme.default_font_size = 32
	hud.theme = scaled_theme
	hud.apply_text_scale(200)
	var frame: Dictionary = _frame()
	frame["spell_actions"] = [_spell_row("ember_bolt", true, null), _spell_row("greater_ward", false, "insufficient_magic_points")]
	hud.present_frame(frame)
	_support.expect_equal(hud.spell_palette.ready_control_count(), 2, "large text does not fabricate or drop ready controls")
	for button: Button in hud.spell_palette.buttons():
		_support.expect(button.focus_mode != Control.FOCUS_NONE, button.text + " remains keyboard focusable")
		_support.expect(not button.clip_text, button.text + " sizes naturally instead of clipping its semantic label")
	_support.expect_equal(hud.spell_palette.spell_grid.columns, 2, "large-text bottom strip derives its column count from current rows")
	hud.free()


func test_chat_emits_only_semantic_current_scopes_and_page_targets() -> void:
	var hud: FullHud = _add_hud(1920)
	var frame: Dictionary = _frame()
	frame["social"]["group"] = null
	hud.present_frame(frame)
	var emitted: Array[Dictionary] = []
	hud.social_message_requested.connect(func(scope: Dictionary, body: String) -> void: emitted.append({"scope": scope.duplicate(true), "body": body}))
	_support.expect(hud.submit_chat("  hello  ", "say"), "say emits")
	_support.expect(not hud.submit_chat("group line", "group"), "group is unavailable without authoritative membership")
	_support.expect(not hud.submit_chat("page", "page", "not-visible"), "page rejects a nonprojected recipient")
	_support.expect(hud.submit_chat("page", "page", "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"), "page accepts a currently visible player character")
	frame["social"]["group"] = _group()
	hud.present_frame(frame)
	_support.expect(hud.submit_chat("group line", "group"), "group emits after a complete frame supplies membership")
	_support.expect_equal(emitted, [
		{"scope": {"kind": "say"}, "body": "hello"},
		{"scope": {"kind": "page", "target_character_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"}, "body": "page"},
		{"scope": {"kind": "group"}, "body": "group line"},
	], "HUD emits scope and body only; adapter identity/envelope facts remain absent")
	hud.free()


func test_panes_resize_restore_focus_filter_and_time_nonart_cues() -> void:
	var hud: FullHud = _add_hud(1920)
	hud.apply_responsive_layout(1920)
	_support.expect(hud.pane_visible("group") and hud.pane_visible("focus") and hud.pane_visible("chat") and hud.pane_visible("feedback"), "native layout opens the four edge-pinned panes")
	hud.apply_responsive_layout(1280)
	_support.expect(not hud.pane_visible("group") and not hud.pane_visible("focus") and not hud.pane_visible("chat") and not hud.pane_visible("feedback"), "minimum layout collapses nonessential panes")
	hud.toggle_pane("chat", true)
	_support.expect_equal(hud.get_viewport().gui_get_focus_owner(), hud.chat_log, "opening chat gives deterministic scrollback focus")
	hud.toggle_pane("group", true)
	var compact_top: float = hud.chat_pane.offset_top
	hud.cycle_output_size("chat")
	_support.expect(hud.chat_pane.offset_top < compact_top and hud.chat_size_button.text == "Compact" and not hud.pane_visible("group"), "expanded output preset prevents same-side pane overlap")
	hud.apply_responsive_layout(1280)
	hud.apply_responsive_layout(1920)
	_support.expect(hud.pane_visible("chat") and not hud.pane_visible("group"), "resize round trip preserves expanded output without reopening its same-side pane")
	hud.toggle_pane("chat", false)
	_support.expect_equal(hud.get_viewport().gui_get_focus_owner(), hud.chat_launcher, "collapsing chat restores its edge launcher")
	var events: Array = [
		{"kind": "feedback", "cue": _combat_cue("First target")},
		{"kind": "feedback", "cue": _combat_cue("Second target")},
		{"kind": "feedback", "cue": _resource_cue()},
	]
	hud.present_observed_events(events, "event:connection:9")
	_support.expect(hud.cue_banner.visible and "First target" in hud.cue_label.text, "first high-salience cue appears as icon-plus-text without blocking")
	hud.advance_cue_time(FullHud.CUE_DURATION_SECONDS + 0.1)
	_support.expect(hud.cue_banner.visible and "Second target" in hud.cue_label.text, "bounded cue timing advances deterministically")
	hud.feedback_filter.select(1)
	hud.refresh_scrollback()
	_support.expect("First target" in hud.feedback_log.text and "RESOURCE" not in hud.feedback_log.text, "combat filter hides system entries without deleting history")
	_support.expect(not hud.feedback_log.bbcode_enabled and not hud.chat_log.bbcode_enabled, "server strings remain plain text")
	hud.free()


func test_display_queued_cue_selects_life_state_for_capture_only() -> void:
	var hud: FullHud = _add_hud(1920)
	hud.presenter.cue_queue.append(hud.presenter.format_cue(_combat_cue("First target")))
	hud.presenter.cue_queue.append(hud.presenter.format_cue(_combat_cue("Second target")))
	hud.presenter.cue_queue.append(hud.presenter.format_cue({
		"kind": "life_state",
		"actor": {"actor_id": "player", "name": "Wayfarer", "kind": "player"},
		"from": "alive",
		"to": "ghost",
	}))
	_support.expect(hud.display_queued_cue("life_state"), "capture seam finds the newest queued life-state cue")
	_support.expect(hud.cue_banner.visible and "alive to ghost" in hud.cue_label.text, "capture seam immediately presents the contemporaneous life-state cue")
	_support.expect(not hud.presenter.cue_queue.any(func(entry: Dictionary) -> bool: return entry.get("kind") == "life_state"), "selected life-state cue is removed from the pending queue")
	_support.expect(not hud.display_queued_cue("resurrection"), "capture seam returns false when the pending queue has no matching cue")
	hud.presenter.cue_queue.clear()
	_support.expect(not hud.display_queued_cue("life_state"), "capture seam returns false for an empty queue")
	hud.free()


func test_logout_reset_clears_every_frame_and_session_fact() -> void:
	var hud: FullHud = _add_hud(1920)
	var frame: Dictionary = _frame()
	hud.present_frame(frame)
	hud.set_selected_actor("actor-z")
	hud.presenter.consume_social_message({"message_id": "logout-message", "scope": {"kind": "say"}, "sender_name": "Visible Ally", "body": "Before logout"})
	hud.present_observed_events([{"kind": "feedback", "cue": _combat_cue("Before logout target")}], "event:logout:1")
	hud.presenter.discard()
	hud.reset_presentation_surface()
	_support.expect_equal(hud.hp_label.text, "HP —/—", "logout reset clears exact HP")
	_support.expect_equal(hud.stamina_label.text, "STAMINA —/—", "logout reset clears exact stamina")
	_support.expect_equal(hud.mp_label.text, "MP —/—", "logout reset clears exact MP")
	_support.expect_equal(hud.selected_actor_id(), "", "logout reset clears local actor selection")
	_support.expect_equal(hud.spell_palette.ready_control_count(), 0, "logout reset clears ready spells")
	_support.expect("Not currently grouped" in hud.group_text(), "logout reset clears group facts")
	_support.expect(hud.focus_button_texts() == ["No other visible actors"], "logout reset clears focus facts")
	_support.expect_equal(hud.page_recipient.item_count, 0, "logout reset clears page recipients")
	_support.expect_equal(hud.chat_log.text, "No messages this session.", "logout reset clears chat")
	_support.expect_equal(hud.feedback_log.text, "No feedback this session.", "logout reset clears feedback")
	_support.expect(not hud.cue_banner.visible, "logout reset clears the active cue")
	hud.free()


func _add_hud(width: int) -> FullHud:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	tree.root.size = Vector2i(width, 1080 if width >= 1500 else 720)
	var hud: FullHud = (load("res://presentation/FullHud.tscn") as PackedScene).instantiate() as FullHud
	hud.set_anchors_preset(Control.PRESET_TOP_LEFT)
	hud.position = Vector2.ZERO
	hud.size = Vector2(width, tree.root.size.y)
	tree.root.add_child(hud)
	hud.apply_responsive_layout(width)
	return hud


func _frame() -> Dictionary:
	return {
		"observer_actor_id": "player",
		"observation_center": _position(0, 0),
		"character": {"resources": {"hp": 7, "max_hp": 12, "peak_hp": 12, "mp": 15, "max_mp": 10, "stamina": -1, "max_stamina": 10}},
		"actors": [
			{"actor_id": "player", "character_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "name": "Wayfarer", "kind": "player", "position": _position(0, 0), "life_state": "alive", "hp": 7, "max_hp": 12, "attack_safety": "invalid"},
			{"actor_id": "actor-a", "character_id": null, "name": "Quartermaster", "kind": "npc", "position": _position(-1, 0), "life_state": "alive", "hp": 8, "max_hp": 8, "attack_safety": "protected"},
			{"actor_id": "actor-z", "character_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", "name": "Visible Ally", "kind": "player", "position": _position(1, 1), "life_state": "alive", "hp": 9, "max_hp": 10, "attack_safety": "protected"},
			{"actor_id": "actor-b", "character_id": null, "name": "Mireling", "kind": "monster", "position": _position(2, 0), "life_state": "alive", "hp": 4, "max_hp": 7, "attack_safety": "open_hostile"},
		],
		"social": {"character_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "group": _group(), "incoming_invitations": [], "outgoing_invitations": [], "following_character_id": null, "pages_enabled": true, "blocked_character_ids": []},
	}


func _group() -> Dictionary:
	return {
		"group_id": "1", "leader_character_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
		"members": [
			{"character_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", "joined_order": "2", "membership_epoch": "1", "connected": true, "absent_since": null},
			{"character_id": "dddddddd-dddd-4ddd-8ddd-dddddddddddd", "joined_order": "3", "membership_epoch": "1", "connected": false, "absent_since": "9"},
			{"character_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "joined_order": "1", "membership_epoch": "1", "connected": true, "absent_since": null},
		],
	}


func _position(x: int, y: int) -> Dictionary:
	return {"realm": "synthetic", "level": "surface", "position": {"x": x, "y": y}}


func _combat_cue(target_name: String) -> Dictionary:
	return {"kind": "physical_combat", "source": null, "target": {"actor_id": target_name.to_lower().replace(" ", "_"), "name": target_name, "kind": "monster"}, "location": null, "mode": "fight", "outcome": {"kind": "missed"}}


func _resource_cue() -> Dictionary:
	return {"kind": "resource", "actor": {"actor_id": "player", "name": "Wayfarer", "kind": "player"}, "resource": "stamina", "reason": "regenerated", "amount": 1, "current": 8, "maximum": 10}


func _spell_row(spell_id: String, enabled: bool, blocked_reason: Variant) -> Dictionary:
	return {
		"spell_id": spell_id,
		"spell_name": spell_id.replace("_", " ").capitalize(),
		"casting_method": "direct",
		"cast_class": "character",
		"target_kind": "actor",
		"mp_cost": 1,
		"stamina_cost": null,
		"hostile_act": true,
		"town_law_violation": false,
		"warm": {"enabled": false, "blocked_reason": "spell_casts_directly", "requires_target_selection": false, "intent": null},
		"cast": {"enabled": enabled, "blocked_reason": blocked_reason, "requires_target_selection": true, "intent": null},
	}
