extends RefCounted

const COOKIE_VALUE: String = "synthetic-session-cookie"
const TOKEN_A: String = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
const TOKEN_B: String = "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE"
const CHARACTER_ID: String = "11111111-1111-4111-8111-111111111111"

var _support: TestSupport


func test_initial_entry_tracks_intermediate_states_and_presents_fresh_welcome() -> void:
	var context: Dictionary = _mounted_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	var observed_states: Array[String] = []
	fake.on_request = func(request: Dictionary) -> void:
		if request["path"] == ControlFacade.ROUTE_TICKET: observed_states.append(client.state_machine.current)
	fake.on_socket_open = func(_record: Dictionary) -> void: observed_states.append(client.state_machine.current)
	_queue_ticket(fake)
	client._on_enter_world_requested()
	_support.expect_equal(observed_states, [ConnectionStateMachine.ISSUING_TICKET, ConnectionStateMachine.SOCKET_CONNECTING], "ticket and connect calls expose truthful intermediate states")
	_support.expect_equal(client.state_machine.current, ConnectionStateMachine.AWAITING_WELCOME, "initial socket waits for required welcome")
	_support.expect(client.world_screen.visible and not client.character_screen.visible, "entry shows the world shell")
	_support.expect("AWAITING_WELCOME" in client.world_screen.connection_status.text, "world shell reports the exact pending lifecycle")
	_queue_server(fake, _server_fixture("accept_server_welcome"))
	client._process(0.0)
	_support.expect_equal(client.state_machine.current, ConnectionStateMachine.ONLINE, "fresh welcome enters online")
	_support.expect(client.authoritative_state.has_authority(), "fresh welcome installs complete authority")
	_support.expect_equal(client.authoritative_state.actor_id(), "player", "interactive actor comes from accepted welcome")
	_support.expect_equal(client.world_screen.world_view.observation_center(), Vector2i(0, 0), "welcome frame reaches the world view behind the seam")
	_support.expect("ONLINE" in client.world_screen.connection_status.text, "presented world reports online")
	_cleanup(client)


func test_updates_replace_duplicate_and_fail_closed_on_lower_or_conflict() -> void:
	var context: Dictionary = _online_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	var update: Dictionary = _server_fixture("accept_state_update")
	var generation_before: int = client.authoritative_state.frame_generation
	_queue_server(fake, update)
	client._process(0.0)
	_support.expect_equal(client.authoritative_state.world_revision(), "3", "newer update replaces authority")
	_support.expect_equal(client.authoritative_state.frame_generation, generation_before + 1, "replacement presents one new frame generation")
	_queue_server(fake, update)
	client._process(0.0)
	_support.expect_equal(client.authoritative_state.frame_generation, generation_before + 1, "exact duplicate does not re-present a frame")
	var lower: Dictionary = update.duplicate(true)
	lower["server_sequence"] = "0"
	lower["world_revision"] = "1"
	_queue_server(fake, lower)
	client._process(0.0)
	_support.expect_equal(client.state_machine.current, ConnectionStateMachine.RETRYABLE_ERROR, "lower update fails closed")
	_support.expect(not client.authoritative_state.has_authority(), "fail-closed lower update clears authority")
	_cleanup(client)

	var conflict_context: Dictionary = _online_context()
	var conflict_client: ClientRoot = conflict_context["client"]
	var conflict: Dictionary = _server_fixture("accept_state_update")
	conflict["server_sequence"] = "1"
	_queue_server(conflict_context["fake"], conflict)
	conflict_client._process(0.0)
	_support.expect_equal(conflict_client.state_machine.current, ConnectionStateMachine.RETRYABLE_ERROR, "equal-sequence conflicting frame fails closed")
	_cleanup(conflict_client)


