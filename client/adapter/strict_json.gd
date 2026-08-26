class_name StrictJson
extends RefCounted

var _input: PackedByteArray = PackedByteArray()
var _index: int = 0
var _max_depth: int = 0
var _error: String = ""


static func decode_bytes(input: PackedByteArray, max_bytes: int, max_depth: int) -> Dictionary:
	var scanner: StrictJson = StrictJson.new()
	return scanner._decode(input, max_bytes, max_depth)


func _decode(input: PackedByteArray, max_bytes: int, max_depth: int) -> Dictionary:
	if input.size() > max_bytes:
		return _failure("JSON input exceeds its byte bound")
	if input.is_empty():
		return _failure("JSON input is empty")
	_input = input
	_index = 0
	_max_depth = max_depth
	_error = ""
	_skip_whitespace()
	if not _scan_value(1):
		return _failure(_error)
	_skip_whitespace()
	if _index != _input.size():
		return _failure("JSON input has trailing data")
	_index = 0
	_error = ""
	_skip_whitespace()
	var parsed: Variant = _construct_value()
	if not _error.is_empty():
		return _failure(_error)
	return {"ok": true, "value": parsed, "error": ""}


func _construct_value() -> Variant:
	_skip_whitespace()
	var byte: int = _input[_index]
	match byte:
		0x7b: return _construct_object()
		0x5b: return _construct_array()
		0x22: return _scan_string_token()
		0x74:
			_scan_literal("true")
			return true
		0x66:
			_scan_literal("false")
			return false
		0x6e:
			_scan_literal("null")
			return null
	return _construct_number()


func _construct_object() -> Dictionary:
	var value: Dictionary = {}
	_index += 1
	_skip_whitespace()
	if _take_if(0x7d): return value
	while _index < _input.size():
		var key: String = _scan_string_token()
		_skip_whitespace()
		_take_if(0x3a)
		value[key] = _construct_value()
		_skip_whitespace()
		if _take_if(0x7d): return value
		_take_if(0x2c)
		_skip_whitespace()
	_fail("validated JSON object could not be constructed")
	return value


func _construct_array() -> Array:
	var value: Array = []
	_index += 1
	_skip_whitespace()
	if _take_if(0x5d): return value
	while _index < _input.size():
		value.append(_construct_value())
		_skip_whitespace()
		if _take_if(0x5d): return value
		_take_if(0x2c)
		_skip_whitespace()
	_fail("validated JSON array could not be constructed")
	return value


func _construct_number() -> Variant:
	var start: int = _index
	_scan_number()
	var text: String = _input.slice(start, _index).get_string_from_ascii()
	if text.contains(".") or text.contains("e") or text.contains("E"):
		var floating: float = text.to_float()
		if not is_finite(floating): _fail("JSON number is non-finite")
		return floating
	var negative: bool = text.begins_with("-")
	var digits: String = text.substr(1) if negative else text
	var maximum: String = "9223372036854775808" if negative else "9223372036854775807"
	if digits.length() > 19 or (digits.length() == 19 and digits > maximum):
		_fail("JSON integer is outside the client integer range")
		return 0
	return text.to_int()


func _scan_value(depth: int) -> bool:
	if depth > _max_depth:
		return _fail("JSON input exceeds its nesting bound")
	if _index >= _input.size():
		return _fail("JSON value is missing")
	var byte: int = _input[_index]
	match byte:
		0x7b:
			return _scan_object(depth)
		0x5b:
			return _scan_array(depth)
		0x22:
			return not _scan_string_token().is_empty()
		0x74:
			return _scan_literal("true")
		0x66:
			return _scan_literal("false")
		0x6e:
			return _scan_literal("null")
		_:
			if byte == 0x2d or _is_digit(byte):
				return _scan_number()
	return _fail("JSON contains an unexpected token")


func _scan_object(depth: int) -> bool:
	_index += 1
	_skip_whitespace()
	if _take_if(0x7d):
		return true
	var seen: Dictionary = {}
	while true:
		if _index >= _input.size() or _input[_index] != 0x22:
			return _fail("JSON object key must be a string")
		var key: String = _scan_string_token()
		if key.is_empty() and not _last_string_was_empty():
			return false
		if seen.has(key):
			return _fail("JSON object contains a duplicate key")
		seen[key] = true
		_skip_whitespace()
		if not _take_if(0x3a):
			return _fail("JSON object key is missing a colon")
		_skip_whitespace()
		if not _scan_value(depth + 1):
			return false
		_skip_whitespace()
		if _take_if(0x7d):
			return true
		if not _take_if(0x2c):
			return _fail("JSON object is missing a comma")
		_skip_whitespace()
	return _fail("JSON object did not terminate")


func _scan_array(depth: int) -> bool:
	_index += 1
	_skip_whitespace()
	if _take_if(0x5d):
		return true
	while true:
		if not _scan_value(depth + 1):
			return false
		_skip_whitespace()
		if _take_if(0x5d):
			return true
		if not _take_if(0x2c):
			return _fail("JSON array is missing a comma")
		_skip_whitespace()
	return _fail("JSON array did not terminate")


