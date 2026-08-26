class_name NativeTransport
extends RefCounted

const REQUEST_TIMEOUT_MSEC: int = 10000

var endpoint: EndpointConfig
var socket: WebSocketPeer = null


func _init(config: EndpointConfig = null) -> void:
	endpoint = config


func control_request(method: int, path: String, headers: PackedStringArray, body: PackedByteArray) -> Dictionary:
	if endpoint == null or not endpoint.validation_errors().is_empty(): return _failure("endpoint configuration is invalid")
	var tls_options: TLSOptions = endpoint.build_tls_options()
	if tls_options == null: return _failure("TLS trust configuration could not be loaded")
	var authority: Dictionary = _split_authority(endpoint.https_base_url)
	if authority.is_empty(): return _failure("HTTPS authority is invalid")
	var client: HTTPClient = HTTPClient.new()
	var connect_error: Error = client.connect_to_host(authority["host"], authority["port"], tls_options)
	if connect_error != OK: return _failure("HTTPS connection could not start")
	var deadline: int = Time.get_ticks_msec() + REQUEST_TIMEOUT_MSEC
	while client.get_status() in [HTTPClient.STATUS_RESOLVING, HTTPClient.STATUS_CONNECTING]:
		if client.poll() != OK or Time.get_ticks_msec() >= deadline: return _failure("HTTPS connection did not become ready")
		OS.delay_msec(1)
	if client.get_status() != HTTPClient.STATUS_CONNECTED: return _failure("HTTPS connection failed")
	var request_error: Error = client.request(method, path, headers, body.get_string_from_utf8())
	if request_error != OK: return _failure("HTTPS request could not start")
	while client.get_status() == HTTPClient.STATUS_REQUESTING:
		if client.poll() != OK or Time.get_ticks_msec() >= deadline: return {"ok": false, "ambiguous": true, "error": "HTTPS outcome is ambiguous"}
		OS.delay_msec(1)
	if not client.has_response(): return {"ok": false, "ambiguous": true, "error": "HTTPS response is missing"}
	var status: int = client.get_response_code()
	var response_headers: PackedStringArray = client.get_response_headers()
	var response_body: PackedByteArray = PackedByteArray()
	while client.get_status() == HTTPClient.STATUS_BODY:
		if client.poll() != OK or Time.get_ticks_msec() >= deadline: return {"ok": false, "ambiguous": true, "error": "HTTPS response body is incomplete"}
		var chunk: PackedByteArray = client.read_response_body_chunk()
		if not chunk.is_empty(): response_body.append_array(chunk)
		else: OS.delay_msec(1)
	return {"ok": true, "ambiguous": false, "status": status, "headers": response_headers, "body": response_body}


func open_socket(ticket: String, hello_bytes: PackedByteArray, headers: PackedStringArray) -> Dictionary:
	if endpoint == null or not endpoint.validation_errors().is_empty(): return _failure("endpoint configuration is invalid")
	var tls_options: TLSOptions = endpoint.build_tls_options()
	if tls_options == null: return _failure("TLS trust configuration could not be loaded")
	socket = WebSocketPeer.new()
	socket.supported_protocols = PackedStringArray([WireCodec.WEBSOCKET_SUBPROTOCOL])
	socket.handshake_headers = headers
	var connect_error: Error = socket.connect_to_url(endpoint.websocket_url, tls_options)
	if connect_error != OK: return _failure("WSS connection could not start")
	var deadline: int = Time.get_ticks_msec() + REQUEST_TIMEOUT_MSEC
	while socket.get_ready_state() == WebSocketPeer.STATE_CONNECTING:
		socket.poll()
		if Time.get_ticks_msec() >= deadline: return _failure("WSS connection timed out")
		OS.delay_msec(1)
	if socket.get_ready_state() != WebSocketPeer.STATE_OPEN: return _failure("WSS connection failed")
	if ticket.is_empty() or socket.send_text(hello_bytes.get_string_from_utf8()) != OK: return _failure("client hello could not be sent")
	return {"ok": true, "ambiguous": false}


func send_socket_bytes(bytes: PackedByteArray) -> bool:
	return socket != null and socket.get_ready_state() == WebSocketPeer.STATE_OPEN and socket.send_text(bytes.get_string_from_utf8()) == OK


func poll_socket() -> Array[PackedByteArray]:
	var packets: Array[PackedByteArray] = []
	if socket == null: return packets
	socket.poll()
	while socket.get_available_packet_count() > 0: packets.append(socket.get_packet())
	return packets


func socket_is_open() -> bool:
	return socket != null and socket.get_ready_state() == WebSocketPeer.STATE_OPEN


func close_socket() -> void:
	if socket != null: socket.close()
	socket = null


func _split_authority(url: String) -> Dictionary:
	var remainder: String = url.substr("https://".length())
	var slash: int = remainder.find("/")
	if slash >= 0: remainder = remainder.substr(0, slash)
	var host: String = remainder
	var port: int = 443
	var colon: int = remainder.rfind(":")
	if colon > 0:
		host = remainder.substr(0, colon)
		port = remainder.substr(colon + 1).to_int()
	if host.is_empty() or port <= 0 or port > 65535: return {}
	return {"host": host, "port": port}


func _failure(message: String) -> Dictionary:
	return {"ok": false, "ambiguous": false, "error": message}