func test_commands_settle_once_across_result_update_overtaking_and_dispositions() -> void:
	var context: Dictionary = _online_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	client._on_intent_requested({"kind": "wait"})
	var first_command_bytes: PackedByteArray = fake.sent_socket_bytes[-1]
	var first_command: Dictionary = WireCodec.new().decode_client_command_envelope(first_command_bytes)["value"]
	_support.expect_equal(first_command["client_sequence"], "1", "first interactive command uses current cursor")
	_support.expect_equal(first_command["actor_id"], "player", "command uses accepted actor identity")
	_support.expect(client.control_state.has_pending_command(), "command installs one pending record")
	var result: Dictionary = _server_fixture("accept_command_result")
	result["command_id"] = first_command["command_id"]
	_queue_server(fake, result)
	client._process(0.0)
	_support.expect(not client.control_state.has_pending_command(), "matching terminal result settles pending command")
	_support.expect_equal(client.control_state.active_next_sequence(), "2", "accepted result consumes sequence once")
	_support.expect(client.world_screen.get("_command_pending"), "result waits for uncovered after revision")
	var feedback_count: int = client.presentation_state.feedback_presenter.feedback_entries.size()
	_queue_server(fake, _server_fixture("accept_state_update"))
	client._process(0.0)
	_support.expect(not client.world_screen.get("_command_pending"), "covering update releases pending presentation")
	_queue_server(fake, result)
	client._process(0.0)
	_support.expect_equal(client.control_state.active_next_sequence(), "2", "replayed terminal result cannot consume twice")
	_support.expect_equal(client.presentation_state.feedback_presenter.feedback_entries.size(), feedback_count, "replayed result is presentation-idempotent")

	client._on_intent_requested({"kind": "wait"})
	var second_command: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[-1])["value"]
	var nonconsuming: Dictionary = _server_fixture("accept_rejected_wrong_actor")
	nonconsuming["command_id"] = second_command["command_id"]
	_queue_server(fake, nonconsuming)
	client._process(0.0)
	_support.expect(not client.control_state.has_pending_command(), "nonconsuming terminal result still settles")
	_support.expect_equal(client.control_state.active_next_sequence(), "2", "wrong-actor rejection leaves cursor reusable")

	client._on_intent_requested({"kind": "wait"})
	var third_command: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[-1])["value"]
	var overtaken: Dictionary = _server_fixture("accept_command_result")
	overtaken["command_id"] = third_command["command_id"]
	_queue_server(fake, overtaken)
	client._process(0.0)
	_support.expect(not client.world_screen.get("_command_pending"), "already-covered result revision releases immediately")
	_cleanup(client)


func test_preview_and_social_use_current_identity_without_command_cursor_state() -> void:
	var context: Dictionary = _online_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	var sequence_before: String = client.control_state.active_next_sequence()
	var preview_path: Array[String] = ["north", "east"]
	client.world_screen.set("_draft_path", preview_path.duplicate())
	client.world_screen.set("_has_draft", true)
	client._on_path_preview_requested(preview_path)
	var preview_request: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[-1])["value"]
	_support.expect(not preview_request.has("client_sequence") and not preview_request.has("command_id"), "preview has no gameplay cursor fields")
	var preview_result: Dictionary = _server_fixture("accept_path_preview_result")
	preview_result["preview_id"] = preview_request["preview_id"]
	_queue_server(fake, preview_result)
	client._process(0.0)
	_support.expect(client.path_preview_state.has_preview() and client.world_screen.has_current_preview(), "current correlated preview reaches world presentation")
	_queue_server(fake, _server_fixture("accept_state_update"))
	client._process(0.0)
	_support.expect(not client.path_preview_state.has_request() and not client.world_screen.has_current_preview(), "replacement frame invalidates preview without retry")
	_queue_server(fake, preview_result)
	client._process(0.0)
	_support.expect(not client.world_screen.has_current_preview(), "stale preview result is discarded")
	_support.expect_equal(client.control_state.active_next_sequence(), sequence_before, "preview lifecycle never moves command cursor")

	client._on_social_message_requested({"kind": "say"}, "Synthetic fixture message")
	var social_request: Dictionary = WireCodec.new().decode_client_command_envelope(fake.sent_socket_bytes[-1])["value"]
	_support.expect_equal(social_request.get("actor_id"), "player", "social send uses accepted actor")
	_support.expect(not client.control_state.has_pending_command(), "social send creates no gameplay pending state")
	var incoming: Dictionary = _server_fixture("accept_social_message_say")
	var message_result: Dictionary = _server_fixture("accept_message_result_accepted")
	message_result["message_id"] = social_request["message_id"]
	_queue_server(fake, incoming)
	_queue_server(fake, message_result)
	client._process(0.0)
	_support.expect_equal(client.presentation_state.feedback_presenter.chat_entries.size(), 2, "incoming delivery and sender result use existing chat presenter")
	_support.expect_equal(client.control_state.active_next_sequence(), sequence_before, "social traffic never moves command cursor")
	_cleanup(client)


