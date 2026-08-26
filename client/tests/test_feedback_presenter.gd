extends RefCounted

var _support: TestSupport


func test_all_feedback_families_format_with_category_icon_and_plain_text() -> void:
	var envelope: Dictionary = _fixture_envelope("accept_feedback_cue_inventory")
	var presenter: FeedbackPresenter = FeedbackPresenter.new()
	var result: Dictionary = presenter.consume_server_envelope(envelope, "connection-a")
	var entries: Array = result["feedback"]
	_support.expect_equal(entries.size(), 16, "one command disposition plus all fifteen cue families render")
	var kinds: Array[String] = []
	for value: Variant in entries:
		var entry: Dictionary = value as Dictionary
		_support.expect(str(entry.get("icon", "")).begins_with("["), "every entry has a non-color icon token")
		_support.expect(not str(entry.get("text", "")).is_empty(), "every entry has readable text")
		_support.expect(str(entry.get("category", "")) in ["combat", "quest", "system"], "every entry has a finite filter category")
		if entry.get("kind") != "command_result":
			kinds.append(str(entry.get("kind")))
	kinds.sort()
	var expected: Array[String] = ["actor_effect", "corpse", "defeat", "effect_damage", "life_state", "npc_message", "physical_combat", "quest", "resource", "resurrection", "spell_impact", "spell_lifecycle", "tile_effect", "transaction", "weapon_fumbled"]
	expected.sort()
	_support.expect_equal(kinds, expected, "the formatter covers the exact Pre-FA cue family inventory")
	_support.expect("Unseen source" in str(entries[1]["text"]), "required-nullable source stays explicit rather than inferred")
	_support.expect("Welcome, traveler." in str(entries[11]["text"]), "NPC content remains literal plain text")


func test_result_update_messages_and_dispositions_deduplicate_by_stable_identity() -> void:
	var envelope: Dictionary = _fixture_envelope("accept_feedback_cue_inventory")
	var presenter: FeedbackPresenter = FeedbackPresenter.new()
	_support.expect_equal((presenter.consume_server_envelope(envelope, "connection-a")["feedback"] as Array).size(), 16, "first result renders")
	var update: Dictionary = {"kind": "state_update", "server_sequence": envelope["server_sequence"], "events": envelope["events"]}
	_support.expect_equal((presenter.consume_server_envelope(update, "connection-a")["feedback"] as Array).size(), 0, "matching state update does not double-present result events")
	_support.expect_equal((presenter.consume_server_envelope(envelope, "connection-a")["feedback"] as Array).size(), 0, "replayed command result is wholly deduplicated")
	var social: Dictionary = _fixture_envelope("accept_social_message_shout")
	_support.expect(not presenter.consume_social_message(social).is_empty(), "first social delivery renders")
	_support.expect(presenter.consume_social_message(social).is_empty(), "stable message ID deduplicates delivery")
	var disposition: Dictionary = {"kind": "message_result", "message_id": social["message_id"], "disposition": "accepted"}
	var result: Dictionary = presenter.consume_message_result(disposition)
	_support.expect("Accepted for routing" in str(result.get("text", "")) and "delivery is not confirmed" in str(result.get("text", "")), "accepted disposition makes no delivery claim")
	_support.expect(presenter.consume_message_result(disposition).is_empty(), "message disposition renders once")


func test_defeat_share_pairs_with_matching_authoritative_total_once_in_either_envelope_order() -> void:
	var command: Dictionary = _fixture_envelope("accept_command_result")
	var update: Dictionary = _fixture_envelope("accept_state_update")
	update["frame"]["character"]["progression"]["experience"] = "5"
	var presenter: FeedbackPresenter = FeedbackPresenter.new()

	var command_result: Dictionary = presenter.consume_server_envelope(command, "connection-a")
	_support.expect_equal(_experience_entries(command_result).size(), 0, "share alone never invents an XP total")
	var update_result: Dictionary = presenter.consume_server_envelope(update, "connection-a")
	var entries: Array[Dictionary] = _experience_entries(update_result)
	_support.expect_equal(entries.size(), 1, "matching state frame completes one XP pair")
	_support.expect_equal(entries[0], {
		"category": "system",
		"icon": "[XP]",
		"text": "[XP] +5 XP; total 5.",
		"high_salience": false,
		"kind": "experience",
	}, "XP presentation is exact plain text with delta and authoritative total")
	_support.expect_equal(_experience_entries(presenter.consume_server_envelope(update, "connection-a")).size(), 0, "state replay does not duplicate XP")
	_support.expect_equal(_experience_entries(presenter.consume_server_envelope(command, "connection-a")).size(), 0, "command replay does not duplicate XP")

	var update_first: FeedbackPresenter = FeedbackPresenter.new()
	_support.expect_equal(_experience_entries(update_first.consume_server_envelope(update, "connection-a")).size(), 1, "state update can supply both facts first")
	_support.expect_equal(_experience_entries(update_first.consume_server_envelope(command, "connection-a")).size(), 0, "later matching result stays deduplicated")


