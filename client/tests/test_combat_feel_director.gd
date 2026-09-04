extends RefCounted

var _support: TestSupport


func test_profile_owns_the_exact_nine_fj_timing_values() -> void:
	var profile: CombatFeelProfile = load("res://presentation/combat_feel_profile.tres") as CombatFeelProfile
	_support.expect(profile != null, "the combat-feel profile loads")
	_support.expect_equal([
		profile.double_activation_window_msec,
		profile.melee_wind_up_msec,
		profile.melee_minimum_payoff_msec,
		profile.ranged_release_msec,
		profile.ranged_minimum_payoff_msec,
		profile.spell_chant_msec,
		profile.spell_release_msec,
		profile.spell_minimum_payoff_msec,
		profile.visual_tail_cap_msec,
	], [420, 180, 280, 160, 260, 600, 180, 300, 700], "the profile is the single source of truth for the locked timing values")


func test_melee_payoff_holds_correlated_feedback_without_holding_state() -> void:
	var fixture: Dictionary = _director()
	var director: CombatFeelDirector = fixture["director"]
	var player: AudioCuePlayer = fixture["player"]
	var deferred: Array[Dictionary] = []
	var flashes: Array[String] = []
	director.deferred_feedback_ready.connect(func(entry: Dictionary) -> void: deferred.append(entry))
	director.visual_flash_requested.connect(func(kind: String) -> void: flashes.append(kind))
	director.note_command_installed({"kind": "physical_attack", "mode": "fight", "target_actor_id": "actor-a"}, 100)
	director.set_test_clock(100)
	var entry: Dictionary = {"kind": "physical_combat", "text": "[HIT] Synthetic", "high_salience": true}
	var cue: Dictionary = {
		"kind": "physical_combat",
		"mode": "fight",
		"target": {"actor_id": "actor-a"},
		"outcome": {"kind": "hit"},
	}
	_support.expect(not director.gate_feedback_entry(entry, cue, "event:1"), "correlated hit presentation is held until the exact melee payoff")
	_support.expect_equal(director.held_feedback_count(), 1, "only presentation is held")
	director.advance(379)
	_support.expect(deferred.is_empty(), "feedback stays held one millisecond before the payoff")
	director.advance(380)
	_support.expect_equal(deferred, [entry], "feedback releases exactly at the 280ms melee payoff")
	_support.expect_equal(flashes, ["physical_combat"], "the synchronized visual flash is emitted with the released text")
	_support.expect_equal((player.play_history[-1] as Dictionary).get("role"), "combat_body_impact", "a hit routes to the body-impact role")
	_support.expect(not director.gate_feedback_entry(entry, cue, "event:1"), "the same stable event identity cannot present or sound twice")
	_support.expect_equal(director.held_feedback_count(), 0, "duplicate suppression does not create a second hold")
	_cleanup(fixture)


func test_ranged_release_and_spell_chant_release_use_exact_boundaries() -> void:
	var fixture: Dictionary = _director()
	var director: CombatFeelDirector = fixture["director"]
	var player: AudioCuePlayer = fixture["player"]
	director.note_command_installed({"kind": "physical_attack", "mode": "shoot", "target_actor_id": "actor-b"}, 1000)
	director.advance(1159)
	_support.expect_equal(_history_roles(player), [], "ranged release does not sound early")
	director.advance(1160)
	_support.expect_equal(_history_roles(player), ["bow_release"], "Shoot release sounds exactly 160ms after installation")

	var chant_completions: Array[bool] = []
	director.spell_chant_complete.connect(func() -> void: chant_completions.append(true))
	director.begin_spell_prepare("spark", 2000)
	_support.expect_equal(_history_roles(player)[-1], "spell_chant", "Prepare starts the quiet chant immediately")
	director.advance(2599)
	_support.expect(chant_completions.is_empty(), "the chant remains active before 600ms")
	director.advance(2600)
	_support.expect_equal(chant_completions.size(), 1, "the chant completes at the exact 600ms boundary")
	director.note_command_installed({"kind": "cast_warmed_spell", "spell_id": "spark"}, 3000)
	director.advance(3179)
	_support.expect_equal(_history_roles(player).count("spell_release"), 0, "spell release does not sound early")
	director.advance(3180)
	_support.expect_equal(_history_roles(player).count("spell_release"), 1, "spell release sounds exactly 180ms after installation")
	_cleanup(fixture)