func test_authoritative_frame_renders_immediately_while_correlated_payoff_waits() -> void:
	var context: Dictionary = _online_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	var world: WorldShellScreen = client.world_screen
	world.combat_feel_director.set_test_clock(100)
	world.note_command_installed({
		"kind": "physical_attack",
		"mode": "fight",
		"target_actor_id": "actor_1",
	})
	var update: Dictionary = _server_fixture("accept_state_update")
	update["frame"]["character"]["resources"]["hp"] = 7
	update["events"].append({
		"kind": "feedback",
		"cue": {
			"kind": "physical_combat",
			"source": {"actor_id": "player", "name": "Wayfarer", "kind": "player"},
			"target": {"actor_id": "actor_1", "name": "Actor 1", "kind": "monster"},
			"location": {"realm": "fixture_realm", "level": "surface", "position": {"x": 1, "y": 0}},
			"mode": "fight",
			"outcome": {
				"kind": "hit",
				"damage": 3,
				"armor_reduction": 0,
				"wound_before": "unhurt",
				"wound_after": "wounded",
				"target_hp": 7,
			},
		},
	})
	_queue_server(fake, update)
	client._process(0.0)
	_support.expect_equal(world.hud.hp_label.text, "HP 7/10", "replacement-frame HP renders immediately and is never held by feel timing")
	_support.expect_equal(world.combat_feel_director.held_feedback_count(), 1, "only the correlated formatted payoff waits")
	_support.expect_equal(_feedback_kind_count(client, "physical_combat"), 0, "correlated physical text is absent while unrelated presentation remains immediate")
	var immediate_count: int = client.presentation_state.feedback_presenter.feedback_entries.size()
	world.combat_feel_director.advance(379)
	_support.expect_equal(client.presentation_state.feedback_presenter.feedback_entries.size(), immediate_count, "formatted combat text does not release early")
	world.combat_feel_director.advance(380)
	_support.expect_equal(client.presentation_state.feedback_presenter.feedback_entries.size(), immediate_count + 1, "formatted combat text releases exactly with the payoff")
	_support.expect_equal(_feedback_kind_count(client, "physical_combat"), 1, "the correlated physical text appears exactly once")
	var roles: Array[String] = []
	for record: Dictionary in world.audio_cue_player.play_history:
		roles.append(str(record.get("role", "")))
	_support.expect_equal(roles, ["combat_swing", "combat_body_impact"], "local release and authoritative impact route once through root composition")
	_cleanup(client)


