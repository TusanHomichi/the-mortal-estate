class_name FakeTransport
extends RefCounted

var responses: Array[Dictionary] = []
var requests: Array[Dictionary] = []
var socket_opens: Array[Dictionary] = []
var socket_packets: Array[PackedByteArray] = []
var sent_socket_bytes: Array[PackedByteArray] = []
var on_request: Callable = Callable()
var on_socket_open: Callable = Callable()
var socket_open_succeeds: bool = true
var socket_send_succeeds: bool = true
var socket_open: bool = false
var socket_close_count: int = 0
var in_flight: int = 0
var maximum_in_flight: int = 0


func queue_response(response: Dictionary) -> void:
	responses.append(response)


func queue_socket_packet(packet: PackedByteArray) -> void:
	socket_packets.append(packet.duplicate())


func control_request(method: int, path: String, headers: PackedStringArray, body: PackedByteArray) -> Dictionary:
	in_flight += 1
	maximum_in_flight = maxi(maximum_in_flight, in_flight)
	requests.append({"method": method, "path": path, "headers": headers.duplicate(), "body": body.duplicate()})
	if not on_request.is_null(): on_request.call(requests[-1])
	var response: Dictionary = responses.pop_front() if not responses.is_empty() else {"ok": false, "ambiguous": false, "error": "no synthetic response"}
	in_flight -= 1
	return response


func open_socket(ticket: String, hello_bytes: PackedByteArray, headers: PackedStringArray) -> Dictionary:
	var record: Dictionary = {"ticket": ticket, "hello_bytes": hello_bytes.duplicate(), "headers": headers.duplicate()}
	socket_opens.append(record)
	if not on_socket_open.is_null(): on_socket_open.call(record)
	if not socket_open_succeeds:
		return {"ok": false, "ambiguous": false, "error": "synthetic socket open failure"}
	socket_open = true
	return {"ok": true, "ambiguous": false}


func send_socket_bytes(bytes: PackedByteArray) -> bool:
	sent_socket_bytes.append(bytes.duplicate())
	return socket_open and socket_send_succeeds


func poll_socket() -> Array[PackedByteArray]:
	var packets: Array[PackedByteArray] = []
	for packet: PackedByteArray in socket_packets:
		packets.append(packet.duplicate())
	socket_packets.clear()
	return packets


func socket_is_open() -> bool: return socket_open


func close_socket() -> void:
	if socket_open:
		socket_open = false
		socket_close_count += 1