func test_uncorrelated_feedback_is_immediate_and_results_route_exact_audio_roles() -> void:
	var fixture: Dictionary = _director()
	var director: CombatFeelDirector = fixture["director"]
	var player: AudioCuePlayer = fixture["player"]
	director.note_command_installed({"kind": "physical_attack", "mode": "fight", "target_actor_id": "actor-a"}, 0)
	director.set_test_clock(1)
	_support.expect(director.gate_feedback_entry({"kind": "resource"}, {"kind": "resource"}, "remote:1"), "uncorrelated authoritative feedback is never delayed")
	director.note_command_result("physical_attack", false, "reject:1")
	director.advance(1000)
	_support.expect_equal(_history_roles(player), ["ui_reject"], "rejection cancels the scheduled swing instead of sounding a failed attack")
	director.note_command_installed({
		"kind": "move_item",
		"item_instance_id": "item:1",
		"destination": {"kind": "carried", "position": "sack_item_1"},
	}, 1100)
	director.note_command_result("move_item", true, "loot:1")
	_support.expect_equal(_history_roles(player), ["ui_reject"], "an accepted result alone cannot claim that the item moved")
	director.note_authoritative_frame({"carried": {"items": []}})
	_support.expect_equal(_history_roles(player), ["ui_reject"], "a replacement frame without the destination does not play stow")
	director.note_authoritative_frame({"carried": {"items": [{
		"position": "sack_item_1",
		"item": {"item_instance_id": "item:1"},
	}]}})
	_support.expect_equal(_history_roles(player), ["ui_reject", "loot_stow"], "accepted result plus replacement-frame destination confirms stow audio")
	_support.expect(player.set_mix(true, 0), "SFX mute can be enabled without touching presentation")
	director.note_command_installed({
		"kind": "move_item",
		"item_instance_id": "item:2",
		"destination": {"kind": "carried", "position": "right_hand"},
	}, 1200)
	director.note_command_result("move_item", true, "loot:muted")
	director.note_authoritative_frame({"carried": {"items": [{
		"position": "right_hand",
		"item": {"item_instance_id": "item:2"},
	}]}})
	_support.expect_equal((player.play_history[-1] as Dictionary).get("muted"), true, "muted audio still records deterministic routing")
	_cleanup(fixture)


func test_feedback_resolves_inside_the_beat_it_belongs_to() -> void:
	# The profile's payoff windows say how *little* a cue may be held for, so a
	# result never lands before the swing that caused it. What bounds the other
	# end is the beat: a result belongs to the round it resolved on, and showing
	# it after the next beat has been struck attributes it to the wrong one.
	var fixture: Dictionary = _director()
	var director: CombatFeelDirector = fixture["director"]
	var deferred: Array[Dictionary] = []
	director.deferred_feedback_ready.connect(func(entry: Dictionary) -> void: deferred.append(entry))

	# A command installed 120 ms before the beat ends: the 280 ms melee window
	# would carry its payoff 160 ms into the next beat.
	director.set_cooldown_deadline(1120)
	director.note_command_installed({"kind": "physical_attack", "mode": "fight", "target_actor_id": "actor-a"}, 1000)
	director.set_test_clock(1000)
	var entry: Dictionary = {"kind": "physical_combat", "text": "[HIT] Synthetic", "high_salience": true}
	var cue: Dictionary = {
		"kind": "physical_combat",
		"mode": "fight",
		"target": {"actor_id": "actor-a"},
		"outcome": {"kind": "hit"},
	}
	_support.expect(not director.gate_feedback_entry(entry, cue, "event:beat"), "the cue is still held for its minimum")
	director.advance(1119)
	_support.expect(deferred.is_empty(), "and held right up to the end of the beat")
	director.advance(1120)
	_support.expect_equal(deferred, [entry], "but resolves on the beat rather than spilling into the next one")
	_cleanup(fixture)


func test_a_payoff_that_already_fits_inside_the_beat_is_left_exactly_where_it_was() -> void:
	var fixture: Dictionary = _director()
	var director: CombatFeelDirector = fixture["director"]
	var deferred: Array[Dictionary] = []
	director.deferred_feedback_ready.connect(func(entry: Dictionary) -> void: deferred.append(entry))
	director.set_cooldown_deadline(4000)
	director.note_command_installed({"kind": "physical_attack", "mode": "fight", "target_actor_id": "actor-a"}, 1000)
	director.set_test_clock(1000)
	var entry: Dictionary = {"kind": "physical_combat", "text": "[HIT] Synthetic"}
	var cue: Dictionary = {
		"kind": "physical_combat",
		"mode": "fight",
		"target": {"actor_id": "actor-a"},
		"outcome": {"kind": "hit"},
	}
	_support.expect(not director.gate_feedback_entry(entry, cue, "event:fits"), "the cue is held for its minimum")
	director.advance(1279)
	_support.expect(deferred.is_empty(), "the beat does not shorten a window that already fits")
	director.advance(1280)
	_support.expect_equal(deferred, [entry], "the profile's own 280 ms payoff still governs")
	_cleanup(fixture)


func test_with_no_observed_beat_the_profile_windows_stand_alone() -> void:
	var fixture: Dictionary = _director()
	var director: CombatFeelDirector = fixture["director"]
	var player: AudioCuePlayer = fixture["player"]
	director.set_cooldown_deadline(-1)
	director.note_command_installed({"kind": "physical_attack", "mode": "shoot", "target_actor_id": "actor-b"}, 1000)
	director.advance(1159)
	_support.expect_equal(_history_roles(player), [], "an unmeasured beat imposes no deadline of its own")
	director.advance(1160)
	_support.expect_equal(_history_roles(player), ["bow_release"], "and the profile's exact boundary is unchanged")
	_cleanup(fixture)


func _director() -> Dictionary:
	var loader: AudioManifestLoader = AudioManifestLoader.new()
	var player: AudioCuePlayer = AudioCuePlayer.new()
	(Engine.get_main_loop() as SceneTree).root.add_child(player)
	_support.expect(player.configure(loader), "combat-feel test audio loads")
	return {
		"player": player,
		"director": CombatFeelDirector.new(load("res://presentation/combat_feel_profile.tres"), player),
	}


func _cleanup(fixture: Dictionary) -> void:
	(fixture["director"] as CombatFeelDirector).discard()
	(fixture["player"] as AudioCuePlayer).free()


func _history_roles(player: AudioCuePlayer) -> Array[String]:
	var roles: Array[String] = []
	for record: Dictionary in player.play_history:
		roles.append(str(record.get("role", "")))
	return roles