func test_decode_close_send_timeout_and_reconnect_paths_are_truthful() -> void:
	var malformed_context: Dictionary = _online_context()
	var malformed_client: ClientRoot = malformed_context["client"]
	malformed_context["fake"].queue_socket_packet("{\"kind\":\"state_update\"}".to_utf8_buffer())
	malformed_client._process(0.0)
	_support.expect_equal(malformed_client.state_machine.current, ConnectionStateMachine.RETRYABLE_ERROR, "malformed strict envelope fails closed")
	_support.expect(not malformed_client.authoritative_state.has_authority(), "decode failure clears stale authority")
	_cleanup(malformed_client)

	var close_context: Dictionary = _online_context()
	var close_client: ClientRoot = close_context["client"]
	close_context["fake"].socket_open = false
	close_client._process(0.0)
	_support.expect_equal(close_client.state_machine.current, ConnectionStateMachine.RECONCILING, "remote close enters reconciliation")
	_support.expect(not close_client.authoritative_state.has_authority(), "remote close clears stale authority")
	_cleanup(close_client)

	var timeout_context: Dictionary = _mounted_context()
	var timeout_client: ClientRoot = timeout_context["client"]
	_queue_ticket(timeout_context["fake"])
	timeout_client._on_enter_world_requested()
	timeout_client.set("_welcome_deadline_msec", Time.get_ticks_msec() - 1)
	timeout_client._process(0.0)
	_support.expect_equal(timeout_client.state_machine.current, ConnectionStateMachine.RETRYABLE_ERROR, "welcome deadline routes to retryable state")
	_cleanup(timeout_client)

	var reconnect_context: Dictionary = _online_context()
	var reconnect_client: ClientRoot = reconnect_context["client"]
	var reconnect_fake: FakeTransport = reconnect_context["fake"]
	reconnect_fake.socket_send_succeeds = false
	reconnect_client._on_intent_requested({"kind": "wait"})
	var pending: PendingCommand = reconnect_client.control_state.pending_command()
	var original_bytes: PackedByteArray = pending.encoded_bytes()
	var old_epoch: String = pending.control_epoch()
	_support.expect_equal(reconnect_client.state_machine.current, ConnectionStateMachine.RECONCILING, "send failure enters reconciliation")
	_support.expect(reconnect_client.control_state.has_pending_command(), "send failure retains immutable pending command")
	reconnect_fake.socket_send_succeeds = true
	reconnect_fake.queue_response(_response(200, _bootstrap(TOKEN_B)))
	_queue_ticket(reconnect_fake)
	reconnect_client._on_reconnect_requested()
	var fresh_welcome: Dictionary = _server_fixture("accept_server_welcome")
	fresh_welcome["connection_id"] = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
	fresh_welcome["control_epoch"] = "8"
	_queue_server(reconnect_fake, fresh_welcome)
	reconnect_client._process(0.0)
	_support.expect_equal(reconnect_client.state_machine.current, ConnectionStateMachine.RECONCILING, "fresh welcome waits for retained command")
	_support.expect_equal(reconnect_client.control_state.active_next_sequence(), "1", "fresh epoch starts at sequence one")
	_support.expect_equal(reconnect_fake.sent_socket_bytes[-1], original_bytes, "reconnect replays exact retained bytes")
	var retained_result: Dictionary = _server_fixture("accept_command_result")
	retained_result["command_id"] = pending.command_id()
	retained_result["after_revision"] = "2"
	_queue_server(reconnect_fake, retained_result)
	reconnect_client._process(0.0)
	_support.expect_equal(reconnect_client.state_machine.current, ConnectionStateMachine.ONLINE, "old-epoch terminal settlement completes reconciliation")
	_support.expect_equal(reconnect_client.control_state.next_sequence_by_epoch[old_epoch], "2", "retained epoch alone consumes its cursor")
	_support.expect_equal(reconnect_client.control_state.active_next_sequence(), "1", "fresh epoch cursor remains untouched")
	_cleanup(reconnect_client)


func test_reconnect_logout_and_tree_exit_close_the_owned_socket() -> void:
	var context: Dictionary = _online_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	fake.queue_response(_response(200, _bootstrap(TOKEN_B)))
	_queue_ticket(fake)
	client._on_reconnect_requested()
	_support.expect_equal(fake.socket_close_count, 1, "reconnect closes the active socket once")
	_queue_server(fake, _server_fixture("accept_server_welcome"))
	client._process(0.0)
	_support.expect_equal(client.state_machine.current, ConnectionStateMachine.ONLINE, "re-entry requires a fresh welcome")
	var chat: Dictionary = _server_fixture("accept_social_message_say")
	_queue_server(fake, chat)
	client._process(0.0)
	_support.expect_equal(client.presentation_state.feedback_presenter.chat_entries.size(), 1, "session presentation exists before logout")
	fake.queue_response(_response(204, null))
	client._on_logout_requested()
	_support.expect_equal(client.state_machine.current, ConnectionStateMachine.SIGNED_OUT, "successful logout enters signed out")
	_support.expect(client.login_screen.visible, "successful logout returns to login")
	_support.expect(client.presentation_state.feedback_presenter.chat_entries.is_empty(), "successful logout discards prior-session presentation")
	_support.expect_equal(client.world_screen.hud.hp_label.text, "HP —/—", "successful logout clears authoritative resources")
	_support.expect_equal(client.world_screen.readiness_status.text, "◇ Beat: no authoritative frame", "successful logout clears authoritative readiness")
	_support.expect_equal(fake.socket_close_count, 2, "logout closes the re-entered socket once")
	_cleanup(client)

	var failure_context: Dictionary = _online_context()
	var failure_client: ClientRoot = failure_context["client"]
	var failure_fake: FakeTransport = failure_context["fake"]
	failure_fake.queue_response(_response(500, {"code": "unavailable", "message": "Synthetic failure"}))
	failure_client._on_logout_requested()
	_support.expect_equal(failure_client.state_machine.current, ConnectionStateMachine.RETRYABLE_ERROR, "logout failure is explicitly retryable")
	_support.expect(failure_client.control_state.has_session_cookie(), "logout failure does not falsely clear retained session")
	_support.expect_equal(failure_fake.socket_close_count, 1, "logout failure still closes socket")
	_cleanup(failure_client)

	var exit_context: Dictionary = _online_context()
	var exit_client: ClientRoot = exit_context["client"]
	var exit_fake: FakeTransport = exit_context["fake"]
	var exit_control: ControlState = exit_client.control_state
	exit_client.free()
	_support.expect_equal(exit_fake.socket_close_count, 1, "tree exit closes active socket idempotently")
	_support.expect(exit_control.has_session_cookie(), "tree exit performs no implicit control logout")


