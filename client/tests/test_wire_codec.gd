extends RefCounted

## The one shared corpus, read where it lives. `tme-protocol` asserts the same
## inventory from the Rust side, so a fixture added, removed, or renamed on
## either side of the wire fails on both.
const DECODERS: Array[String] = [
	"decimal_u64", "decimal_i64", "login_request_v1", "logout_request_v1",
	"character_select_request_v1", "socket_ticket_request_v1",
	"forgive_player_kill_mark_request_v1", "session_bootstrap_v1", "character_selection_v1",
	"socket_ticket_v1", "forgive_player_kill_mark_result_v1", "control_error_v1",
	"client_hello_envelope", "client_command_envelope", "server_envelope",
]

var _support: TestSupport


func test_protocol_constants_are_pinned() -> void:
	_support.expect_equal(WireCodec.PROTOCOL_MAJOR, 1, "protocol major")
	_support.expect_equal(WireCodec.PROTOCOL_MINOR, 8, "protocol minor")
	_support.expect_equal(WireCodec.CONTROL_API_VERSION, 3, "control API version")
	_support.expect_equal(WireCodec.WEBSOCKET_SUBPROTOCOL, "tme.v1", "WebSocket subprotocol")
	_support.expect_equal(WireCodec.MAX_INPUT_BYTES, 32768, "protocol input bytes")
	_support.expect_equal(WireCodec.MAX_JSON_NESTING, 32, "protocol nesting")
	_support.expect_equal(WireCodec.MAX_SUPPORTED_MINORS, 8, "minor list bound")
	_support.expect_equal(WireCodec.MAX_MOVE_PATH_STEPS, 3, "move path bound")
	_support.expect_equal(WireCodec.MAX_SERVER_ENVELOPE_BYTES, 262144, "server envelope bytes")
	_support.expect_equal(WireCodec.MAX_SOCIAL_BODY_SCALARS, 280, "social scalar bound")
	_support.expect_equal(WireCodec.MAX_SOCIAL_BODY_BYTES, 1024, "social byte bound")
	_support.expect_equal(WireCodec.MAX_FEEDBACK_TRANSACTION_COSTS, 64, "feedback cost receipt bound")
	_support.expect_equal(WireCodec.MAX_FEEDBACK_TRANSACTION_REWARDS, 64, "feedback reward receipt bound")
	_support.expect_equal(WireCodec.MAX_CONTROL_INPUT_BYTES, 16384, "control input bytes")
	_support.expect_equal(WireCodec.MAX_CONTROL_JSON_NESTING, 16, "control nesting")


func test_fixture_metadata_and_accept_reject_labels_are_strict() -> void:
	var actual_files: PackedStringArray = DirAccess.get_files_at(TestSupport.wire_fixture_root())
	actual_files.sort()
	var expected_files: PackedStringArray = PackedStringArray()
	for decoder: String in DECODERS: expected_files.append(decoder + ".json")
	expected_files.sort()
	_support.expect_equal(actual_files, expected_files, "fixture file inventory must be exact")
	var pattern: RegEx = RegEx.create_from_string("^[a-z0-9]+(?:_[a-z0-9]+)*$")
	for decoder: String in DECODERS:
		var fixture: Dictionary = _load_fixture(decoder)
		_support.expect(not fixture.is_empty(), decoder + " metadata must be readable")
		if fixture.is_empty(): continue
		_support.expect(_keys_equal(fixture, ["schema_version", "decoder", "cases"]), decoder + " top-level keys")
		_support.expect_equal(fixture.get("schema_version"), 1, decoder + " schema version")
		_support.expect_equal(fixture.get("decoder"), decoder, decoder + " filename stem")
		_support.expect(typeof(fixture.get("cases")) == TYPE_ARRAY and not fixture["cases"].is_empty(), decoder + " nonempty cases")
		var case_ids: Dictionary = {}
		var reasons: Dictionary = {}
		for case: Variant in fixture["cases"]:
			_support.expect(typeof(case) == TYPE_DICTIONARY, decoder + " case object")
			if typeof(case) != TYPE_DICTIONARY: continue
			var has_utf8: bool = case.has("input_utf8")
			var has_hex: bool = case.has("input_hex")
			var expected_keys: Array = ["case_id", "expect", "reason", "input_utf8" if has_utf8 else "input_hex"]
			_support.expect(_keys_equal(case, expected_keys), decoder + " exact case keys")
			_support.expect(has_utf8 != has_hex, decoder + " exactly one input source")
			_support.expect(pattern.search(str(case.get("case_id", ""))) != null, decoder + " lower-snake case ID")
			_support.expect(pattern.search(str(case.get("reason", ""))) != null, decoder + " lower-snake reason")
			_support.expect(case.get("expect") in ["accept", "reject"], decoder + " finite expectation")
			_support.expect(not case_ids.has(case.get("case_id")), decoder + " unique case ID")
			_support.expect(not reasons.has(case.get("reason")), decoder + " unique reason")
			case_ids[case.get("case_id")] = true
			reasons[case.get("reason")] = true
			if has_hex: _support.expect(_valid_hex(case["input_hex"]), decoder + " lowercase even input hex")


