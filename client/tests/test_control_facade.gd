extends RefCounted

const COOKIE_VALUE: String = "synthetic-session-cookie"
const TOKEN_A: String = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
const TOKEN_B: String = "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE"
const CHARACTER_ID: String = "11111111-1111-4111-8111-111111111111"
const MARK_ID: String = "55555555-5555-4555-8555-555555555555"
const REQUEST_ID: String = "77777777-7777-4777-8777-777777777777"

var _support: TestSupport


func test_control_facade_serializes_requests_and_rotates_csrf() -> void:
	var fake: FakeTransport = FakeTransport.new()
	var facade: ControlFacade = ControlFacade.new(fake, _endpoint())
	var nested_result: Dictionary = {}
	fake.on_request = func(_request: Dictionary) -> void: nested_result = facade.bootstrap_session()
	fake.queue_response(_response(200, _bootstrap(TOKEN_A), PackedStringArray(["Set-Cookie: __Host-tme_session=" + COOKIE_VALUE + "; Path=/; Secure; HttpOnly; SameSite=Strict"])))
	_support.expect_accept(facade.login("fixture_user", "synthetic fixture password"), "login succeeds")
	_support.expect_reject(nested_result, "nested control request is rejected while token is held")
	_support.expect_equal(fake.maximum_in_flight, 1, "only one control request may be active")
	_support.expect_equal(facade.state.csrf_token_for_control(), TOKEN_A, "login bootstrap installs token")
	fake.on_request = Callable()
	fake.queue_response(_response(200, _bootstrap(TOKEN_B)))
	_support.expect_accept(facade.bootstrap_session(), "explicit bootstrap succeeds")
	_support.expect_equal(facade.state.csrf_token_for_control(), TOKEN_B, "successful bootstrap is the token rotation source")
	var machine: ConnectionStateMachine = ConnectionStateMachine.new(facade.state, AuthoritativeState.new())
	_support.expect(machine.transition(ConnectionStateMachine.AUTHENTICATING), "signed out to authenticating")
	_support.expect(machine.transition(ConnectionStateMachine.BOOTSTRAPPED), "authenticating to bootstrapped")
	_support.expect(not machine.transition(ConnectionStateMachine.ONLINE), "online cannot bypass ticket and welcome")


func test_cookie_csrf_and_ticket_placement_are_endpoint_exact() -> void:
	var fake: FakeTransport = FakeTransport.new()
	var state: ControlState = ControlState.new()
	state.set_session_cookie(COOKIE_VALUE)
	state.set_csrf_token(TOKEN_A)
	var facade: ControlFacade = ControlFacade.new(fake, _endpoint(), state)
	fake.queue_response(_response(200, {"control_api_version": 3, "character": _character()}))
	_support.expect_accept(facade.select_character(CHARACTER_ID), "selection response")
	fake.queue_response(_response(200, {"ticket": TOKEN_B, "protocol_major": 1, "supported_minors": [8], "expires_in_seconds": "30"}))
	_support.expect_accept(facade.issue_socket_ticket(), "ticket response")
	_support.expect_accept(facade.connect_socket(), "socket opens with one-use ticket")
	_support.expect(not state.has_admission_ticket(), "ticket is erased from control state before WSS")
	_support.expect(not _headers_contain(fake.socket_opens[0]["headers"], "cookie:"), "WSS carries no cookie")
	_support.expect(_headers_contain(fake.socket_opens[0]["headers"], "origin: https://fixture.invalid"), "WSS carries exact Origin")
	var hello: Dictionary = WireCodec.new().decode_client_hello_envelope(fake.socket_opens[0]["hello_bytes"])
	_support.expect_accept(hello, "socket hello is exact current protocol")
	fake.queue_response(_response(200, {"control_api_version": 3, "mark_id": MARK_ID, "replay_status": "new"}))
	_support.expect_accept(facade.forgive_player_kill_mark(MARK_ID, REQUEST_ID), "forgiveness response")
	fake.queue_response(_response(204, null))
	_support.expect_accept(facade.logout(), "logout response")
	for request: Dictionary in fake.requests:
		if request["path"] == ControlFacade.ROUTE_SESSION: continue
		_support.expect(_headers_contain(request["headers"], "cookie: __host-tme_session="), request["path"] + " control request carries cookie")
	var select_body: Dictionary = StrictJson.decode_bytes(fake.requests[0]["body"], 16384, 16)["value"]
	_support.expect(_keys_equal(select_body, ["csrf_token", "character_id"]), "selection body has exact CSRF placement")
	var ticket_body: Dictionary = StrictJson.decode_bytes(fake.requests[1]["body"], 16384, 16)["value"]
	_support.expect(_keys_equal(ticket_body, ["csrf_token"]), "ticket body has exact CSRF placement")
	var forgive_request: Dictionary = fake.requests[2]
	_support.expect(_headers_contain(forgive_request["headers"], "x-tme-csrf: " + TOKEN_A.to_lower()), "forgiveness carries header CSRF")
	var forgive_body: Dictionary = StrictJson.decode_bytes(forgive_request["body"], 16384, 16)["value"]
	_support.expect(_keys_equal(forgive_body, ["request_id"]), "forgiveness body contains stable request ID only")
	var logout_body: Dictionary = StrictJson.decode_bytes(fake.requests[3]["body"], 16384, 16)["value"]
	_support.expect(_keys_equal(logout_body, ["csrf_token"]), "logout body carries CSRF")


