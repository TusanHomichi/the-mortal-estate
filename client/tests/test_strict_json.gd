extends RefCounted

var _support: TestSupport


func test_raw_input_rejects_byte_and_depth_overflow() -> void:
	var protocol_boundary: PackedByteArray = ("\"" + "a".repeat(WireCodec.MAX_INPUT_BYTES - 2) + "\"").to_utf8_buffer()
	_support.expect_accept(StrictJson.decode_bytes(protocol_boundary, WireCodec.MAX_INPUT_BYTES, WireCodec.MAX_JSON_NESTING), "protocol boundary-sized JSON must parse")
	_support.expect_reject(StrictJson.decode_bytes(protocol_boundary + PackedByteArray([0x20]), WireCodec.MAX_INPUT_BYTES, WireCodec.MAX_JSON_NESTING), "protocol byte overflow must reject")
	var control_boundary: PackedByteArray = ("\"" + "a".repeat(WireCodec.MAX_CONTROL_INPUT_BYTES - 2) + "\"").to_utf8_buffer()
	_support.expect_accept(StrictJson.decode_bytes(control_boundary, WireCodec.MAX_CONTROL_INPUT_BYTES, WireCodec.MAX_CONTROL_JSON_NESTING), "control boundary-sized JSON must parse")
	_support.expect_reject(StrictJson.decode_bytes(control_boundary + PackedByteArray([0x20]), WireCodec.MAX_CONTROL_INPUT_BYTES, WireCodec.MAX_CONTROL_JSON_NESTING), "control byte overflow must reject")
	var server_boundary: PackedByteArray = ("\"" + "a".repeat(WireCodec.MAX_SERVER_ENVELOPE_BYTES - 2) + "\"").to_utf8_buffer()
	_support.expect_accept(StrictJson.decode_bytes(server_boundary, WireCodec.MAX_SERVER_ENVELOPE_BYTES, WireCodec.MAX_JSON_NESTING), "server boundary-sized JSON must parse")
	_support.expect_reject(StrictJson.decode_bytes(server_boundary + PackedByteArray([0x20]), WireCodec.MAX_SERVER_ENVELOPE_BYTES, WireCodec.MAX_JSON_NESTING), "server byte overflow must reject")
	var depth_ok: String = "[".repeat(31) + "0" + "]".repeat(31)
	var depth_bad: String = "[".repeat(32) + "0" + "]".repeat(32)
	_support.expect_accept(StrictJson.decode_bytes(depth_ok.to_utf8_buffer(), 1024, 32), "depth boundary must parse")
	_support.expect_reject(StrictJson.decode_bytes(depth_bad.to_utf8_buffer(), 1024, 32), "depth overflow must reject")


func test_raw_input_rejects_invalid_utf8_duplicate_keys_and_malformed_json() -> void:
	_support.expect_reject(StrictJson.decode_bytes(PackedByteArray([0x7b, 0x22, 0x78, 0x22, 0x3a, 0xff, 0x7d]), 128, 8), "invalid UTF-8 must reject")
	_support.expect_reject(StrictJson.decode_bytes("{\"x\":1,\"x\":2}".to_utf8_buffer(), 128, 8), "duplicate keys must reject")
	_support.expect_reject(StrictJson.decode_bytes("{\"x\":}".to_utf8_buffer(), 128, 8), "malformed JSON must reject")
	_support.expect_reject(StrictJson.decode_bytes("{\"x\":01}".to_utf8_buffer(), 128, 8), "noncanonical JSON number must reject")
	_support.expect_accept(StrictJson.decode_bytes("{\"x\":\"ok\",\"y\":[true,null,-1.5e2]}".to_utf8_buffer(), 128, 8), "valid strict JSON must parse")