func _mounted_context() -> Dictionary:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	var client: ClientRoot = (load("res://scenes/ClientRoot.tscn") as PackedScene).instantiate() as ClientRoot
	tree.root.add_child(client)
	var fake: FakeTransport = FakeTransport.new()
	client.control_state.set_session_cookie(COOKIE_VALUE)
	client.control_state.accept_bootstrap(_bootstrap(TOKEN_A))
	client.state_machine.current = ConnectionStateMachine.BOOTSTRAPPED
	client.control_state.lifecycle = ConnectionStateMachine.BOOTSTRAPPED
	client.facade = ControlFacade.new(fake, _endpoint(), client.control_state)
	return {"client": client, "fake": fake}


func _online_context() -> Dictionary:
	var context: Dictionary = _mounted_context()
	var client: ClientRoot = context["client"]
	var fake: FakeTransport = context["fake"]
	_queue_ticket(fake)
	client._on_enter_world_requested()
	_queue_server(fake, _server_fixture("accept_server_welcome"))
	client._process(0.0)
	return context


func _cleanup(client: ClientRoot) -> void:
	if is_instance_valid(client): client.free()


func _feedback_kind_count(client: ClientRoot, kind: String) -> int:
	var count: int = 0
	for entry: Dictionary in client.presentation_state.feedback_presenter.feedback_entries:
		if entry.get("kind") == kind:
			count += 1
	return count


func _queue_ticket(fake: FakeTransport) -> void:
	fake.queue_response(_response(200, {"ticket": TOKEN_B, "protocol_major": 1, "supported_minors": [8], "expires_in_seconds": "30"}))


func _queue_server(fake: FakeTransport, envelope: Dictionary) -> void:
	var encoded: Dictionary = WireCodec.new().encode_server_envelope(envelope)
	_support.expect_accept(encoded, "synthetic server envelope remains strict")
	if encoded.get("ok", false): fake.queue_socket_packet(encoded["bytes"])


func _server_fixture(case_id: String) -> Dictionary:
	var fixture: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(TestSupport.wire_fixture_path("server_envelope")))
	for case_value: Variant in fixture.get("cases", []):
		var fixture_case: Dictionary = case_value as Dictionary
		if fixture_case.get("case_id") == case_id:
			var decoded: Dictionary = WireCodec.new().decode_server_envelope(str(fixture_case["input_utf8"]).to_utf8_buffer())
			return decoded.get("value", {}).duplicate(true)
	return {}


func _endpoint() -> EndpointConfig:
	var config: EndpointConfig = EndpointConfig.new()
	config.https_base_url = "https://fixture.invalid"
	config.websocket_url = "wss://fixture.invalid/v3/socket"
	config.origin = "https://fixture.invalid"
	return config


func _bootstrap(token: String) -> Dictionary:
	return {
		"control_api_version": 3,
		"account": {"account_id": "99999999-9999-4999-8999-999999999999", "display_name": "Fixture Account"},
		"session": {"session_id": "88888888-8888-4888-8888-888888888888", "idle_timeout_seconds": "300", "absolute_timeout_seconds": "3600"},
		"csrf_token": token,
		"characters": [{"character_id": CHARACTER_ID, "slot": 1, "display_name": "Wayfarer"}],
		"selected_character_id": CHARACTER_ID,
		"player_kill_marks": {"active_count": 0, "gameplay_locked": false, "active_marks": [], "forgivable_marks": []},
	}


func _response(status: int, value: Variant) -> Dictionary:
	var body: PackedByteArray = PackedByteArray() if value == null else JSON.stringify(value).to_utf8_buffer()
	return {"ok": true, "ambiguous": false, "status": status, "headers": PackedStringArray(), "body": body}
