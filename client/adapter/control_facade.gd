class_name ControlFacade
extends RefCounted

const ROUTE_LOGIN: String = "/v3/login"
const ROUTE_SESSION: String = "/v3/session"
const ROUTE_LOGOUT: String = "/v3/logout"
const ROUTE_SELECT: String = "/v3/characters/select"
const ROUTE_TICKET: String = "/v3/socket-tickets"
const ROUTE_FORGIVE_PREFIX: String = "/v3/player-kill-marks/"
const ROUTE_SOCKET: String = "/v3/socket"

var transport: RefCounted
var endpoint: EndpointConfig
var state: ControlState
var codec: WireCodec
var redactor: SecretRedactor

var _operation_active: bool = false


func _init(injected_transport: RefCounted, config: EndpointConfig, control_state: ControlState = null) -> void:
	transport = injected_transport
	endpoint = config
	state = control_state if control_state != null else ControlState.new()
	codec = WireCodec.new()
	redactor = SecretRedactor.new()


func login(username: String, password: String) -> Dictionary:
	if not _begin_operation(): return _busy()
	state.begin_login(password)
	redactor.register(password)
	var encoded: Dictionary = codec.encode_login_request_v1({"username": username, "password": password})
	if not encoded["ok"]:
		state.finish_login()
		redactor.forget(password)
		_end_operation()
		return encoded
	var response: Dictionary = transport.control_request(HTTPClient.METHOD_POST, ROUTE_LOGIN, _headers(false, true), encoded["bytes"])
	state.finish_login()
	redactor.forget(password)
	if not response.get("ok", false):
		_end_operation()
		return _transport_failure(response)
	if response.get("status") != 200:
		_end_operation()
		return _status_failure(response)
	var cookie: String = _session_cookie_from_headers(response.get("headers", PackedStringArray()))
	if cookie.is_empty():
		_end_operation()
		return _failure("login response did not establish a session")
	state.set_session_cookie(cookie)
	redactor.register(cookie)
	var decoded: Dictionary = codec.decode_session_bootstrap_v1(response["body"])
	if decoded["ok"]: _accept_bootstrap(decoded["value"])
	_end_operation()
	return decoded


func bootstrap_session() -> Dictionary:
	if not _begin_operation(): return _busy()
	var result: Dictionary = _bootstrap_while_held()
	_end_operation()
	return result


func logout() -> Dictionary:
	return _mutation(ROUTE_LOGOUT, codec.encode_logout_request_v1({"csrf_token": state.csrf_token_for_control()}), Callable(), true)


func select_character(character_id: String) -> Dictionary:
	return _mutation(ROUTE_SELECT, codec.encode_character_select_request_v1({"csrf_token": state.csrf_token_for_control(), "character_id": character_id}), Callable(codec, "decode_character_selection_v1"))


func issue_socket_ticket() -> Dictionary:
	var result: Dictionary = _mutation(ROUTE_TICKET, codec.encode_socket_ticket_request_v1({"csrf_token": state.csrf_token_for_control()}), Callable(codec, "decode_socket_ticket_v1"))
	if result.get("ok", false):
		state.set_admission_ticket(result["value"]["ticket"])
		redactor.register(result["value"]["ticket"])
	return result


func forgive_player_kill_mark(mark_id: String, request_id: String) -> Dictionary:
	if not _begin_operation(): return _busy()
	var encoded: Dictionary = codec.encode_forgive_player_kill_mark_request_v1({"request_id": request_id})
	if not encoded["ok"]:
		_end_operation()
		return encoded
	var headers: PackedStringArray = _headers(true, true)
	headers.append("X-Tme-Csrf: " + state.csrf_token_for_control())
	var response: Dictionary = transport.control_request(HTTPClient.METHOD_POST, ROUTE_FORGIVE_PREFIX + mark_id + "/forgive", headers, encoded["bytes"])
	var result: Dictionary = _decode_mutation_response(response, Callable(codec, "decode_forgive_player_kill_mark_result_v1"), false)
	_end_operation()
	return result


func connect_socket() -> Dictionary:
	var ticket: String = state.consume_admission_ticket()
	if ticket.is_empty(): return _failure("no one-use admission ticket is available")
	var encoded: Dictionary = codec.encode_client_hello_envelope({"kind": "client_hello", "ticket": ticket, "supported_minors": [WireCodec.PROTOCOL_MINOR]})
	if not encoded["ok"]: return encoded
	var headers: PackedStringArray = PackedStringArray(["Origin: " + endpoint.origin])
	var result: Dictionary = transport.open_socket(ticket, encoded["bytes"], headers)
	redactor.forget(ticket)
	return result