func test_shared_fixture_verdicts_match_labels() -> void:
	var codec: WireCodec = WireCodec.new()
	var verdict_count: int = 0
	for decoder: String in DECODERS:
		var fixture: Dictionary = _load_fixture(decoder)
		for case: Dictionary in fixture["cases"]:
			var input: PackedByteArray = case["input_utf8"].to_utf8_buffer() if case.has("input_utf8") else _decode_hex(case["input_hex"])
			var result: Dictionary = codec.decode_named(decoder, input)
			_support.expect_equal(result["ok"], case["expect"] == "accept", decoder + "/" + case["case_id"] + " verdict")
			verdict_count += 1
	_support.expect(verdict_count > 0, "shared fixtures must execute")


func test_decimal_i64_strings_never_coerce_through_float() -> void:
	var codec: WireCodec = WireCodec.new()
	for text: String in ["9007199254740991", "9007199254740992", "9007199254740993", "9223372036854775807", "-9223372036854775808", "0"]:
		var result: Dictionary = codec.decode_decimal_i64(JSON.stringify(text).to_utf8_buffer())
		_support.expect_accept(result, text + " must decode")
		_support.expect_equal(typeof(result["value"]), TYPE_STRING, text + " remains a string")
		_support.expect_equal(JSON.stringify(result["value"]), JSON.stringify(text), text + " re-encodes byte-identically")


func test_decimal_i64_rejects_numeric_noncanonical_and_out_of_range() -> void:
	var codec: WireCodec = WireCodec.new()
	var fixture: Dictionary = _load_fixture("decimal_i64")
	for case: Dictionary in fixture["cases"]:
		if case["expect"] == "reject":
			_support.expect_reject(codec.decode_decimal_i64(case["input_utf8"].to_utf8_buffer()), case["case_id"])
	var command_fixture: Dictionary = _load_fixture("client_command_envelope")
	for case: Dictionary in command_fixture["cases"]:
		if case["case_id"] in ["reject_zero_exact_gold", "reject_negative_exact_gold", "reject_numeric_exact_gold"]:
			_support.expect_reject(codec.decode_client_command_envelope(case["input_utf8"].to_utf8_buffer()), case["case_id"])
	var server_fixture: Dictionary = _load_fixture("server_envelope")
	for case: Dictionary in server_fixture["cases"]:
		if case["case_id"] in ["reject_numeric_wide_gold", "reject_negative_observer_gold", "reject_negative_gold_pile"]:
			_support.expect_reject(codec.decode_server_envelope(case["input_utf8"].to_utf8_buffer()), case["case_id"])


func test_bootstrap_decodes_exact_current_shape() -> void:
	var codec: WireCodec = WireCodec.new()
	var fixture: Dictionary = _load_fixture("session_bootstrap_v1")
	for case: Dictionary in fixture["cases"]:
		var result: Dictionary = codec.decode_session_bootstrap_v1(case["input_utf8"].to_utf8_buffer())
		_support.expect_equal(result["ok"], case["expect"] == "accept", case["case_id"])
		if result["ok"]:
			_support.expect_equal((result["value"] as Dictionary).keys(), ["control_api_version", "account", "session", "csrf_token", "characters", "selected_character_id", "player_kill_marks"], "bootstrap normalized key order")
			for entry: Dictionary in result["value"]["characters"]:
				_support.expect(_keys_equal(entry, ["character_id", "slot", "display_name"]), "character summary carries no world identity")


func test_all_current_control_and_wire_variants_are_covered() -> void:
	var command_fixture: Dictionary = _load_fixture("client_command_envelope")
	var command_cases: Dictionary = {}
	for case: Dictionary in command_fixture["cases"]: command_cases[case["case_id"]] = true
	for intent: String in ["move_path", "traverse", "open", "close", "inspect", "wait", "physical_attack", "nock", "unload_bow", "warm_spell", "cast_spell", "cast_warmed_spell", "fizzle_warmed_spell", "search_corpse", "move_item", "move_gold", "drink_item", "clear_self_defense", "invite", "accept_invite", "decline_invite", "cancel_invite", "leave_group", "remove_member", "disband_group", "transfer_leadership", "begin_follow", "end_follow", "set_pages_enabled", "block", "unblock", "offer_item", "accept_item_offer", "refuse_item_offer", "withdraw_item_offer"]:
		_support.expect(command_cases.has("accept_" + intent), "positive fixture for intent " + intent)
	_support.expect(command_cases.has("accept_path_preview"), "positive path preview request fixture")
	for scope: String in ["say", "shout", "group", "page"]: _support.expect(command_cases.has("accept_social_" + scope), "positive social scope " + scope)
	var server_fixture: Dictionary = _load_fixture("server_envelope")
	var server_cases: Dictionary = {}
	for case: Dictionary in server_fixture["cases"]: server_cases[case["case_id"]] = true
	for envelope: String in ["server_welcome", "command_result", "state_update"]: _support.expect(server_cases.has("accept_" + envelope), "positive server envelope " + envelope)
	_support.expect(server_cases.has("accept_social_message_say"), "positive social server envelope")
	_support.expect(server_cases.has("accept_message_result_accepted"), "positive message result")
	_support.expect(server_cases.has("accept_server_draining_shutdown"), "positive draining envelope")
	_support.expect(server_cases.has("accept_error_unavailable"), "positive error envelope")
	_support.expect(server_cases.has("accept_path_preview_result"), "positive path preview result fixture")