func test_defeat_share_pairing_is_bounded_character_exact_and_cleared_on_discard() -> void:
	var presenter: FeedbackPresenter = FeedbackPresenter.new()
	for index: int in range(FeedbackPresenter.MAX_PENDING_XP_PAIRS + 1):
		presenter.consume_server_envelope(_xp_command(index + 1), "connection-a")
	_support.expect_equal(presenter._pending_xp_pairs.size(), FeedbackPresenter.MAX_PENDING_XP_PAIRS, "pending XP pairs are bounded")
	_support.expect_equal(_experience_entries(presenter.consume_server_envelope(_xp_update(1, "5"), "connection-a")).size(), 0, "evicted oldest share cannot be reconstructed from a frame")
	_support.expect_equal(_experience_entries(presenter.consume_server_envelope(_xp_update(FeedbackPresenter.MAX_PENDING_XP_PAIRS + 1, "10"), "connection-a")).size(), 1, "newest retained share still pairs with an advancing authoritative total")

	var mismatched: FeedbackPresenter = FeedbackPresenter.new()
	mismatched.consume_server_envelope(_xp_command(7), "connection-a")
	var wrong_character: Dictionary = _xp_update(7, "5")
	wrong_character["frame"]["social"]["character_id"] = "22222222-2222-4222-8222-222222222222"
	_support.expect_equal(_experience_entries(mismatched.consume_server_envelope(wrong_character, "connection-a")).size(), 0, "frame character must match the rewarded character")

	var discarded: FeedbackPresenter = FeedbackPresenter.new()
	discarded.consume_server_envelope(_xp_command(9), "connection-a")
	discarded.discard()
	_support.expect_equal(_experience_entries(discarded.consume_server_envelope(_xp_update(9, "5"), "connection-a")).size(), 0, "discard clears pending pairing state")

	var missing_total: FeedbackPresenter = FeedbackPresenter.new()
	missing_total.consume_server_envelope(_xp_command(11), "connection-a")
	var incomplete: Dictionary = _xp_update(11, "5")
	incomplete["frame"]["character"]["progression"].erase("experience")
	_support.expect_equal(_experience_entries(missing_total.consume_server_envelope(incomplete, "connection-a")).size(), 0, "missing authoritative total produces no XP line")

	var unchanged_total: FeedbackPresenter = FeedbackPresenter.new()
	unchanged_total.consume_server_envelope(_xp_welcome("0"), "connection-a")
	unchanged_total.consume_server_envelope(_xp_command(13), "connection-a")
	_support.expect_equal(_experience_entries(unchanged_total.consume_server_envelope(_xp_update(13, "0"), "connection-a")).size(), 0, "nonadvancing frame is not presented as the result of a positive XP share")

	var reconnected: FeedbackPresenter = FeedbackPresenter.new()
	reconnected.consume_server_envelope(_xp_command(15), "connection-a")
	reconnected.consume_server_envelope(_xp_welcome("0"), "connection-b")
	_support.expect(reconnected._pending_xp_pairs.is_empty(), "a new connection clears unmatched prior-connection XP state")


func test_scrollback_and_visual_cue_queue_are_bounded_and_discardable() -> void:
	var presenter: FeedbackPresenter = FeedbackPresenter.new()
	for index: int in range(205):
		presenter.consume_social_message({"message_id": "message-%d" % index, "scope": {"kind": "say"}, "sender_name": "Speaker", "body": "Line %d" % index})
		presenter.consume_observed_events([{"kind": "feedback", "cue": _resource_cue(index)}], "resource-%d" % index)
	_support.expect_equal(presenter.chat_entries.size(), FeedbackPresenter.MAX_CHAT_ENTRIES, "chat keeps only the newest 200 entries")
	_support.expect_equal(presenter.feedback_entries.size(), FeedbackPresenter.MAX_FEEDBACK_ENTRIES, "feedback keeps only the newest 200 entries")
	for index: int in range(12):
		presenter.consume_observed_events([{"kind": "feedback", "cue": _defeat_cue(index)}], "defeat-%d" % index)
	_support.expect_equal(presenter.cue_queue.size(), FeedbackPresenter.MAX_CUE_QUEUE, "high-salience visual queue keeps only eight pending cues")
	presenter.discard()
	_support.expect(presenter.chat_entries.is_empty() and presenter.feedback_entries.is_empty() and presenter.cue_queue.is_empty(), "discard clears all transient history and cues")
	_support.expect(not presenter.consume_social_message({"message_id": "message-204", "scope": {"kind": "say"}, "sender_name": "Speaker", "body": "Fresh"}).is_empty(), "discard clears deduplication identities")