func test_ambiguous_mutation_rebootstraps_before_decision() -> void:
	for operation: String in ["select", "ticket", "forgive", "logout"]:
		var fake: FakeTransport = FakeTransport.new()
		var state: ControlState = ControlState.new()
		state.set_session_cookie(COOKIE_VALUE)
		state.set_csrf_token(TOKEN_A)
		var facade: ControlFacade = ControlFacade.new(fake, _endpoint(), state)
		fake.queue_response({"ok": false, "ambiguous": true, "error": "synthetic ambiguous outcome"})
		fake.queue_response(_response(200, _bootstrap(TOKEN_B)))
		var result: Dictionary
		match operation:
			"select": result = facade.select_character(CHARACTER_ID)
			"ticket": result = facade.issue_socket_ticket()
			"forgive": result = facade.forgive_player_kill_mark(MARK_ID, REQUEST_ID)
			_: result = facade.logout()
		_support.expect(not result["ok"] and result["ambiguous"] and result["rebootstrap"], operation + " reports ambiguity after recovery")
		_support.expect_equal(fake.requests.size(), 2, operation + " performs mutation then bootstrap only")
		_support.expect_equal(fake.requests[1]["path"], ControlFacade.ROUTE_SESSION, operation + " recovers through bootstrap")
		_support.expect_equal(fake.requests.filter(func(request: Dictionary) -> bool: return request["path"] == fake.requests[0]["path"]).size(), 1, operation + " is never blindly retried")
		_support.expect_equal(state.csrf_token_for_control(), TOKEN_B, operation + " recovery rotates token")


func test_socket_facade_decodes_strict_batches_and_rejects_malformed_packets() -> void:
	var fake: FakeTransport = FakeTransport.new()
	fake.socket_open = true
	var facade: ControlFacade = ControlFacade.new(fake, _endpoint())
	fake.queue_socket_packet(_server_fixture_bytes("accept_server_welcome"))
	fake.queue_socket_packet(_server_fixture_bytes("accept_social_message_say"))
	var batch: Dictionary = facade.poll_server_envelopes()
	_support.expect_accept(batch, "strict socket batch decodes")
	_support.expect_equal(batch["envelopes"].map(func(envelope: Dictionary) -> String: return envelope["kind"]), ["server_welcome", "social_message"], "socket batch preserves packet order")
	_support.expect(batch["socket_open"], "socket batch reports post-poll open truth")
	fake.queue_socket_packet("{\"kind\":\"state_update\"}".to_utf8_buffer())
	var malformed: Dictionary = facade.poll_server_envelopes()
	_support.expect_reject(malformed, "strict receive rejects malformed envelope")
	_support.expect(malformed["socket_open"], "decode failure still reports post-poll socket truth")


func test_socket_facade_owns_command_preview_social_and_immutable_replay() -> void:
	var context: Dictionary = _live_socket_context()
	var fake: FakeTransport = context["fake"]
	var facade: ControlFacade = context["facade"]
	var state: ControlState = context["state"]
	var authority: AuthoritativeState = context["authority"]
	var intent: Dictionary = {"kind": "wait"}
	var submitted: Dictionary = facade.submit_command(intent, authority)
	_support.expect_accept(submitted, "gameplay command installs and sends")
	_support.expect(submitted["pending_installed"] and submitted["sent"], "command result distinguishes installation and send")
	var command: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[0])["value"]
	_support.expect_equal(command, {
		"kind": "command", "command_id": submitted["command_id"], "control_epoch": "7", "client_sequence": "1",
		"observed_world_revision": "2", "actor_id": "player", "intent": {"kind": "wait"},
	}, "command derives exact current identity and cursor")
	var installed_bytes: PackedByteArray = state.pending_command().encoded_bytes()
	intent["kind"] = "rest"
	var rejected_second: Dictionary = facade.submit_command({"kind": "wait"}, authority)
	_support.expect_reject(rejected_second, "only one gameplay command may be pending")
	_support.expect_equal(fake.sent_socket_bytes.size(), 1, "rejected second command is not sent")
	_support.expect_accept(facade.replay_pending_command(), "pending command replays")
	_support.expect_equal(fake.sent_socket_bytes[1], installed_bytes, "replay sends only immutable installed bytes")
	_support.expect_equal(state.pending_command().intent_facts(), {"kind": "wait"}, "pending intent is immutable from caller mutation")
	var sequence_before: String = state.active_next_sequence()
	var preview_state: PathPreviewState = PathPreviewState.new()
	var preview_path: Array[String] = ["north", "east"]
	var preview: Dictionary = facade.submit_path_preview(preview_path, preview_state, authority)
	_support.expect_accept(preview, "path preview sends")
	var preview_wire: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[2])["value"]
	_support.expect_equal(preview_wire.get("kind"), "path_preview", "preview uses Path 8 envelope")
	_support.expect(not preview_wire.has("command_id") and not preview_wire.has("client_sequence"), "preview has no gameplay command cursor")
	_support.expect_equal(state.active_next_sequence(), sequence_before, "preview does not consume command sequence")
	var social: Dictionary = facade.send_social_message({"kind": "shout"}, "Fixture hello", authority)
	_support.expect_accept(social, "social message sends")
	var social_wire: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[3])["value"]
	_support.expect_equal(social_wire, {
		"kind": "social_message", "message_id": social["message_id"], "control_epoch": "7",
		"actor_id": "player", "scope": {"kind": "shout"}, "body": "Fixture hello",
	}, "social message derives transient current identity")
	_support.expect_equal(state.pending_command().command_id(), submitted["command_id"], "preview and social do not replace gameplay pending state")


