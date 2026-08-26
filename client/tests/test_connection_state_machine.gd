extends RefCounted

const CHARACTER_ID: String = "11111111-1111-4111-8111-111111111111"

var _support: TestSupport


func test_initial_welcome_requires_selected_character_and_observer_actor_identity() -> void:
	var context: Dictionary = _context(ConnectionStateMachine.AWAITING_WELCOME)
	var machine: ConnectionStateMachine = context["machine"]
	var authority: AuthoritativeState = context["authority"]
	var welcome: Dictionary = _welcome()
	_support.expect(machine.accept_welcome(welcome), "a selected character and matching observer actor are accepted")
	_support.expect_equal(machine.current, ConnectionStateMachine.ONLINE, "initial welcome enters online")
	_support.expect_equal(authority.actor_id(), "player", "accepted welcome retains actor identity")
	_support.expect_equal(context["control"].active_control_epoch, "7", "accepted welcome installs control epoch")

	var unselected_context: Dictionary = _context(ConnectionStateMachine.AWAITING_WELCOME)
	(unselected_context["control"] as ControlState).selected_character_id = ""
	_support.expect(not unselected_context["machine"].accept_welcome(_welcome()), "welcome without a selected character is rejected")
	_support.expect(not unselected_context["authority"].has_authority(), "unselected welcome mutates no authority")
	_support.expect_equal(unselected_context["control"].active_control_epoch, "", "unselected welcome mutates no control epoch")

	var unknown_context: Dictionary = _context(ConnectionStateMachine.AWAITING_WELCOME)
	(unknown_context["control"] as ControlState).selected_character_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
	_support.expect(not unknown_context["machine"].accept_welcome(_welcome()), "welcome for a character outside the bootstrap is rejected")
	_support.expect(not unknown_context["authority"].has_authority(), "unknown selection mutates no authority")

	var actor_context: Dictionary = _context(ConnectionStateMachine.AWAITING_WELCOME)
	var wrong_actor: Dictionary = _welcome()
	wrong_actor["actor_id"] = "other_actor"
	_support.expect(not actor_context["machine"].accept_welcome(wrong_actor), "welcome actor must equal frame observer")
	_support.expect(not actor_context["authority"].has_authority(), "actor mismatch mutates no authority")


func test_reconnect_welcome_preserves_existing_lifecycle_policy() -> void:
	var reconnect: Dictionary = _context(ConnectionStateMachine.RECONCILING)
	_support.expect(reconnect["machine"].accept_welcome(_welcome()), "reconciling accepts fresh welcome")
	_support.expect_equal(reconnect["machine"].current, ConnectionStateMachine.RECONCILING, "pending reconciliation does not enter online implicitly")
	_support.expect(reconnect["machine"].complete_reconciliation(true), "decided retained command completes reconciliation")

	var online: Dictionary = _context(ConnectionStateMachine.ONLINE)
	_support.expect(not online["machine"].accept_welcome(_welcome()), "an online connection accepts no second welcome")


func test_reconnect_transition_clears_authority_before_fresh_welcome() -> void:
	var control: ControlState = _control()
	var authority: AuthoritativeState = AuthoritativeState.new()
	authority.accept_welcome(_welcome())
	var machine: ConnectionStateMachine = ConnectionStateMachine.new(control, authority)
	machine.current = ConnectionStateMachine.ONLINE
	control.lifecycle = machine.current
	_support.expect(machine.socket_disconnected(), "online socket loss enters reconciliation")
	_support.expect(not authority.has_authority(), "reconciliation clears accepted frame")
	_support.expect_equal(authority.actor_id(), "", "reconciliation clears accepted actor")


func _context(state: String) -> Dictionary:
	var control: ControlState = _control()
	var authority: AuthoritativeState = AuthoritativeState.new()
	var machine: ConnectionStateMachine = ConnectionStateMachine.new(control, authority)
	machine.current = state
	control.lifecycle = state
	return {"control": control, "authority": authority, "machine": machine}


func _control() -> ControlState:
	var control: ControlState = ControlState.new()
	control.accept_bootstrap({
		"csrf_token": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
		"selected_character_id": CHARACTER_ID,
		"characters": [{"character_id": CHARACTER_ID}],
	})
	return control


func _welcome() -> Dictionary:
	var fixture: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(TestSupport.wire_fixture_path("server_envelope")))
	for case_value: Variant in fixture.get("cases", []):
		var fixture_case: Dictionary = case_value as Dictionary
		if fixture_case.get("case_id") == "accept_server_welcome":
			return WireCodec.new().decode_server_envelope(str(fixture_case["input_utf8"]).to_utf8_buffer())["value"]
	return {}