func _scan_string_token() -> String:
	var start: int = _index
	_index += 1
	while _index < _input.size():
		var byte: int = _input[_index]
		if byte == 0x22:
			_index += 1
			var token: String = _input.slice(start, _index).get_string_from_utf8()
			var decoded: Variant = JSON.parse_string(token)
			if typeof(decoded) != TYPE_STRING:
				_fail("JSON string could not be decoded")
				return ""
			return decoded as String
		if byte < 0x20:
			_fail("JSON string contains an unescaped control byte")
			return ""
		if byte == 0x5c:
			_index += 1
			if _index >= _input.size():
				_fail("JSON string has an incomplete escape")
				return ""
			var escaped: int = _input[_index]
			if escaped == 0x75:
				if not _scan_unicode_escape():
					return ""
				continue
			if escaped not in [0x22, 0x5c, 0x2f, 0x62, 0x66, 0x6e, 0x72, 0x74]:
				_fail("JSON string contains an invalid escape")
				return ""
			_index += 1
			continue
		if byte < 0x80:
			_index += 1
			continue
		if not _scan_utf8_scalar():
			return ""
	_fail("JSON string is not terminated")
	return ""


func _scan_unicode_escape() -> bool:
	var first: int = _read_hex_escape()
	if first < 0:
		return false
	if first >= 0xd800 and first <= 0xdbff:
		if _index + 1 >= _input.size() or _input[_index] != 0x5c or _input[_index + 1] != 0x75:
			return _fail("JSON string contains an unpaired high surrogate")
		_index += 1
		var second: int = _read_hex_escape()
		if second < 0xdc00 or second > 0xdfff:
			return _fail("JSON string contains an invalid surrogate pair")
		return true
	if first >= 0xdc00 and first <= 0xdfff:
		return _fail("JSON string contains an unpaired low surrogate")
	return true


func _read_hex_escape() -> int:
	if _index >= _input.size() or _input[_index] != 0x75:
		_fail("JSON string has an invalid unicode escape")
		return -1
	_index += 1
	if _index + 4 > _input.size():
		_fail("JSON string has an incomplete unicode escape")
		return -1
	var value: int = 0
	for offset: int in range(4):
		var digit: int = _hex_value(_input[_index + offset])
		if digit < 0:
			_fail("JSON string has a non-hex unicode escape")
			return -1
		value = value * 16 + digit
	_index += 4
	return value


func _scan_utf8_scalar() -> bool:
	var first: int = _input[_index]
	var length: int = 0
	var minimum_second: int = 0x80
	var maximum_second: int = 0xbf
	if first >= 0xc2 and first <= 0xdf:
		length = 2
	elif first >= 0xe0 and first <= 0xef:
		length = 3
		if first == 0xe0:
			minimum_second = 0xa0
		elif first == 0xed:
			maximum_second = 0x9f
	elif first >= 0xf0 and first <= 0xf4:
		length = 4
		if first == 0xf0:
			minimum_second = 0x90
		elif first == 0xf4:
			maximum_second = 0x8f
	else:
		return _fail("JSON input contains invalid UTF-8")
	if _index + length > _input.size():
		return _fail("JSON input contains truncated UTF-8")
	var second: int = _input[_index + 1]
	if second < minimum_second or second > maximum_second:
		return _fail("JSON input contains invalid UTF-8")
	for offset: int in range(2, length):
		var continuation: int = _input[_index + offset]
		if continuation < 0x80 or continuation > 0xbf:
			return _fail("JSON input contains invalid UTF-8")
	_index += length
	return true


func _scan_number() -> bool:
	if _take_if(0x2d) and _index >= _input.size():
		return _fail("JSON number has no integer part")
	if _take_if(0x30):
		if _index < _input.size() and _is_digit(_input[_index]):
			return _fail("JSON number has a leading zero")
	else:
		if _index >= _input.size() or not _is_nonzero_digit(_input[_index]):
			return _fail("JSON number has an invalid integer part")
		while _index < _input.size() and _is_digit(_input[_index]):
			_index += 1
	if _take_if(0x2e):
		if _index >= _input.size() or not _is_digit(_input[_index]):
			return _fail("JSON number has an invalid fraction")
		while _index < _input.size() and _is_digit(_input[_index]):
			_index += 1
	if _index < _input.size() and _input[_index] in [0x65, 0x45]:
		_index += 1
		if _index < _input.size() and _input[_index] in [0x2b, 0x2d]:
			_index += 1
		if _index >= _input.size() or not _is_digit(_input[_index]):
			return _fail("JSON number has an invalid exponent")
		while _index < _input.size() and _is_digit(_input[_index]):
			_index += 1
	return true


func _scan_literal(literal: String) -> bool:
	var bytes: PackedByteArray = literal.to_ascii_buffer()
	if _index + bytes.size() > _input.size():
		return _fail("JSON literal is truncated")
	for offset: int in range(bytes.size()):
		if _input[_index + offset] != bytes[offset]:
			return _fail("JSON literal is invalid")
	_index += bytes.size()
	return true


func _skip_whitespace() -> void:
	while _index < _input.size() and _input[_index] in [0x20, 0x09, 0x0a, 0x0d]:
		_index += 1


func _take_if(byte: int) -> bool:
	if _index < _input.size() and _input[_index] == byte:
		_index += 1
		return true
	return false


func _last_string_was_empty() -> bool:
	return _error.is_empty()


func _is_digit(byte: int) -> bool:
	return byte >= 0x30 and byte <= 0x39


func _is_nonzero_digit(byte: int) -> bool:
	return byte >= 0x31 and byte <= 0x39


func _hex_value(byte: int) -> int:
	if byte >= 0x30 and byte <= 0x39:
		return byte - 0x30
	if byte >= 0x41 and byte <= 0x46:
		return byte - 0x41 + 10
	if byte >= 0x61 and byte <= 0x66:
		return byte - 0x61 + 10
	return -1


func _fail(message: String) -> bool:
	_error = message
	return false


func _failure(message: String) -> Dictionary:
	return {"ok": false, "value": null, "error": message}