func test_socket_facade_failure_results_preserve_only_durable_command_state() -> void:
	var command_context: Dictionary = _live_socket_context()
	var command_fake: FakeTransport = command_context["fake"]
	command_fake.socket_send_succeeds = false
	var command_result: Dictionary = command_context["facade"].submit_command({"kind": "wait"}, command_context["authority"])
	_support.expect_reject(command_result, "command send failure is structured")
	_support.expect(command_result.get("pending_installed", false), "failed command send reports retained installation")
	_support.expect(command_context["state"].has_pending_command(), "failed command send retains immutable pending command")

	var preview_context: Dictionary = _live_socket_context()
	var preview_facade: ControlFacade = preview_context["facade"]
	var preview_authority: AuthoritativeState = preview_context["authority"]
	var preview_state: PathPreviewState = PathPreviewState.new()
	var preview_path: Array[String] = ["north"]
	preview_context["fake"].socket_send_succeeds = false
	_support.expect_reject(preview_facade.submit_path_preview(preview_path, preview_state, preview_authority), "preview send failure is structured")
	_support.expect(not preview_state.has_request(), "unsent preview request is cleared")
	_support.expect_reject(preview_facade.send_social_message({"kind": "say"}, "Fixture hello", preview_authority), "social send failure is structured")
	_support.expect(not preview_context["state"].has_pending_command(), "social failure creates no durable retry state")
	preview_facade.close_socket()
	preview_facade.close_socket()
	_support.expect_equal(preview_context["fake"].socket_close_count, 1, "socket close delegation is idempotent")


func _endpoint() -> EndpointConfig:
	var config: EndpointConfig = EndpointConfig.new()
	config.https_base_url = "https://fixture.invalid"
	config.websocket_url = "wss://fixture.invalid/v3/socket"
	config.origin = "https://fixture.invalid"
	return config


func _bootstrap(token: String) -> Dictionary:
	return {"control_api_version": 3, "account": {"account_id": "99999999-9999-4999-8999-999999999999", "display_name": "Fixture Account"}, "session": {"session_id": "88888888-8888-4888-8888-888888888888", "idle_timeout_seconds": "300", "absolute_timeout_seconds": "3600"}, "csrf_token": token, "characters": [_character()], "selected_character_id": CHARACTER_ID, "player_kill_marks": {"active_count": 0, "gameplay_locked": false, "active_marks": [], "forgivable_marks": []}}


func _character() -> Dictionary:
	return {"character_id": CHARACTER_ID, "slot": 1, "display_name": "Wayfarer"}


func _response(status: int, value: Variant, headers: PackedStringArray = PackedStringArray()) -> Dictionary:
	var body: PackedByteArray = PackedByteArray() if value == null else JSON.stringify(value).to_utf8_buffer()
	return {"ok": true, "ambiguous": false, "status": status, "headers": headers, "body": body}


func _headers_contain(headers: Variant, expected_lowercase: String) -> bool:
	for header: String in headers:
		if header.to_lower().begins_with(expected_lowercase): return true
	return false


func _keys_equal(value: Dictionary, expected: Array) -> bool:
	var actual_keys: Array = value.keys()
	actual_keys.sort()
	var expected_keys: Array = expected.duplicate()
	expected_keys.sort()
	return actual_keys == expected_keys


func _live_socket_context() -> Dictionary:
	var fake: FakeTransport = FakeTransport.new()
	fake.socket_open = true
	var state: ControlState = ControlState.new()
	state.accept_welcome("7")
	var authority: AuthoritativeState = AuthoritativeState.new()
	var welcome: Dictionary = WireCodec.new().decode_server_envelope(_server_fixture_bytes("accept_server_welcome"))["value"]
	authority.accept_welcome(welcome)
	return {"fake": fake, "state": state, "authority": authority, "facade": ControlFacade.new(fake, _endpoint(), state)}


func _server_fixture_bytes(case_id: String) -> PackedByteArray:
	var fixture: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(TestSupport.wire_fixture_path("server_envelope")))
	for case_value: Variant in fixture.get("cases", []):
		var fixture_case: Dictionary = case_value as Dictionary
		if fixture_case.get("case_id") == case_id:
			return str(fixture_case["input_utf8"]).to_utf8_buffer()
	return PackedByteArray()