func test_path_preview_burden_total_is_exact_without_float_coercion() -> void:
	var codec: WireCodec = WireCodec.new()
	var fixture: Dictionary = _load_fixture("server_envelope")
	var accepted: Dictionary = {}
	for case: Dictionary in fixture["cases"]:
		if case["case_id"] == "accept_path_preview_result":
			accepted = codec.decode_server_envelope(case["input_utf8"].to_utf8_buffer())["value"]
			break
	_support.expect(not accepted.is_empty(), "accepted path preview fixture must be available")
	if accepted.is_empty(): return
	accepted["preview"]["burden"]["total_burden"] = "2"
	_support.expect_reject(codec.encode_server_envelope(accepted), "mismatched burden total")
	accepted["preview"]["burden"]["item_burden"] = "18446744073709551615"
	accepted["preview"]["burden"]["coin_burden"] = "1"
	accepted["preview"]["burden"]["total_burden"] = "18446744073709551615"
	_support.expect_reject(codec.encode_server_envelope(accepted), "overflowing burden total")


func test_feedback_nested_bounds_ids_and_text_fail_closed() -> void:
	var codec: WireCodec = WireCodec.new()
	var inventory: Dictionary = {}
	for case: Dictionary in _load_fixture("server_envelope")["cases"]:
		if case["case_id"] == "accept_feedback_cue_inventory":
			inventory = codec.decode_server_envelope(case["input_utf8"].to_utf8_buffer())["value"]
			break
	_support.expect(not inventory.is_empty(), "feedback inventory fixture must be available")
	if inventory.is_empty(): return

	var excessive: Dictionary = inventory.duplicate(true)
	for event: Dictionary in excessive["events"]:
		if event["cue"]["kind"] != "transaction": continue
		var receipt: Dictionary = event["cue"]["costs"][0]
		for _index: int in range(WireCodec.MAX_FEEDBACK_TRANSACTION_COSTS):
			event["cue"]["costs"].append(receipt.duplicate(true))
		break
	_support.expect_reject(codec.encode_server_envelope(excessive), "feedback receipt overflow")

	var malformed_corpse: Dictionary = inventory.duplicate(true)
	for event: Dictionary in malformed_corpse["events"]:
		if event["cue"]["kind"] == "corpse":
			event["cue"]["corpse_id"] = "corpse:01"
			break
	_support.expect_reject(codec.encode_server_envelope(malformed_corpse), "noncanonical feedback corpse ID")

	var control_text: Dictionary = inventory.duplicate(true)
	for event: Dictionary in control_text["events"]:
		if event["cue"]["kind"] == "npc_message":
			event["cue"]["response"] = "line\nbreak"
			break
	_support.expect_reject(codec.encode_server_envelope(control_text), "feedback control text")


func _load_fixture(decoder: String) -> Dictionary:
	var bytes: PackedByteArray = FileAccess.get_file_as_bytes(TestSupport.wire_fixture_path(decoder))
	if bytes.is_empty(): return {}
	var decoded: Dictionary = StrictJson.decode_bytes(bytes, 4 * 1024 * 1024, 64)
	if not decoded["ok"] or typeof(decoded["value"]) != TYPE_DICTIONARY: return {}
	return decoded["value"] as Dictionary


func _keys_equal(value: Dictionary, expected: Array) -> bool:
	var actual_keys: Array = value.keys()
	actual_keys.sort()
	var expected_keys: Array = expected.duplicate()
	expected_keys.sort()
	return actual_keys == expected_keys


func _valid_hex(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING or value.is_empty() or value.length() % 2 != 0: return false
	for index: int in range(value.length()):
		if value[index] not in "0123456789abcdef": return false
	return true


func _decode_hex(text: String) -> PackedByteArray:
	var bytes: PackedByteArray = PackedByteArray()
	for index: int in range(0, text.length(), 2): bytes.append(text.substr(index, 2).hex_to_int())
	return bytes