func poll_server_envelopes() -> Dictionary:
	var envelopes: Array[Dictionary] = []
	for packet: PackedByteArray in transport.poll_socket():
		var decoded: Dictionary = codec.decode_server_envelope(packet)
		if not decoded.get("ok", false):
			return {"ok": false, "envelopes": envelopes, "socket_open": socket_is_open(), "error": redactor.redact(str(decoded.get("error", "server envelope decode failed")))}
		envelopes.append((decoded["value"] as Dictionary).duplicate(true))
	return {"ok": true, "envelopes": envelopes, "socket_open": socket_is_open(), "error": ""}


func submit_command(intent: Dictionary, authority: AuthoritativeState) -> Dictionary:
	var identity: Dictionary = _current_socket_identity(authority, true)
	if not identity.get("ok", false): return identity
	var command_id: String = _random_uuid_v4()
	var command: Dictionary = {
		"kind": "command",
		"command_id": command_id,
		"control_epoch": identity["control_epoch"],
		"client_sequence": identity["client_sequence"],
		"observed_world_revision": identity["world_revision"],
		"actor_id": identity["actor_id"],
		"intent": intent.duplicate(true),
	}
	var encoded: Dictionary = codec.encode_client_command_envelope(command)
	if not encoded.get("ok", false): return _failure(str(encoded.get("error", "command encoding failed")))
	var pending: PendingCommand = PendingCommand.create(
		encoded["bytes"], command_id, command["control_epoch"], command["client_sequence"],
		command["observed_world_revision"], command["actor_id"], command["intent"],
	)
	if not state.install_pending(pending): return _failure("a gameplay command is already pending")
	if not transport.send_socket_bytes(pending.encoded_bytes()):
		return {"ok": false, "pending_installed": true, "sent": false, "command_id": command_id, "error": "socket command send failed"}
	return {"ok": true, "pending_installed": true, "sent": true, "command_id": command_id, "error": ""}


func replay_pending_command() -> Dictionary:
	var pending: PendingCommand = state.pending_command()
	if pending == null: return _failure("no gameplay command is pending")
	if not transport.send_socket_bytes(pending.encoded_bytes()):
		return {"ok": false, "pending_installed": true, "sent": false, "command_id": pending.command_id(), "error": "socket command replay failed"}
	return {"ok": true, "pending_installed": true, "sent": true, "command_id": pending.command_id(), "error": ""}


func submit_path_preview(path: Array[String], preview_state: PathPreviewState, authority: AuthoritativeState) -> Dictionary:
	var identity: Dictionary = _current_socket_identity(authority, false)
	if not identity.get("ok", false):
		preview_state.clear()
		return identity
	var preview_id: String = _random_uuid_v4()
	var request: Dictionary = preview_state.begin(
		path, identity["control_epoch"], identity["world_revision"],
		identity["actor_id"], preview_id,
	)
	if request.is_empty(): return _failure("path preview request is invalid")
	var encoded: Dictionary = codec.encode_client_command_envelope(request)
	if not encoded.get("ok", false):
		preview_state.clear()
		return _failure(str(encoded.get("error", "path preview encoding failed")))
	if not transport.send_socket_bytes(encoded["bytes"]):
		preview_state.clear()
		return {"ok": false, "sent": false, "preview_id": preview_id, "error": "socket path preview send failed"}
	return {"ok": true, "sent": true, "preview_id": preview_id, "error": ""}


func send_social_message(scope: Dictionary, body: String, authority: AuthoritativeState) -> Dictionary:
	var identity: Dictionary = _current_socket_identity(authority, false)
	if not identity.get("ok", false): return identity
	var message_id: String = _random_uuid_v4()
	var message: Dictionary = {
		"kind": "social_message",
		"message_id": message_id,
		"control_epoch": identity["control_epoch"],
		"actor_id": identity["actor_id"],
		"scope": scope.duplicate(true),
		"body": body,
	}
	var encoded: Dictionary = codec.encode_client_command_envelope(message)
	if not encoded.get("ok", false): return _failure(str(encoded.get("error", "social message encoding failed")))
	if not transport.send_socket_bytes(encoded["bytes"]):
		return {"ok": false, "sent": false, "message_id": message_id, "error": "socket social message send failed"}
	return {"ok": true, "sent": true, "message_id": message_id, "error": ""}


func socket_is_open() -> bool: return transport.socket_is_open()
func close_socket() -> void: transport.close_socket()
func operation_active() -> bool: return _operation_active


