extends RefCounted

var _support: TestSupport


func test_projected_row_counts_create_exact_controls_without_padding_or_cap() -> void:
	var palette: SpellPalette = _add_palette()
	for count: int in [0, 1, 3, 9, 10, 45]:
		palette.present_rows(_rows(count))
		_support.expect_equal(palette.ready_control_count(), count, "%d projected rows create exactly %d controls" % [count, count])
		_support.expect_equal(palette.spell_ids().size(), count, "spell identity count follows projection")
		_support.expect(palette.buttons().all(func(button: Button) -> bool: return "Empty" not in button.text), "ready collection has no empty padding")
	palette.free()


func test_selection_readiness_and_activation_are_semantic() -> void:
	var palette: SpellPalette = _add_palette()
	var rows: Array[Dictionary] = _rows(3)
	rows[1]["cast"]["enabled"] = false
	rows[1]["cast"]["blocked_reason"] = "not_ready"
	var activated: Array[String] = []
	palette.spell_primary_activated.connect(func(spell_id: String) -> void: activated.append(spell_id))
	palette.present_rows(rows, "spell_1", "spell_1")
	_support.expect("◈ " in palette.buttons()[1].text and "Not Ready" in palette.buttons()[1].text, "Prepare marker and exact reason are textual")
	palette.buttons()[1].pressed.emit()
	_support.expect_equal(activated, ["spell_1"], "button emits stable spell identity rather than a slot number")
	_support.expect(palette.buttons()[1].focus_mode != Control.FOCUS_NONE, "blocked spell row remains focusable for explanation")
	palette.free()


func test_pressed_activation_defers_and_coalesces_selection_and_frame_rebuilds() -> void:
	var palette: SpellPalette = _add_palette()
	palette.present_rows(_rows(1))
	var emitting_button: Button = palette.buttons()[0]
	emitting_button.grab_focus()
	var emission_facts: Dictionary = {}
	palette.spell_primary_activated.connect(func(_spell_id: String) -> void:
		palette.select_spell("spell_0")
		palette.present_rows(_rows(3), "spell_0", "spell_0")
		palette.present_rows(_rows(2), "spell_1")
		emission_facts["button_valid"] = is_instance_valid(emitting_button)
		emission_facts["button_parent"] = emitting_button.get_parent()
		emission_facts["control_count"] = palette.ready_control_count()
		emission_facts["rebuild_queued"] = palette.get("_rebuild_queued")
	)
	emitting_button.pressed.emit()
	_support.expect_equal(emission_facts.get("button_valid"), true, "pressed emitter remains valid throughout outward spell activation")
	_support.expect_equal(emission_facts.get("button_parent"), palette.spell_grid, "pressed emitter remains parented while its signal is live")
	_support.expect_equal(emission_facts.get("control_count"), 1, "neither selection nor frame presentation frees the live emitter synchronously")
	_support.expect_equal(emission_facts.get("rebuild_queued"), true, "selection and frame presentation coalesce behind one deferred rebuild")
	_support.expect_equal(palette.get("_button_signal_depth"), 0, "button signal depth returns to zero after outward activation")
	_support.expect_equal(palette.get_viewport().gui_get_focus_owner(), emitting_button, "focus remains on the live emitter until the deferred flush")
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect_equal(palette.ready_control_count(), 2, "the coalesced rebuild presents only the latest projected rows")
	_support.expect_equal(palette.spell_ids(), ["spell_0", "spell_1"], "deferred lifecycle preserves the exact projected-row contract")
	_support.expect(not is_instance_valid(emitting_button), "the old emitter is released only after its pressed emission has returned")
	var focus_owner: Control = palette.get_viewport().gui_get_focus_owner()
	_support.expect(
		focus_owner != null and focus_owner.get_meta("spell_id", "") == "spell_0",
		"deferred rebuild restores keyboard/controller focus to the matching spell ID",
	)
	palette.free()


func _add_palette() -> SpellPalette:
	var palette: SpellPalette = (load("res://presentation/SpellPalette.tscn") as PackedScene).instantiate() as SpellPalette
	(Engine.get_main_loop() as SceneTree).root.add_child(palette)
	return palette


func _rows(count: int) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	for index: int in range(count):
		result.append({
			"spell_id": "spell_%d" % index,
			"spell_name": "Spell %d" % index,
			"casting_method": "direct",
			"cast_class": "character",
			"target_kind": "actor",
			"mp_cost": 1,
			"stamina_cost": null,
			"cast": {"enabled": true, "blocked_reason": null, "requires_target_selection": true, "intent": null},
			"warm": {"enabled": false, "blocked_reason": "spell_casts_directly", "requires_target_selection": false, "intent": null},
		})
	return result
