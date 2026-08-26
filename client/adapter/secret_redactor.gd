class_name SecretRedactor
extends RefCounted

const REDACTED: String = "[REDACTED]"

var _registered: Array[String] = []


func register(value: String) -> void:
	if not value.is_empty() and value not in _registered:
		_registered.append(value)
		_registered.sort_custom(func(left: String, right: String) -> bool: return left.length() > right.length())


func forget(value: String) -> void:
	_registered.erase(value)


func clear() -> void:
	_registered.clear()


func redact(text: String) -> String:
	var safe: String = text
	for value: String in _registered:
		safe = safe.replace(value, REDACTED)
	return safe


func registered_count() -> int:
	return _registered.size()