func test_nested_feedback_outcomes_remain_meaningful_and_exhaustive() -> void:
	var presenter: FeedbackPresenter = FeedbackPresenter.new()
	var physical_outcomes: Array[Dictionary] = [
		{"kind": "hit", "damage": 4, "armor_reduction": 1, "wound_before": "unhurt", "wound_after": "wounded", "target_hp": 6},
		{"kind": "missed"}, {"kind": "blocked"}, {"kind": "no_sight"},
		{"kind": "not_ready", "current_time": "9", "ready_at": "10"},
	]
	for outcome: Dictionary in physical_outcomes:
		var entry: Dictionary = presenter.format_cue({"kind": "physical_combat", "source": null, "target": _actor("Target"), "location": null, "mode": "fight", "outcome": outcome})
		_support.expect(not str(entry["text"]).is_empty() and entry["category"] == "combat", "physical outcome " + str(outcome["kind"]) + " is meaningful")
	var spell_states: Array[Dictionary] = [
		{"kind": "warmed", "warmed_at": "1", "ready_at": "2"}, {"kind": "ready", "ready_at": "2"},
		{"kind": "cast", "mp_cost": 2, "stamina_cost": null}, {"kind": "fizzled", "reason": "damage"},
		{"kind": "failed", "reason": "invalid_path", "mp_cost": null, "stamina_cost": 1},
	]
	for state: Dictionary in spell_states:
		var entry: Dictionary = presenter.format_cue({"kind": "spell_lifecycle", "actor": _actor("Caster"), "spell_id": "spark", "spell_name": "Spark", "state": state})
		_support.expect(not str(entry["text"]).is_empty() and "Spark" in str(entry["text"]), "spell state " + str(state["kind"]) + " is meaningful")
	for change: Dictionary in [{"kind": "applied", "remaining_rounds": 3}, {"kind": "ticked", "remaining_rounds": null}, {"kind": "expired"}, {"kind": "removed"}]:
		var entry: Dictionary = presenter.format_cue({"kind": "actor_effect", "actor": _actor("Target"), "location": _location(), "effect_id": "effect:1", "effect_kind": "poison", "change": change})
		_support.expect(not str(entry["text"]).is_empty(), "effect change " + str(change["kind"]) + " is meaningful")


func _fixture_envelope(case_id: String) -> Dictionary:
	var file: FileAccess = FileAccess.open(TestSupport.wire_fixture_path("server_envelope"), FileAccess.READ)
	var document: Dictionary = JSON.parse_string(file.get_as_text()) as Dictionary
	for value: Variant in document.get("cases", []):
		var fixture_case: Dictionary = value as Dictionary
		if fixture_case.get("case_id") == case_id:
			return JSON.parse_string(fixture_case["input_utf8"]) as Dictionary
	return {}


func _experience_entries(result: Dictionary) -> Array[Dictionary]:
	var entries: Array[Dictionary] = []
	for value: Variant in result.get("feedback", []):
		var entry: Dictionary = value as Dictionary
		if entry.get("kind") == "experience":
			entries.append(entry)
	return entries


func _xp_command(sequence: int) -> Dictionary:
	return {
		"kind": "command_result",
		"command_id": "command-%d" % sequence,
		"disposition": {"kind": "accepted"},
		"server_sequence": str(sequence),
		"events": [{
			"kind": "defeat_reward_share",
			"character_id": "11111111-1111-4111-8111-111111111111",
			"amount": 5,
		}],
	}


func _xp_update(sequence: int, total: String) -> Dictionary:
	return {
		"kind": "state_update",
		"server_sequence": str(sequence),
		"events": [],
		"frame": {
			"social": {"character_id": "11111111-1111-4111-8111-111111111111"},
			"character": {"progression": {"experience": total}},
		},
	}


func _xp_welcome(total: String) -> Dictionary:
	return {
		"kind": "server_welcome",
		"frame": {
			"social": {"character_id": "11111111-1111-4111-8111-111111111111"},
			"character": {"progression": {"experience": total}},
		},
	}


func _resource_cue(index: int) -> Dictionary:
	return {"kind": "resource", "actor": _actor("Player"), "resource": "stamina", "reason": "regenerated", "amount": 1, "current": index, "maximum": 300}


func _defeat_cue(index: int) -> Dictionary:
	return {"kind": "defeat", "actor": _actor("Target %d" % index), "location": _location(), "cause": "physical", "credited_source": null}


func _actor(name: String) -> Dictionary:
	return {"actor_id": name.to_lower().replace(" ", "_"), "name": name, "kind": "monster"}


func _location() -> Dictionary:
	return {"realm": "synthetic", "level": "surface", "position": {"x": 1, "y": 2}}
