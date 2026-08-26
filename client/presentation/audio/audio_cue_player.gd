class_name AudioCuePlayer
extends Node

var loader: AudioManifestLoader
var play_history: Array[Dictionary] = []
var _voices_by_role: Dictionary = {}
var _muted: bool = false
var _volume_percent: int = 100
var _voice_sequence: int = 0


func configure(value: AudioManifestLoader) -> bool:
	loader = value
	return loader != null and not loader.load_manifest().is_empty()


func set_mix(muted: bool, volume_percent: int) -> bool:
	if volume_percent < 0 or volume_percent > 100:
		return false
	_muted = muted
	_volume_percent = volume_percent
	return true


func play_role(role: String, event_identity: String = "") -> Dictionary:
	if loader == null:
		return {}
	var cue: Dictionary = loader.cue_for_role(role, event_identity)
	if cue.is_empty():
		return {}
	_cleanup_role(role)
	var voices: Array = _voices_by_role.get(role, [])
	var cap: int = int(cue.get("max_simultaneous_voices", 1))
	while voices.size() >= cap:
		var stolen: Dictionary = voices.pop_front()
		var stolen_player: AudioStreamPlayer = stolen.get("player")
		if is_instance_valid(stolen_player):
			_free_voice(stolen_player)
	_voice_sequence += 1
	var record: Dictionary = {
		"sequence": _voice_sequence,
		"role": role,
		"cue_id": cue.get("id"),
		"event_identity": event_identity,
		"muted": _muted or _volume_percent == 0,
	}
	if bool(record["muted"]):
		play_history.append(record)
		return record
	var player: AudioStreamPlayer = AudioStreamPlayer.new()
	player.name = "Audio_%s_%d" % [role, _voice_sequence]
	player.stream = cue["stream"]
	player.volume_db = float(cue["gain_db"]) + linear_to_db(float(_volume_percent) / 100.0)
	player.pitch_scale = _deterministic_pitch(cue, event_identity)
	add_child(player)
	voices.append({"sequence": _voice_sequence, "player": player})
	_voices_by_role[role] = voices
	player.finished.connect(_on_voice_finished.bind(role, player))
	player.play()
	record["pitch_scale"] = player.pitch_scale
	record["volume_db"] = player.volume_db
	play_history.append(record)
	return record


func active_voice_count(role: String) -> int:
	_cleanup_role(role)
	return (_voices_by_role.get(role, []) as Array).size()


func discard() -> void:
	for role: Variant in _voices_by_role.keys():
		for voice: Dictionary in _voices_by_role[role]:
			var player: AudioStreamPlayer = voice.get("player")
			if is_instance_valid(player):
				_free_voice(player)
	_voices_by_role.clear()
	play_history.clear()


func _deterministic_pitch(cue: Dictionary, event_identity: String) -> float:
	var minimum: float = float(cue.get("pitch_min", 1.0))
	var maximum: float = float(cue.get("pitch_max", 1.0))
	if is_equal_approx(minimum, maximum):
		return minimum
	var digest: PackedByteArray = event_identity.sha256_buffer()
	var fraction: float = float(digest[1]) / 255.0
	return lerpf(minimum, maximum, fraction)


func _cleanup_role(role: String) -> void:
	var retained: Array = []
	for voice: Dictionary in _voices_by_role.get(role, []):
		var player: AudioStreamPlayer = voice.get("player")
		if is_instance_valid(player) and player.playing:
			retained.append(voice)
	_voices_by_role[role] = retained


func _on_voice_finished(role: String, player: AudioStreamPlayer) -> void:
	if is_instance_valid(player):
		player.queue_free()
	_cleanup_role(role)


func _free_voice(player: AudioStreamPlayer) -> void:
	player.stop()
	if player.get_parent() == self:
		remove_child(player)
	player.free()