func _current_socket_identity(authority: AuthoritativeState, include_sequence: bool) -> Dictionary:
	if authority == null or not authority.has_authority(): return _failure("authoritative identity is unavailable")
	var identity: Dictionary = {
		"ok": true,
		"control_epoch": state.active_control_epoch,
		"world_revision": authority.world_revision(),
		"actor_id": authority.actor_id(),
	}
	if include_sequence: identity["client_sequence"] = state.active_next_sequence()
	for key: String in ["control_epoch", "world_revision", "actor_id"]:
		if str(identity[key]).is_empty(): return _failure("current socket identity is incomplete")
	if include_sequence and str(identity["client_sequence"]).is_empty(): return _failure("current command sequence is unavailable")
	return identity


func _random_uuid_v4() -> String:
	var bytes: PackedByteArray = Crypto.new().generate_random_bytes(16)
	if bytes.size() != 16: return ""
	bytes[6] = (bytes[6] & 0x0f) | 0x40
	bytes[8] = (bytes[8] & 0x3f) | 0x80
	var hex: String = bytes.hex_encode()
	return hex.substr(0, 8) + "-" + hex.substr(8, 4) + "-" + hex.substr(12, 4) + "-" + hex.substr(16, 4) + "-" + hex.substr(20, 12)


func _mutation(path: String, encoded: Dictionary, decoder: Callable, logout_request: bool = false) -> Dictionary:
	if not _begin_operation(): return _busy()
	if not encoded["ok"]:
		_end_operation()
		return encoded
	var response: Dictionary = transport.control_request(HTTPClient.METHOD_POST, path, _headers(true, true), encoded["bytes"])
	var result: Dictionary = _decode_mutation_response(response, decoder, logout_request)
	_end_operation()
	return result


func _decode_mutation_response(response: Dictionary, decoder: Callable, logout_request: bool) -> Dictionary:
	if not response.get("ok", false):
		if response.get("ambiguous", false):
			var recovered: Dictionary = _bootstrap_while_held()
			return {"ok": false, "ambiguous": true, "rebootstrap": recovered.get("ok", false), "error": "mutation outcome requires a fresh decision"}
		return _transport_failure(response)
	if logout_request and response.get("status") == 204:
		state.clear_session()
		redactor.clear()
		return {"ok": true, "value": null, "error": ""}
	if response.get("status") != 200: return _status_failure(response)
	if decoder.is_null(): return {"ok": true, "value": null, "error": ""}
	var decoded: Dictionary = decoder.call(response["body"])
	if decoded.get("ok", false) and decoded["value"] is Dictionary and decoded["value"].has("character"):
		state.selected_character_id = decoded["value"]["character"]["character_id"]
	return decoded


func _bootstrap_while_held() -> Dictionary:
	var response: Dictionary = transport.control_request(HTTPClient.METHOD_GET, ROUTE_SESSION, _headers(true, false), PackedByteArray())
	if not response.get("ok", false): return _transport_failure(response)
	if response.get("status") != 200: return _status_failure(response)
	var decoded: Dictionary = codec.decode_session_bootstrap_v1(response["body"])
	if decoded["ok"]: _accept_bootstrap(decoded["value"])
	return decoded


func _accept_bootstrap(value: Dictionary) -> void:
	var previous: String = state.csrf_token_for_control()
	if not previous.is_empty(): redactor.forget(previous)
	state.accept_bootstrap(value)
	redactor.register(state.csrf_token_for_control())


func _headers(with_cookie: bool, json_body: bool) -> PackedStringArray:
	var headers: PackedStringArray = PackedStringArray(["Origin: " + endpoint.origin, "Accept: application/json"])
	if json_body: headers.append("Content-Type: application/json")
	if with_cookie and state.has_session_cookie(): headers.append("Cookie: __Host-tme_session=" + state.session_cookie_for_control())
	return headers


func _session_cookie_from_headers(headers: Variant) -> String:
	for header: String in headers:
		if header.to_lower().begins_with("set-cookie: __host-tme_session="):
			var start: int = header.find("=") + 1
			var ending: int = header.find(";", start)
			return header.substr(start) if ending < 0 else header.substr(start, ending - start)
	return ""


func _begin_operation() -> bool:
	if _operation_active: return false
	_operation_active = true
	return true


func _end_operation() -> void: _operation_active = false
func _busy() -> Dictionary: return _failure("another control request is already in flight")
func _transport_failure(_response: Dictionary) -> Dictionary: return _failure("control transport failed")
func _status_failure(response: Dictionary) -> Dictionary: return _failure("control request failed with status " + str(response.get("status", 0)))
func _failure(message: String) -> Dictionary: return {"ok": false, "error": redactor.redact(message)}
