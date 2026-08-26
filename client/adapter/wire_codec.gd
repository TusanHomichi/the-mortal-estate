class_name WireCodec
extends RefCounted

const PROTOCOL_MAJOR: int = 1
const PROTOCOL_MINOR: int = 8
const CONTROL_API_VERSION: int = 3
const WEBSOCKET_SUBPROTOCOL: String = "tme.v1"
const MAX_INPUT_BYTES: int = 32768
const MAX_JSON_NESTING: int = 32
const MAX_SUPPORTED_MINORS: int = 8
const MAX_MOVE_PATH_STEPS: int = 3
const MAX_SERVER_ENVELOPE_BYTES: int = 262144
const MAX_SOCIAL_BODY_SCALARS: int = 280
const MAX_SOCIAL_BODY_BYTES: int = 1024
const MAX_CONTROL_INPUT_BYTES: int = 16384
const MAX_CONTROL_JSON_NESTING: int = 16
const OBSERVER_FRAME_CONTRACT_VERSION: int = 7
const STATIC_SCENE_CONTEXT_CONTRACT_VERSION: int = 1
const MAX_STATIC_SCENE_TILES: int = 225
const MAX_STATIC_SCENE_PROPS: int = 128
const MAX_STATIC_TRANSITION_APERTURES: int = 64
const MAX_MERCHANT_PURCHASE_ITEMS: int = 128
const MAX_FEEDBACK_TRANSACTION_COSTS: int = 64
const MAX_FEEDBACK_TRANSACTION_REWARDS: int = 64
const MAX_OBSERVED_EVENTS: int = 64

const CONTROL_ERROR_CODES: Array[String] = [
	"malformed_request", "invalid_credentials", "rate_limited",
	"authentication_required", "csrf_rejected", "character_not_owned",
	"character_not_selected", "gameplay_mark_locked",
	"forgiveness_unavailable", "unavailable",
]
const DIRECTIONS: Array[String] = [
	"north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest",
]
const CARRIED_POSITIONS: Array[String] = [
	"left_hand", "right_hand", "left_finger_1", "left_finger_2", "left_finger_3", "left_finger_4",
	"right_finger_1", "right_finger_2", "right_finger_3", "right_finger_4",
	"belt_1", "belt_2", "belt_3", "belt_4", "belt_back",
	"sack_item_1", "sack_item_2", "sack_item_3", "sack_item_4", "sack_item_5",
	"sack_item_6", "sack_item_7", "sack_item_8", "sack_item_9", "sack_item_10",
	"sack_item_11", "sack_item_12", "sack_item_13", "sack_item_14", "sack_item_15",
	"sack_item_16", "sack_item_17", "sack_item_18", "sack_item_19", "sack_item_20",
	"head", "neck", "left_arm", "right_arm", "gloves", "inner_armor", "outer_armor", "boots",
]

var _error: String = ""


func decode_named(decoder: String, input: PackedByteArray) -> Dictionary:
	match decoder:
		"decimal_u64": return decode_decimal_u64(input)
		"decimal_i64": return decode_decimal_i64(input)
		"login_request_v1": return decode_login_request_v1(input)
		"logout_request_v1": return decode_logout_request_v1(input)
		"character_select_request_v1": return decode_character_select_request_v1(input)
		"socket_ticket_request_v1": return decode_socket_ticket_request_v1(input)
		"forgive_player_kill_mark_request_v1": return decode_forgive_player_kill_mark_request_v1(input)
		"session_bootstrap_v1": return decode_session_bootstrap_v1(input)
		"character_selection_v1": return decode_character_selection_v1(input)
		"socket_ticket_v1": return decode_socket_ticket_v1(input)
		"forgive_player_kill_mark_result_v1": return decode_forgive_player_kill_mark_result_v1(input)
		"control_error_v1": return decode_control_error_v1(input)
		"client_hello_envelope": return decode_client_hello_envelope(input)
		"client_command_envelope": return decode_client_command_envelope(input)
		"server_envelope": return decode_server_envelope(input)
	return _failure("unrecognized decoder")


func decode_decimal_u64(input: PackedByteArray) -> Dictionary:
	return _decode(input, MAX_INPUT_BYTES, MAX_JSON_NESTING, Callable(self, "_decimal_u64"))


func decode_decimal_i64(input: PackedByteArray) -> Dictionary:
	return _decode(input, MAX_INPUT_BYTES, MAX_JSON_NESTING, Callable(self, "_decimal_i64"))


func decode_login_request_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_login_request"))


func decode_logout_request_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_logout_request"))


func decode_character_select_request_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_character_select_request"))


func decode_socket_ticket_request_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_socket_ticket_request"))


func decode_forgive_player_kill_mark_request_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_forgive_request"))


func decode_forgive_player_kill_mark_result_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_forgive_result"))


func decode_account_summary_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_account_summary"))


func decode_session_summary_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_session_summary"))


func decode_character_summary_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_character_summary"))


func decode_session_bootstrap_v1(input: PackedByteArray) -> Dictionary:
	var decoded: Dictionary = _decode_control(input, Callable(self, "_session_bootstrap"))
	if decoded["ok"]:
		var value: Dictionary = decoded["value"] as Dictionary
		var ordered: Dictionary = {}
		for key: String in ["control_api_version", "account", "session", "csrf_token", "characters", "selected_character_id", "player_kill_marks"]:
			ordered[key] = value[key] if value.has(key) else null
		decoded["value"] = ordered
	return decoded


func decode_player_kill_mark_summary_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_player_kill_mark_summary"))


func decode_forgivable_player_kill_mark_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_forgivable_mark"))


func decode_player_kill_mark_state_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_player_kill_mark_state"))


func decode_character_selection_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_character_selection"))


func decode_socket_ticket_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_socket_ticket"))


func decode_control_error_v1(input: PackedByteArray) -> Dictionary:
	return _decode_control(input, Callable(self, "_control_error"))


func decode_client_hello_envelope(input: PackedByteArray) -> Dictionary:
	return _decode(input, MAX_INPUT_BYTES, MAX_JSON_NESTING, Callable(self, "_client_hello"))


func decode_client_command_envelope(input: PackedByteArray) -> Dictionary:
	return _decode(input, MAX_INPUT_BYTES, MAX_JSON_NESTING, Callable(self, "_client_command"))


func decode_server_envelope(input: PackedByteArray) -> Dictionary:
	return _decode(input, MAX_SERVER_ENVELOPE_BYTES, MAX_JSON_NESTING, Callable(self, "_server_envelope"))


func encode_login_request_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_login_request"), MAX_CONTROL_INPUT_BYTES)
func encode_logout_request_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_logout_request"), MAX_CONTROL_INPUT_BYTES)
func encode_character_select_request_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_character_select_request"), MAX_CONTROL_INPUT_BYTES)
func encode_socket_ticket_request_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_socket_ticket_request"), MAX_CONTROL_INPUT_BYTES)
func encode_forgive_player_kill_mark_request_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_forgive_request"), MAX_CONTROL_INPUT_BYTES)
func encode_forgive_player_kill_mark_result_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_forgive_result"), MAX_CONTROL_INPUT_BYTES)
func encode_account_summary_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_account_summary"), MAX_CONTROL_INPUT_BYTES)
func encode_session_summary_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_session_summary"), MAX_CONTROL_INPUT_BYTES)
func encode_character_summary_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_character_summary"), MAX_CONTROL_INPUT_BYTES)
func encode_session_bootstrap_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_session_bootstrap"), MAX_CONTROL_INPUT_BYTES)
func encode_player_kill_mark_summary_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_player_kill_mark_summary"), MAX_CONTROL_INPUT_BYTES)
func encode_forgivable_player_kill_mark_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_forgivable_mark"), MAX_CONTROL_INPUT_BYTES)
func encode_player_kill_mark_state_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_player_kill_mark_state"), MAX_CONTROL_INPUT_BYTES)
func encode_character_selection_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_character_selection"), MAX_CONTROL_INPUT_BYTES)
func encode_socket_ticket_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_socket_ticket"), MAX_CONTROL_INPUT_BYTES)
func encode_control_error_v1(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_control_error"), MAX_CONTROL_INPUT_BYTES)
func encode_client_hello_envelope(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_client_hello"), MAX_INPUT_BYTES)
func encode_client_command_envelope(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_client_command"), MAX_INPUT_BYTES)
func encode_server_envelope(value: Dictionary) -> Dictionary: return _encode(value, Callable(self, "_server_envelope"), MAX_SERVER_ENVELOPE_BYTES)


func _decode_control(input: PackedByteArray, validator: Callable) -> Dictionary:
	return _decode(input, MAX_CONTROL_INPUT_BYTES, MAX_CONTROL_JSON_NESTING, validator)


func _decode(input: PackedByteArray, max_bytes: int, max_depth: int, validator: Callable) -> Dictionary:
	var raw: Dictionary = StrictJson.decode_bytes(input, max_bytes, max_depth)
	if not raw["ok"]:
		return raw
	_error = ""
	if not validator.call(raw["value"]):
		return _failure(_error)
	return {"ok": true, "value": raw["value"], "error": ""}


func _encode(value: Variant, validator: Callable, max_bytes: int) -> Dictionary:
	_error = ""
	if not validator.call(value):
		return _failure(_error)
	var bytes: PackedByteArray = JSON.stringify(value).to_utf8_buffer()
	if bytes.size() > max_bytes:
		return _failure("encoded value exceeds its byte bound")
	return {"ok": true, "bytes": bytes, "error": ""}


func _login_request(value: Variant) -> bool:
	return _fields(value, ["username", "password"]) and _username(value["username"]) and _password(value["password"])


func _logout_request(value: Variant) -> bool:
	return _fields(value, ["csrf_token"]) and _secret_token(value["csrf_token"])


func _character_select_request(value: Variant) -> bool:
	return _fields(value, ["csrf_token", "character_id"]) and _secret_token(value["csrf_token"]) and _uuid(value["character_id"], true)


func _socket_ticket_request(value: Variant) -> bool:
	return _fields(value, ["csrf_token"]) and _secret_token(value["csrf_token"])


func _forgive_request(value: Variant) -> bool:
	return _fields(value, ["request_id"]) and _uuid(value["request_id"], true)


func _forgive_result(value: Variant) -> bool:
	return _fields(value, ["control_api_version", "mark_id", "replay_status"]) \
		and _exact_int(value["control_api_version"], CONTROL_API_VERSION) \
		and _uuid(value["mark_id"], true) and _enum(value["replay_status"], ["new", "replayed"])


func _account_summary(value: Variant) -> bool:
	return _fields(value, ["account_id", "display_name"]) and _uuid(value["account_id"], true) and _display_name(value["display_name"])


func _session_summary(value: Variant) -> bool:
	return _fields(value, ["session_id", "idle_timeout_seconds", "absolute_timeout_seconds"]) \
		and _uuid(value["session_id"], true) and _decimal_u64(value["idle_timeout_seconds"]) and _decimal_u64(value["absolute_timeout_seconds"])


func _character_summary(value: Variant) -> bool:
	return _fields(value, ["character_id", "slot", "display_name"]) \
		and _uuid(value["character_id"], true) and _integer(value["slot"], 0, 255) \
		and _display_name(value["display_name"])


func _session_bootstrap(value: Variant) -> bool:
	if not _fields(value, ["control_api_version", "account", "session", "csrf_token", "characters", "player_kill_marks"], ["selected_character_id"]):
		return false
	if not _exact_int(value["control_api_version"], CONTROL_API_VERSION) or not _account_summary(value["account"]) or not _session_summary(value["session"]) or not _secret_token(value["csrf_token"]):
		return false
	if not _array_of(value["characters"], Callable(self, "_character_summary")):
		return false
	var character_ids: Dictionary = {}
	for character: Variant in value["characters"]:
		if character_ids.has(character["character_id"]): return _fail("bootstrap character identities must be unique")
		character_ids[character["character_id"]] = true
	if value.has("selected_character_id") and value["selected_character_id"] != null and not _uuid(value["selected_character_id"], true):
		return false
	return _player_kill_mark_state(value["player_kill_marks"])


func _player_kill_mark_summary(value: Variant) -> bool:
	if not _fields(value, ["mark_id", "victim_character_id", "victim_display_name", "assessed_at"], ["expires_at"]):
		return false
	if not _uuid(value["mark_id"], true) or not _uuid(value["victim_character_id"], true) or not _display_name(value["victim_display_name"]) or not _wire_label(value["assessed_at"]):
		return false
	return not value.has("expires_at") or value["expires_at"] == null or _wire_label(value["expires_at"])


func _forgivable_mark(value: Variant) -> bool:
	return _fields(value, ["mark_id", "killer_character_id", "killer_display_name", "assessed_at"]) \
		and _uuid(value["mark_id"], true) and _uuid(value["killer_character_id"], true) \
		and _display_name(value["killer_display_name"]) and _wire_label(value["assessed_at"])


func _player_kill_mark_state(value: Variant) -> bool:
	return _fields(value, ["active_count", "gameplay_locked", "active_marks", "forgivable_marks"]) \
		and _integer(value["active_count"], 0, 4294967295) and _boolean(value["gameplay_locked"]) \
		and _array_of(value["active_marks"], Callable(self, "_player_kill_mark_summary")) \
		and _array_of(value["forgivable_marks"], Callable(self, "_forgivable_mark"))


func _character_selection(value: Variant) -> bool:
	return _fields(value, ["control_api_version", "character"]) and _exact_int(value["control_api_version"], CONTROL_API_VERSION) and _character_summary(value["character"])


func _socket_ticket(value: Variant) -> bool:
	if not _fields(value, ["ticket", "protocol_major", "supported_minors", "expires_in_seconds"]):
		return false
	if not _secret_token(value["ticket"]) or not _exact_int(value["protocol_major"], PROTOCOL_MAJOR):
		return false
	if typeof(value["supported_minors"]) != TYPE_ARRAY or value["supported_minors"].size() != 1 or not _exact_int(value["supported_minors"][0], PROTOCOL_MINOR):
		return _fail("socket ticket must select only the current protocol minor")
	return _decimal_u64(value["expires_in_seconds"])


func _control_error(value: Variant) -> bool:
	return _fields(value, ["code"]) and _enum(value["code"], CONTROL_ERROR_CODES)


func _coord(value: Variant) -> bool:
	return _fields(value, ["x", "y"]) and _integer(value["x"], -2147483648, 2147483647) and _integer(value["y"], -2147483648, 2147483647)


func _position(value: Variant) -> bool:
	return _fields(value, ["realm", "level", "position"]) and _wire_label(value["realm"]) and _wire_label(value["level"]) and _coord(value["position"])


func _navigation(value: Variant) -> bool:
	if typeof(value) == TYPE_STRING:
		return _enum(value, ["walk", "swim", "door", "pit", "passage", "portal"])
	if not _fields(value, [], ["stairs", "climb"]):
		return false
	var object: Dictionary = value as Dictionary
	if object.size() != 1:
		return _fail("navigation variant must contain one tag")
	var tag: String = object.keys()[0] as String
	return tag in ["stairs", "climb"] and _fields(object[tag], ["direction"]) and _enum(object[tag]["direction"], ["up", "down"])


func _transition(value: Variant) -> bool:
	if not _fields(value, ["navigation", "target"], ["door_open"]):
		return false
	if not _navigation(value["navigation"]) or not _position(value["target"]):
		return false
	return not value.has("door_open") or value["door_open"] == null or _boolean(value["door_open"])


func _observer_tile(value: Variant) -> bool:
	if not _fields(value, ["position"], ["terrain_id", "terrain_name", "passable", "move_cost", "transition"]):
		return false
	if not _coord(value["position"]):
		return false
	for key: String in ["terrain_id", "terrain_name"]:
		if value.has(key) and value[key] != null and not _wire_label(value[key]): return false
	if value.has("passable") and value["passable"] != null and not _boolean(value["passable"]): return false
	if value.has("move_cost") and value["move_cost"] != null and not _integer(value["move_cost"], -2147483648, 2147483647): return false
	if value.has("transition") and value["transition"] != null and not _transition(value["transition"]): return false
	return true


func _observer_actor(value: Variant) -> bool:
	return _fields(value, ["actor_id", "character_id", "name", "kind", "position", "life_state", "hp", "max_hp", "attack_safety"]) \
		and _actor_id(value["actor_id"]) and _wire_label(value["name"]) and _enum(value["kind"], ["player", "monster", "npc"]) \
		and (value["character_id"] == null or _uuid(value["character_id"], true)) \
		and _position(value["position"]) and _enum(value["life_state"], ["alive", "ghost", "awaiting_resurrection", "dead"]) \
		and _integer(value["hp"], -2147483648, 2147483647) and _integer(value["max_hp"], -2147483648, 2147483647) \
		and _enum(value["attack_safety"], ["invalid", "protected", "open_self_defense", "open_evil_player", "open_hostile"])


func _observer_item(value: Variant) -> bool:
	return _fields(value, ["item_instance_id", "item_definition_id", "name", "quantity", "binding"]) \
		and _printable_ascii(value["item_instance_id"], 128) and _wire_label(value["item_definition_id"]) \
		and _wire_label(value["name"]) and _integer(value["quantity"], 0, 4294967295) \
		and _enum(value["binding"], ["unbound", "bound"])


func _loot_owner(value: Variant) -> bool:
	if not _fields(value, ["kind", "id"]): return false
	if value["kind"] == "character": return _uuid(value["id"], true)
	if value["kind"] == "transient_actor": return _actor_id(value["id"])
	return _fail("loot owner variant is invalid")


func _loot_claim(value: Variant) -> bool:
	return _fields(value, ["owner", "basis"]) and _loot_owner(value["owner"]) and _enum(value["basis"], ["killing_blow", "character_death_pile"])


func _observer_corpse(value: Variant) -> bool:
	if not _fields(value, ["corpse_id", "origin_actor_id", "origin_kind", "origin_name", "location", "sequence", "searched"], ["loot_claim"]): return false
	if not _printable_ascii(value["corpse_id"], 128) or not _actor_id(value["origin_actor_id"]) or not _enum(value["origin_kind"], ["player", "monster", "npc"]): return false
	if not _wire_label(value["origin_name"]) or not _position(value["location"]) or not _decimal_u64(value["sequence"]) or not _boolean(value["searched"]): return false
	return not value.has("loot_claim") or value["loot_claim"] == null or _loot_claim(value["loot_claim"])


func _observer_ground_item(value: Variant) -> bool:
	if not _fields(value, ["item_instance_id", "item_definition_id", "name", "quantity", "binding", "location"], ["loot_claim"]): return false
	var item: Dictionary = {"item_instance_id": value["item_instance_id"], "item_definition_id": value["item_definition_id"], "name": value["name"], "quantity": value["quantity"], "binding": value["binding"]}
	if not _observer_item(item) or not _position(value["location"]): return false
	return not value.has("loot_claim") or value["loot_claim"] == null or _loot_claim(value["loot_claim"])


func _observer_gold_pile(value: Variant) -> bool:
	if not _fields(value, ["gold_pile_id", "amount", "location"], ["loot_claim"]): return false
	if not _wire_label(value["gold_pile_id"]) or not _decimal_i64(value["amount"]) or _decimal_i64_negative(value["amount"]) or not _position(value["location"]): return false
	return not value.has("loot_claim") or value["loot_claim"] == null or _loot_claim(value["loot_claim"])


func _group_member(value: Variant) -> bool:
	if not _fields(value, ["character_id", "joined_order", "membership_epoch", "connected"], ["absent_since"]): return false
	if not _uuid(value["character_id"], true) or not _decimal_u64(value["joined_order"]) or not _decimal_u64(value["membership_epoch"]) or not _boolean(value["connected"]): return false
	return not value.has("absent_since") or value["absent_since"] == null or _decimal_u64(value["absent_since"])


func _group_view(value: Variant) -> bool:
	return _fields(value, ["group_id", "leader_character_id", "members"]) and _decimal_u64(value["group_id"]) and _uuid(value["leader_character_id"], true) and _array_of(value["members"], Callable(self, "_group_member"))


func _group_invitation(value: Variant) -> bool:
	if not _fields(value, ["invitation_id", "issuer_character_id", "target_character_id", "expires_at"], ["group_id"]): return false
	if not _decimal_u64(value["invitation_id"]) or not _uuid(value["issuer_character_id"], true) or not _uuid(value["target_character_id"], true) or not _decimal_u64(value["expires_at"]): return false
	return not value.has("group_id") or value["group_id"] == null or _decimal_u64(value["group_id"])


func _social_view(value: Variant) -> bool:
	if not _fields(value, ["character_id", "incoming_invitations", "outgoing_invitations", "pages_enabled", "blocked_character_ids"], ["group", "following_character_id"]): return false
	if not _uuid(value["character_id"], true) or not _array_of(value["incoming_invitations"], Callable(self, "_group_invitation")) or not _array_of(value["outgoing_invitations"], Callable(self, "_group_invitation")): return false
	if not _boolean(value["pages_enabled"]) or not _array_of(value["blocked_character_ids"], Callable(self, "_non_nil_uuid")): return false
	if value.has("group") and value["group"] != null and not _group_view(value["group"]): return false
	return not value.has("following_character_id") or value["following_character_id"] == null or _uuid(value["following_character_id"], true)


func _character_identity(value: Variant) -> bool:
	if not _fields(value, ["base_class_id", "current_class_id", "display_class", "nationality_id", "sex_or_gender_display"]): return false
	for key: String in ["base_class_id", "current_class_id", "display_class", "nationality_id"]:
		if not _wire_label(value[key]): return false
	return value["sex_or_gender_display"] == null or _wire_label(value["sex_or_gender_display"])


func _character_attributes(value: Variant) -> bool:
	if not _fields(value, ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"]): return false
	for key: String in ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"]:
		if not _integer(value[key], -2147483648, 2147483647): return false
	return true


func _character_resources(value: Variant) -> bool:
	if not _fields(value, ["hp", "max_hp", "peak_hp", "mp", "max_mp", "stamina", "max_stamina"]): return false
	for key: String in ["hp", "max_hp", "peak_hp", "mp", "max_mp", "stamina", "max_stamina"]:
		if not _integer(value[key], -2147483648, 2147483647): return false
	return true


func _character_progression(value: Variant) -> bool:
	return _fields(value, ["level", "experience", "pending_target_level"]) \
		and _integer(value["level"], -2147483648, 2147483647) and _decimal_i64(value["experience"]) \
		and (value["pending_target_level"] == null or _integer(value["pending_target_level"], -2147483648, 2147483647))


func _physical_attribute_adds(value: Variant) -> bool:
	return _fields(value, ["strength_adds", "dexterity_adds"]) \
		and _integer(value["strength_adds"], -2147483648, 2147483647) \
		and _integer(value["dexterity_adds"], -2147483648, 2147483647)


func _promotion_entry(value: Variant) -> bool:
	return _fields(value, ["from_class_id", "to_class_id", "level"]) \
		and _wire_label(value["from_class_id"]) and _wire_label(value["to_class_id"]) \
		and _integer(value["level"], -2147483648, 2147483647)


func _known_spell(value: Variant) -> bool:
	return _fields(value, ["spell_id", "lane", "learned_at_level"]) \
		and _wire_label(value["spell_id"]) and _wire_label(value["lane"]) \
		and _integer(value["learned_at_level"], -2147483648, 2147483647)


func _skill_entry(value: Variant) -> bool:
	if not _fields(value, ["track_id", "level", "critique_rank", "practice_points", "learning_rate", "track_display", "level_title"]): return false
	if not _wire_label(value["track_id"]) or not _integer(value["level"], 0, 255) or not _integer(value["critique_rank"], 0, 255): return false
	if not _decimal_u64(value["practice_points"]) or not _decimal_u64(value["learning_rate"]): return false
	return (value["track_display"] == null or _wire_label(value["track_display"])) \
		and (value["level_title"] == null or _wire_label(value["level_title"]))


func _controlled_character(value: Variant) -> bool:
	return _fields(value, ["identity", "alignment", "karma_points", "attributes", "resources", "progression", "physical_attribute_adds", "promotion_history", "known_spells", "skill_ledger"]) \
		and _character_identity(value["identity"]) and _enum(value["alignment"], ["lawful", "neutral", "chaotic", "evil"]) \
		and _integer(value["karma_points"], 0, 4294967295) and _character_attributes(value["attributes"]) \
		and _character_resources(value["resources"]) and _character_progression(value["progression"]) \
		and _physical_attribute_adds(value["physical_attribute_adds"]) \
		and _array_of(value["promotion_history"], Callable(self, "_promotion_entry")) \
		and _array_of(value["known_spells"], Callable(self, "_known_spell")) \
		and _array_of(value["skill_ledger"], Callable(self, "_skill_entry"))


func _owned_item(value: Variant) -> bool:
	if not _fields(value, ["item_instance_id", "item_definition_id", "name", "quantity", "identified", "appraised", "known_unit_value_gold", "known_stack_value_gold", "unit_burden", "stack_burden", "binding", "bow_readiness"]): return false
	if not _printable_ascii(value["item_instance_id"], 128) or not _wire_label(value["item_definition_id"]) or not _wire_label(value["name"]): return false
	if not _integer(value["quantity"], 0, 4294967295) or not _boolean(value["identified"]) or not _boolean(value["appraised"]): return false
	for key: String in ["known_unit_value_gold", "known_stack_value_gold"]:
		if value[key] != null and not _decimal_u64(value[key]): return false
	if not _decimal_u64(value["unit_burden"]) or not _decimal_u64(value["stack_burden"]): return false
	if not _enum(value["binding"], ["unrestricted", "bind_on_first_character_touch", "bound"]): return false
	return value["bow_readiness"] == null or _enum(value["bow_readiness"], ["unnocked", "nocked"])


func _positioned_item(value: Variant) -> bool:
	if not _fields(value, ["position", "item", "valid_placements"]): return false
	if not _enum(value["position"], CARRIED_POSITIONS) or not _owned_item(value["item"]): return false
	return _array_of_enum(value["valid_placements"], ["hand", "ring_finger", "belt_side", "belt_back", "sack", "head", "neck", "arm", "gloves", "inner_armor", "outer_armor", "boots"])


func _carried_layout(value: Variant) -> bool:
	if not _fields(value, ["items", "gold"]) or not _array_of(value["items"], Callable(self, "_positioned_item")): return false
	if not _fields(value["gold"], ["left_hand", "right_hand", "sack"]): return false
	for key: String in ["left_hand", "right_hand", "sack"]:
		if not _decimal_i64(value["gold"][key]) or _decimal_i64_negative(value["gold"][key]): return false
	return true


func _warmed_spell(value: Variant) -> bool:
	return _fields(value, ["spell_id", "warmed_at", "ready_at", "status"]) \
		and _wire_label(value["spell_id"]) and _decimal_u64(value["warmed_at"]) \
		and _decimal_u64(value["ready_at"]) and _enum(value["status"], ["warming", "ready"])


func _spell_action_state(value: Variant) -> bool:
	if not _fields(value, ["enabled", "blocked_reason", "requires_target_selection", "intent"]): return false
	if not _boolean(value["enabled"]) or not _boolean(value["requires_target_selection"]): return false
	if value["blocked_reason"] != null and not _wire_label(value["blocked_reason"]): return false
	return value["intent"] == null or _intent(value["intent"])


func _spell_action(value: Variant) -> bool:
	if not _fields(value, ["spell_id", "spell_name", "casting_method", "cast_class", "target_kind", "mp_cost", "stamina_cost", "hostile_act", "town_law_violation", "warm", "cast"]): return false
	if not _wire_label(value["spell_id"]) or not _wire_label(value["spell_name"]): return false
	if not _enum(value["casting_method"], ["direct", "warm_then_cast"]) or not _enum(value["cast_class"], ["character", "path", "path_or_character", "self", "not_applicable"]): return false
	if value["target_kind"] != null and not _enum(value["target_kind"], ["actor", "area", "coordinate", "direction", "door", "item", "none", "self"]): return false
	for key: String in ["mp_cost", "stamina_cost"]:
		if value[key] != null and not _integer(value[key], -2147483648, 2147483647): return false
	return _boolean(value["hostile_act"]) and _boolean(value["town_law_violation"]) \
		and _spell_action_state(value["warm"]) and _spell_action_state(value["cast"])


func _transaction_requirement(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("transaction requirement must contain a kind")
	match value["kind"]:
		"current_class": return _fields(value, ["kind", "class_id"]) and _wire_label(value["class_id"])
		"minimum_level": return _fields(value, ["kind", "level"]) and _integer(value["level"], -2147483648, 2147483647)
		"exact_karma": return _fields(value, ["kind", "karma_points"]) and _integer(value["karma_points"], 0, 4294967295)
		"exact_alignment": return _fields(value, ["kind", "alignment"]) and _enum(value["alignment"], ["lawful", "neutral", "chaotic", "evil"])
		"minimum_skill_level": return _fields(value, ["kind", "track_id", "level"]) and _wire_label(value["track_id"]) and _integer(value["level"], 0, 255)
		"minimum_carried_gold": return _fields(value, ["kind", "amount"]) and _decimal_i64(value["amount"])
		"carried_item": return _fields(value, ["kind", "item_definition_id", "quantity"]) and _wire_label(value["item_definition_id"]) and _integer(value["quantity"], 0, 4294967295)
		"carried_position_empty": return _fields(value, ["kind", "position"]) and _enum(value["position"], CARRIED_POSITIONS)
		"spell_unknown": return _fields(value, ["kind", "spell_id"]) and _wire_label(value["spell_id"])
		"quest_unstarted": return _fields(value, ["kind", "quest_id"]) and _wire_label(value["quest_id"])
		"quest_at_stage": return _fields(value, ["kind", "quest_id", "stage_id"]) and _wire_label(value["quest_id"]) and _wire_label(value["stage_id"])
		"npc_accompanying": return _fields(value, ["kind", "npc_actor_id"]) and _actor_id(value["npc_actor_id"])
	return _fail("transaction requirement variant is invalid")


func _transaction_cost(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("transaction cost must contain a kind")
	if value["kind"] == "carried_gold": return _fields(value, ["kind", "amount"]) and _decimal_i64(value["amount"])
	if value["kind"] == "selected_carried_item": return _fields(value, ["kind", "quantity"]) and _integer(value["quantity"], 0, 4294967295)
	return _fail("transaction cost variant is invalid")


func _transaction_reward(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("transaction reward must contain a kind")
	match value["kind"]:
		"experience": return _fields(value, ["kind", "amount"]) and _integer(value["amount"], -2147483648, 2147483647)
		"item": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "position"]) and _printable_ascii(value["item_instance_id"], 128) and _wire_label(value["item_definition_id"]) and _enum(value["position"], CARRIED_POSITIONS)
		"class": return _fields(value, ["kind", "to_class_id", "to_class_display"]) and _wire_label(value["to_class_id"]) and _wire_label(value["to_class_display"])
		"spell": return _fields(value, ["kind", "spell_id"]) and _wire_label(value["spell_id"])
		"quest_stage": return _fields(value, ["kind", "quest_id", "stage_id"]) and _wire_label(value["quest_id"]) and _wire_label(value["stage_id"])
	return _fail("transaction reward variant is invalid")


func _service_transaction(value: Variant) -> bool:
	return _fields(value, ["transaction_id", "label", "requirements", "costs", "rewards", "actions"]) \
		and _wire_label(value["transaction_id"]) and _wire_label(value["label"]) \
		and _array_of(value["requirements"], Callable(self, "_transaction_requirement")) \
		and _array_of(value["costs"], Callable(self, "_transaction_cost")) \
		and _array_of(value["rewards"], Callable(self, "_transaction_reward")) \
		and _array_of(value["actions"], Callable(self, "_observer_action_option"))


func _merchant_listing(value: Variant) -> bool:
	return _fields(value, ["item", "origin", "price_gold", "purchase"]) \
		and _owned_item(value["item"]) and _enum(value["origin"], ["authored_stock", "pawn_pool"]) \
		and _decimal_i64(value["price_gold"]) and _observer_action_option(value["purchase"])


func _item_service_operation(value: Variant) -> bool:
	return _fields(value, ["operation", "actions"]) \
		and _enum(value["operation"], ["appraise", "identify", "enchant_weapon"]) \
		and _array_of(value["actions"], Callable(self, "_observer_action_option"))


func _restoration_outcome(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("restoration outcome must contain a kind")
	if value["kind"] == "restore_resource": return _fields(value, ["kind", "resource"]) and _enum(value["resource"], ["hp", "mp", "stamina"])
	if value["kind"] == "cure_status": return _fields(value, ["kind", "status"]) and _enum(value["status"], ["blindness", "poison"])
	if value["kind"] == "priest_resurrection": return _fields(value, ["kind"])
	return _fail("restoration outcome variant is invalid")


func _restoration_operation(value: Variant) -> bool:
	return _fields(value, ["operation_id", "label", "requirements", "costs", "outcome", "actions"]) \
		and _wire_label(value["operation_id"]) and _wire_label(value["label"]) \
		and _array_of(value["requirements"], Callable(self, "_transaction_requirement")) \
		and _array_of(value["costs"], Callable(self, "_transaction_cost")) and _restoration_outcome(value["outcome"]) \
		and _array_of(value["actions"], Callable(self, "_observer_action_option"))


func _service_capability(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("service capability must contain a kind")
	match value["kind"]:
		"skill_training":
			return _fields(value, ["kind", "capability_id", "offered_track_ids", "selected_track_id", "actions"]) and _wire_label(value["capability_id"]) \
				and _array_of(value["offered_track_ids"], Callable(self, "_wire_label")) and (value["selected_track_id"] == null or _wire_label(value["selected_track_id"])) \
				and _array_of(value["actions"], Callable(self, "_observer_action_option"))
		"skill_critique": return _capability_actions(value, ["kind", "capability_id", "actions"])
		"spell_teaching":
			return _fields(value, ["kind", "capability_id", "spell_ids", "actions"]) and _wire_label(value["capability_id"]) \
				and _array_of(value["spell_ids"], Callable(self, "_wire_label")) and _array_of(value["actions"], Callable(self, "_observer_action_option"))
		"class_promotion":
			return _fields(value, ["kind", "capability_id", "target_class_id", "actions"]) and _wire_label(value["capability_id"]) \
				and _wire_label(value["target_class_id"]) and _array_of(value["actions"], Callable(self, "_observer_action_option"))
		"service_transaction":
			return _fields(value, ["kind", "capability_id", "transactions"]) and _wire_label(value["capability_id"]) \
				and _array_of(value["transactions"], Callable(self, "_service_transaction"))
		"merchant":
			return _fields(value, ["kind", "capability_id", "listings", "buy_all", "sales"]) and _wire_label(value["capability_id"]) \
				and _array_of(value["listings"], Callable(self, "_merchant_listing")) and _observer_action_option(value["buy_all"]) \
				and _array_of(value["sales"], Callable(self, "_observer_action_option"))
		"item_service":
			return _fields(value, ["kind", "capability_id", "operations"]) and _wire_label(value["capability_id"]) \
				and _array_of(value["operations"], Callable(self, "_item_service_operation"))
		"restoration":
			return _fields(value, ["kind", "capability_id", "operations"]) and _wire_label(value["capability_id"]) \
				and _array_of(value["operations"], Callable(self, "_restoration_operation"))
		"bank":
			return _fields(value, ["kind", "capability_id", "bank_id", "balance_gold", "transaction_cap_gold", "deposit_actions", "withdrawal_actions"]) \
				and _wire_label(value["capability_id"]) and _wire_label(value["bank_id"]) and _decimal_i64(value["balance_gold"]) and _decimal_i64(value["transaction_cap_gold"]) \
				and _array_of(value["deposit_actions"], Callable(self, "_observer_action_option")) and _array_of(value["withdrawal_actions"], Callable(self, "_observer_action_option"))
		"locker":
			return _fields(value, ["kind", "capability_id", "vault_id", "capacity", "item_count", "items", "deposit_actions", "withdrawal_actions"]) \
				and _wire_label(value["capability_id"]) and _wire_label(value["vault_id"]) and _integer(value["capacity"], 0, 4294967295) and _integer(value["item_count"], 0, 4294967295) \
				and _array_of(value["items"], Callable(self, "_owned_item")) and _array_of(value["deposit_actions"], Callable(self, "_observer_action_option")) \
				and _array_of(value["withdrawal_actions"], Callable(self, "_observer_action_option"))
	return _fail("service capability variant is invalid")


func _capability_actions(value: Variant, fields: Array) -> bool:
	return _fields(value, fields) and _wire_label(value["capability_id"]) \
		and _array_of(value["actions"], Callable(self, "_observer_action_option"))


func _service(value: Variant) -> bool:
	return _fields(value, ["service_id", "name", "position", "capabilities"]) \
		and _wire_label(value["service_id"]) and _wire_label(value["name"]) and _position(value["position"]) \
		and _array_of(value["capabilities"], Callable(self, "_service_capability"))


func _npc_interaction_outcome(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("npc interaction outcome must contain a kind")
	if value["kind"] in ["speak", "begin_follow", "end_follow"]: return _fields(value, ["kind"])
	if value["kind"] == "complete_escort": return _fields(value, ["kind", "npc_actor_id"]) and _actor_id(value["npc_actor_id"])
	if value["kind"] == "climb": return _fields(value, ["kind", "direction"]) and _enum(value["direction"], ["up", "down"])
	return _fail("npc interaction outcome variant is invalid")


func _npc_interaction(value: Variant) -> bool:
	return _fields(value, ["interaction_id", "label", "requirements", "costs", "rewards", "outcome", "actions"]) \
		and _wire_label(value["interaction_id"]) and _wire_label(value["label"]) \
		and _array_of(value["requirements"], Callable(self, "_transaction_requirement")) \
		and _array_of(value["costs"], Callable(self, "_transaction_cost")) and _array_of(value["rewards"], Callable(self, "_transaction_reward")) \
		and _npc_interaction_outcome(value["outcome"]) and _array_of(value["actions"], Callable(self, "_observer_action_option"))


func _npc(value: Variant) -> bool:
	return _fields(value, ["actor_id", "name", "following_character_id", "interactions"]) \
		and _actor_id(value["actor_id"]) and _wire_label(value["name"]) \
		and (value["following_character_id"] == null or _uuid(value["following_character_id"], true)) \
		and _array_of(value["interactions"], Callable(self, "_npc_interaction"))


func _quest_state(value: Variant) -> bool:
	return _fields(value, ["quest_id", "quest_title", "stage_id", "stage_label", "terminal"]) \
		and _wire_label(value["quest_id"]) and _wire_label(value["quest_title"]) and _wire_label(value["stage_id"]) \
		and _wire_label(value["stage_label"]) and _boolean(value["terminal"])


func _item_offer(value: Variant) -> bool:
	return _fields(value, ["item", "sender_character_id", "recipient_character_id", "source_position", "actions"]) \
		and _owned_item(value["item"]) and _uuid(value["sender_character_id"], true) \
		and _uuid(value["recipient_character_id"], true) and _enum(value["source_position"], CARRIED_POSITIONS) \
		and _array_of(value["actions"], Callable(self, "_observer_action_option"))


func _observer_action_option(value: Variant) -> bool:
	if not _fields(value, ["id", "label", "enabled"], ["blocked_reason", "intent"]): return false
	if not _printable_ascii(value["id"], 192) or not _printable_ascii(value["label"], 256) or not _boolean(value["enabled"]): return false
	if value.has("blocked_reason") and value["blocked_reason"] != null and not _wire_label(value["blocked_reason"]): return false
	return not value.has("intent") or value["intent"] == null or _intent(value["intent"])


func _inspect_exit_status(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("inspect exit status must contain a kind")
	if value["kind"] in ["walkable", "blocked_terrain", "out_of_bounds"]: return _fields(value, ["kind"])
	if value["kind"] == "door": return _fields(value, ["kind", "open", "target"]) and _boolean(value["open"]) and _position(value["target"])
	return _fail("inspect exit status variant is invalid")


func _inspect_exit(value: Variant) -> bool:
	if not _fields(value, ["direction", "location", "terrain", "move_cost", "status"]): return false
	if not _enum(value["direction"], DIRECTIONS) or not _position(value["location"]) or not _inspect_exit_status(value["status"]): return false
	if value["terrain"] != null and not _wire_label(value["terrain"]): return false
	return value["move_cost"] == null or _integer(value["move_cost"], -2147483648, 2147483647)


func _inspect_actor(value: Variant) -> bool:
	return _fields(value, ["direction", "actor_id", "actor", "kind", "location", "hp"]) \
		and _enum(value["direction"], DIRECTIONS) and _actor_id(value["actor_id"]) and _wire_label(value["actor"]) \
		and _enum(value["kind"], ["player", "monster", "npc"]) and _position(value["location"]) \
		and _integer(value["hp"], -2147483648, 2147483647)


func _inspect_ground_item(value: Variant) -> bool:
	if not _fields(value, ["item_instance_id", "item_definition_id", "name", "quantity", "binding", "location", "direction"]): return false
	var item: Dictionary = {"item_instance_id": value["item_instance_id"], "item_definition_id": value["item_definition_id"], "name": value["name"], "quantity": value["quantity"], "binding": value["binding"]}
	return _observer_item(item) and _position(value["location"]) \
		and (value["direction"] == null or _enum(value["direction"], DIRECTIONS))


func _observer_frame(value: Variant) -> bool:
	if not _fields(value, ["contract_version", "logical_time", "ready_at", "observer_actor_id", "observation_center", "observation_radius", "can_act", "tiles", "actors", "corpses", "corpses_truncated", "ground_items", "ground_items_truncated", "gold_piles", "gold_piles_truncated", "character", "carried", "burden", "warmed_spell", "spell_actions", "services_here", "npcs_here", "quest_log", "action_options", "action_options_truncated", "social", "incoming_item_offers", "outgoing_item_offers"]): return false
	if not _exact_int(value["contract_version"], OBSERVER_FRAME_CONTRACT_VERSION) or not _decimal_u64(value["logical_time"]) or not _decimal_u64(value["ready_at"]) or not _actor_id(value["observer_actor_id"]): return false
	if not _position(value["observation_center"]) or not _integer(value["observation_radius"], 0, 4294967295) or not _boolean(value["can_act"]): return false
	if not _array_of(value["tiles"], Callable(self, "_observer_tile")) or not _array_of(value["actors"], Callable(self, "_observer_actor")): return false
	if not _array_of(value["corpses"], Callable(self, "_observer_corpse")) or not _boolean(value["corpses_truncated"]): return false
	if not _array_of(value["ground_items"], Callable(self, "_observer_ground_item")) or not _boolean(value["ground_items_truncated"]): return false
	if not _array_of(value["gold_piles"], Callable(self, "_observer_gold_pile")) or not _boolean(value["gold_piles_truncated"]): return false
	if not _controlled_character(value["character"]) or not _carried_layout(value["carried"]) or not _burden(value["burden"]): return false
	if value["warmed_spell"] != null and not _warmed_spell(value["warmed_spell"]): return false
	if not _array_of(value["spell_actions"], Callable(self, "_spell_action")): return false
	if not _array_of(value["services_here"], Callable(self, "_service")) or not _array_of(value["npcs_here"], Callable(self, "_npc")): return false
	if not _array_of(value["quest_log"], Callable(self, "_quest_state")): return false
	if not _array_of(value["action_options"], Callable(self, "_observer_action_option")) or not _boolean(value["action_options_truncated"]): return false
	if not _social_view(value["social"]) or not _array_of(value["incoming_item_offers"], Callable(self, "_item_offer")): return false
	return _array_of(value["outgoing_item_offers"], Callable(self, "_item_offer"))


func _static_scene_context(value: Variant) -> bool:
	if not _fields(value, [
		"contract_version", "site", "bounds", "content_digest", "visual_manifest_digest",
		"scene_role", "presentation_mode", "world_zoom", "tiles", "walkable_mask",
		"static_props", "transition_apertures",
	]): return false
	if not _exact_int(value["contract_version"], STATIC_SCENE_CONTEXT_CONTRACT_VERSION): return false
	if not _fields(value["site"], ["realm", "level"]) \
		or not _wire_label(value["site"]["realm"]) or not _wire_label(value["site"]["level"]): return false
	if not _fields(value["bounds"], ["min", "max"]) \
		or not _coord(value["bounds"]["min"]) or not _coord(value["bounds"]["max"]): return false
	var minimum: Dictionary = value["bounds"]["min"]
	var maximum: Dictionary = value["bounds"]["max"]
	if minimum["x"] > maximum["x"] or minimum["y"] > maximum["y"]: return _fail("static scene bounds are inverted")
	if not _wire_label(value["content_digest"]) or str(value["content_digest"]).length() != 64: return false
	if not _wire_label(value["visual_manifest_digest"]) or str(value["visual_manifest_digest"]).length() != 64: return false
	if not _enum(value["scene_role"], ["overworld", "combat_space", "interior"]): return false
	if not _enum(value["presentation_mode"], ["overworld_town", "combat_space"]): return false
	if typeof(value["world_zoom"]) != TYPE_ARRAY or value["world_zoom"].size() != 2 \
		or not _integer(value["world_zoom"][0], 1, 4294967295) \
		or not _integer(value["world_zoom"][1], 1, 4294967295): return false
	if typeof(value["tiles"]) != TYPE_ARRAY or value["tiles"].size() > MAX_STATIC_SCENE_TILES: return false
	var width: int = maximum["x"] - minimum["x"] + 1
	var height: int = maximum["y"] - minimum["y"] + 1
	if width <= 0 or height <= 0 or width * height != value["tiles"].size(): return _fail("static scene tile rectangle differs from bounds")
	var tile_walkability: Dictionary = {}
	for tile_value: Variant in value["tiles"]:
		if not _static_scene_tile(tile_value): return false
		var tile: Dictionary = tile_value
		var key: String = str(tile["position"]["x"]) + ":" + str(tile["position"]["y"])
		if tile_walkability.has(key): return _fail("static scene contains a duplicate tile")
		tile_walkability[key] = tile["walkable"]
	if typeof(value["walkable_mask"]) != TYPE_ARRAY or value["walkable_mask"].size() > value["tiles"].size(): return false
	var mask: Dictionary = {}
	for position_value: Variant in value["walkable_mask"]:
		if not _coord(position_value): return false
		var key: String = str(position_value["x"]) + ":" + str(position_value["y"])
		if mask.has(key) or not tile_walkability.get(key, false): return _fail("static walkable mask contains a duplicate or non-walkable coordinate")
		mask[key] = true
	for key: String in tile_walkability:
		if bool(tile_walkability[key]) != mask.has(key): return _fail("static walkable mask differs from tile walkability")
	if typeof(value["static_props"]) != TYPE_ARRAY or value["static_props"].size() > MAX_STATIC_SCENE_PROPS: return false
	for prop: Variant in value["static_props"]:
		if not _fields(prop, ["id", "visual_family", "anchor", "layer"]) \
			or not _wire_label(prop["id"]) or not _wire_label(prop["visual_family"]) \
			or not _coord(prop["anchor"]) or not _integer(prop["layer"], -2147483648, 2147483647): return false
	if typeof(value["transition_apertures"]) != TYPE_ARRAY \
		or value["transition_apertures"].size() > MAX_STATIC_TRANSITION_APERTURES: return false
	for aperture: Variant in value["transition_apertures"]:
		if not _fields(aperture, ["at", "navigation", "target"]) or not _coord(aperture["at"]) \
			or not _navigation(aperture["navigation"]) or not _position(aperture["target"]): return false
	return true


func _static_scene_tile(value: Variant) -> bool:
	if not _fields(value, ["position", "terrain_ids", "walkable"]) \
		or not _coord(value["position"]) or not _boolean(value["walkable"]): return false
	if typeof(value["terrain_ids"]) != TYPE_ARRAY or value["terrain_ids"].is_empty(): return false
	for terrain_id: Variant in value["terrain_ids"]:
		if not _wire_label(terrain_id): return false
	return true


func _feedback_actor(value: Variant) -> bool:
	return _fields(value, ["actor_id", "name", "kind"]) and _actor_id(value["actor_id"]) \
		and _wire_label(value["name"]) and _enum(value["kind"], ["player", "monster", "npc"])


func _feedback_effect_change(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("feedback effect change must contain a kind")
	match value["kind"]:
		"applied", "ticked": return _fields(value, ["kind", "remaining_rounds"]) and (value["remaining_rounds"] == null or _integer(value["remaining_rounds"], 0, 4294967295))
		"expired", "removed": return _fields(value, ["kind"])
	return _fail("feedback effect change variant is invalid")


func _feedback_transaction_source(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("feedback transaction source must contain a kind")
	match value["kind"]:
		"skill_training": return _fields(value, ["kind", "service_id", "capability_id", "track_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["track_id"])
		"spell_learning": return _fields(value, ["kind", "service_id", "capability_id", "spell_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["spell_id"])
		"class_promotion": return _fields(value, ["kind", "service_id", "capability_id", "transaction_id", "target_class_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["transaction_id"]) and _wire_label(value["target_class_id"])
		"service_transaction": return _fields(value, ["kind", "service_id", "capability_id", "transaction_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["transaction_id"])
		"merchant_purchase":
			if not _fields(value, ["kind", "service_id", "capability_id", "item_instance_ids"]) or not _wire_label(value["service_id"]) or not _wire_label(value["capability_id"]): return false
			return typeof(value["item_instance_ids"]) == TYPE_ARRAY and value["item_instance_ids"].size() <= MAX_MERCHANT_PURCHASE_ITEMS and _array_of(value["item_instance_ids"], Callable(self, "_item_instance_id"))
		"merchant_sale": return _fields(value, ["kind", "service_id", "capability_id", "item_instance_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _item_instance_id(value["item_instance_id"])
		"item_service": return _fields(value, ["kind", "service_id", "capability_id", "operation", "item_instance_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _enum(value["operation"], ["appraise", "identify", "enchant_weapon"]) and _item_instance_id(value["item_instance_id"])
		"restoration_service": return _fields(value, ["kind", "service_id", "capability_id", "operation_id", "corpse_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["operation_id"]) and (value["corpse_id"] == null or (_printable_ascii(value["corpse_id"], 128) and _sequence_id(value["corpse_id"], "corpse:")))
		"npc_interaction": return _fields(value, ["kind", "npc_actor_id", "interaction_id"]) and _actor_id(value["npc_actor_id"]) and _wire_label(value["interaction_id"])
		"bank_deposit": return _fields(value, ["kind", "service_id", "capability_id", "bank_id", "gold_pile_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["bank_id"]) and _wire_label(value["gold_pile_id"]) and _sequence_id(value["gold_pile_id"], "gold:")
		"bank_withdrawal": return _fields(value, ["kind", "service_id", "capability_id", "bank_id", "amount"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["bank_id"]) and _decimal_i64(value["amount"])
	return _fail("feedback transaction source variant is invalid")


func _feedback_transaction_cost(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("feedback transaction cost must contain a kind")
	match value["kind"]:
		"carried_gold": return _fields(value, ["kind", "amount", "position", "before", "after"]) and _decimal_i64(value["amount"]) and _enum(value["position"], ["left_hand", "right_hand", "sack"]) and _decimal_i64(value["before"]) and _decimal_i64(value["after"])
		"ground_gold_pile": return _fields(value, ["kind", "gold_pile_id", "amount"]) and _wire_label(value["gold_pile_id"]) and _sequence_id(value["gold_pile_id"], "gold:") and _decimal_i64(value["amount"])
		"bank_balance": return _fields(value, ["kind", "bank_id", "amount", "before", "after"]) and _wire_label(value["bank_id"]) and _decimal_i64(value["amount"]) and _decimal_i64(value["before"]) and _decimal_i64(value["after"])
		"selected_carried_item": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "consumed_quantity", "remaining_quantity"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"]) and _integer(value["consumed_quantity"], 0, 4294967295) and _integer(value["remaining_quantity"], 0, 4294967295)
		"merchant_item": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "quantity", "pawn_listing_price_gold"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"]) and _integer(value["quantity"], 0, 4294967295) and _decimal_i64(value["pawn_listing_price_gold"])
	return _fail("feedback transaction cost variant is invalid")


func _feedback_transaction_reward(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("feedback transaction reward must contain a kind")
	match value["kind"]:
		"learning_rate": return _fields(value, ["kind", "track_id", "before", "after"]) and _wire_label(value["track_id"]) and _decimal_u64(value["before"]) and _decimal_u64(value["after"])
		"experience": return _fields(value, ["kind", "amount", "total_xp"]) and _i32(value["amount"]) and _decimal_i64(value["total_xp"])
		"item": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "position", "quantity"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"]) and _enum(value["position"], CARRIED_POSITIONS) and _integer(value["quantity"], 0, 4294967295)
		"class": return _fields(value, ["kind", "from_class_id", "from_class_display", "to_class_id", "to_class_display"]) and _wire_label(value["from_class_id"]) and _wire_label(value["from_class_display"]) and _wire_label(value["to_class_id"]) and _wire_label(value["to_class_display"])
		"spell": return _fields(value, ["kind", "spell_id", "learned_at_level"]) and _wire_label(value["spell_id"]) and _i32(value["learned_at_level"])
		"carried_gold": return _fields(value, ["kind", "amount", "position", "before", "after"]) and _decimal_i64(value["amount"]) and _enum(value["position"], ["left_hand", "right_hand", "sack"]) and _decimal_i64(value["before"]) and _decimal_i64(value["after"])
		"bank_balance": return _fields(value, ["kind", "bank_id", "amount", "before", "after"]) and _wire_label(value["bank_id"]) and _decimal_i64(value["amount"]) and _decimal_i64(value["before"]) and _decimal_i64(value["after"])
		"ground_gold_pile": return _fields(value, ["kind", "gold_pile_id", "amount"]) and _wire_label(value["gold_pile_id"]) and _sequence_id(value["gold_pile_id"], "gold:") and _decimal_i64(value["amount"])
		"merchant_item": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "quantity", "listing_price_gold"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"]) and _integer(value["quantity"], 0, 4294967295) and _decimal_i64(value["listing_price_gold"])
		"item_appraised": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "unit_value_gold", "total_value_gold"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"]) and _decimal_u64(value["unit_value_gold"]) and _decimal_u64(value["total_value_gold"])
		"item_identified": return _fields(value, ["kind", "item_instance_id", "item_definition_id"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"])
		"item_enchanted": return _fields(value, ["kind", "item_instance_id", "item_definition_id", "enchantment_instance_id", "combat_add_rating_bonus", "tags", "remaining_rounds"]) and _item_instance_id(value["item_instance_id"]) and _wire_label(value["item_definition_id"]) and _wire_label(value["enchantment_instance_id"]) and _i32(value["combat_add_rating_bonus"]) and _array_of(value["tags"], Callable(self, "_wire_label")) and (value["remaining_rounds"] == null or _integer(value["remaining_rounds"], 0, 4294967295))
		"resource_restored": return _fields(value, ["kind", "resource", "before", "after", "maximum"]) and _enum(value["resource"], ["hp", "mp", "stamina"]) and _i32(value["before"]) and _i32(value["after"]) and _i32(value["maximum"])
		"status_cured": return _fields(value, ["kind", "status", "removed_count"]) and _enum(value["status"], ["blindness", "poison"]) and _integer(value["removed_count"], 0, 4294967295)
		"priest_resurrection": return _fields(value, ["kind", "corpse_id", "method", "current_hp", "current_stamina"]) and _printable_ascii(value["corpse_id"], 128) and _sequence_id(value["corpse_id"], "corpse:") and _enum(value["method"], ["gods", "priest", "thaumaturge"]) and _i32(value["current_hp"]) and _i32(value["current_stamina"])
		"npc_interaction": return _fields(value, ["kind", "npc_actor_id", "interaction_id", "outcome"]) and _actor_id(value["npc_actor_id"]) and _wire_label(value["interaction_id"]) and _npc_interaction_outcome(value["outcome"])
		"quest_stage": return _fields(value, ["kind", "quest_id", "before_stage_id", "after_stage_id"]) and _wire_label(value["quest_id"]) and (value["before_stage_id"] == null or _wire_label(value["before_stage_id"])) and _wire_label(value["after_stage_id"])
	return _fail("feedback transaction reward variant is invalid")


func _feedback_cue(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("feedback cue must contain a kind")
	match value["kind"]:
		"physical_combat":
			if not _fields(value, ["kind", "source", "target", "location", "mode", "outcome"]) or (value["source"] != null and not _feedback_actor(value["source"])) or not _feedback_actor(value["target"]) or (value["location"] != null and not _position(value["location"])) or not _enum(value["mode"], ["fight", "kick", "jumpkick", "poke", "shoot", "throw"]): return false
			var outcome: Variant = value["outcome"]
			if typeof(outcome) != TYPE_DICTIONARY or not outcome.has("kind"): return _fail("physical outcome must contain a kind")
			match outcome["kind"]:
				"hit": return _fields(outcome, ["kind", "damage", "armor_reduction", "wound_before", "wound_after", "target_hp"]) and _i32(outcome["damage"]) and _i32(outcome["armor_reduction"]) and _enum(outcome["wound_before"], ["unhurt", "wounded", "badly_wounded", "near_death", "dead"]) and _enum(outcome["wound_after"], ["unhurt", "wounded", "badly_wounded", "near_death", "dead"]) and _i32(outcome["target_hp"])
				"missed", "blocked", "no_sight": return _fields(outcome, ["kind"])
				"not_ready": return _fields(outcome, ["kind", "current_time", "ready_at"]) and _decimal_u64(outcome["current_time"]) and _decimal_u64(outcome["ready_at"])
			return _fail("physical outcome variant is invalid")
		"weapon_fumbled": return _fields(value, ["kind", "actor", "mode", "result"]) and _feedback_actor(value["actor"]) and _enum(value["mode"], ["fight", "kick", "jumpkick", "poke", "shoot", "throw"]) and _enum(value["result"], ["dropped", "bow_unnocked"])
		"spell_lifecycle":
			if not _fields(value, ["kind", "actor", "spell_id", "spell_name", "state"]) or not _feedback_actor(value["actor"]) or not _wire_label(value["spell_id"]) or not _wire_label(value["spell_name"]): return false
			var state: Variant = value["state"]
			if typeof(state) != TYPE_DICTIONARY or not state.has("kind"): return _fail("spell lifecycle state must contain a kind")
			match state["kind"]:
				"warmed": return _fields(state, ["kind", "warmed_at", "ready_at"]) and _decimal_u64(state["warmed_at"]) and _decimal_u64(state["ready_at"])
				"ready": return _fields(state, ["kind", "ready_at"]) and _decimal_u64(state["ready_at"])
				"cast": return _fields(state, ["kind", "mp_cost", "stamina_cost"]) and (state["mp_cost"] == null or _i32(state["mp_cost"])) and (state["stamina_cost"] == null or _i32(state["stamina_cost"]))
				"fizzled": return _fields(state, ["kind", "reason"]) and _enum(state["reason"], ["replaced", "canceled", "rest", "healing_balm", "damage", "defeat"])
				"failed": return _fields(state, ["kind", "reason", "mp_cost", "stamina_cost"]) and _enum(state["reason"], ["invalid_path", "above_skill_attempt"]) and (state["mp_cost"] == null or _i32(state["mp_cost"])) and (state["stamina_cost"] == null or _i32(state["stamina_cost"]))
			return _fail("spell lifecycle state variant is invalid")
		"spell_impact":
			if not _fields(value, ["kind", "source", "spell_id", "spell_name", "target", "location", "outcome"]) or (value["source"] != null and not _feedback_actor(value["source"])) or not _wire_label(value["spell_id"]) or not _wire_label(value["spell_name"]) or not _feedback_actor(value["target"]) or not _position(value["location"]): return false
			var impact: Variant = value["outcome"]
			if typeof(impact) != TYPE_DICTIONARY or not impact.has("kind"): return _fail("spell impact must contain a kind")
			if impact["kind"] == "damaged": return _fields(impact, ["kind", "damage", "target_hp"]) and _i32(impact["damage"]) and _i32(impact["target_hp"])
			if impact["kind"] == "healed": return _fields(impact, ["kind", "amount", "target_hp"]) and _i32(impact["amount"]) and _i32(impact["target_hp"])
			return _fail("spell impact variant is invalid")
		"actor_effect": return _fields(value, ["kind", "actor", "location", "effect_id", "effect_kind", "change"]) and _feedback_actor(value["actor"]) and _position(value["location"]) and _wire_label(value["effect_id"]) and _wire_label(value["effect_kind"]) and _feedback_effect_change(value["change"])
		"tile_effect": return _fields(value, ["kind", "location", "effect_id", "effect_kind", "change"]) and _position(value["location"]) and _wire_label(value["effect_id"]) and _wire_label(value["effect_kind"]) and _feedback_effect_change(value["change"])
		"effect_damage": return _fields(value, ["kind", "actor", "location", "effect_id", "effect_kind", "damage", "actor_hp"]) and _feedback_actor(value["actor"]) and _position(value["location"]) and _wire_label(value["effect_id"]) and _wire_label(value["effect_kind"]) and _i32(value["damage"]) and _i32(value["actor_hp"])
		"resource": return _fields(value, ["kind", "actor", "resource", "reason", "amount", "current", "maximum"]) and _feedback_actor(value["actor"]) and _enum(value["resource"], ["hp", "mp", "stamina"]) and _enum(value["reason"], ["movement_spend", "physical_spend", "spell_cost", "regenerated", "restored", "balm"]) and _i32(value["amount"]) and (value["current"] == null or _i32(value["current"])) and _i32(value["maximum"])
		"transaction": return _fields(value, ["kind", "actor", "source", "costs", "rewards"]) and _feedback_actor(value["actor"]) and _feedback_transaction_source(value["source"]) and typeof(value["costs"]) == TYPE_ARRAY and value["costs"].size() <= MAX_FEEDBACK_TRANSACTION_COSTS and _array_of(value["costs"], Callable(self, "_feedback_transaction_cost")) and typeof(value["rewards"]) == TYPE_ARRAY and value["rewards"].size() <= MAX_FEEDBACK_TRANSACTION_REWARDS and _array_of(value["rewards"], Callable(self, "_feedback_transaction_reward"))
		"quest": return _fields(value, ["kind", "quest_id", "quest_title", "before_stage_id", "after_stage_id", "after_stage_label", "terminal"]) and _wire_label(value["quest_id"]) and _wire_label(value["quest_title"]) and (value["before_stage_id"] == null or _wire_label(value["before_stage_id"])) and _wire_label(value["after_stage_id"]) and _wire_label(value["after_stage_label"]) and _boolean(value["terminal"])
		"npc_message": return _fields(value, ["kind", "npc_actor_id", "npc_name", "interaction_id", "response"]) and _actor_id(value["npc_actor_id"]) and _wire_label(value["npc_name"]) and _wire_label(value["interaction_id"]) and _feedback_text(value["response"])
		"defeat": return _fields(value, ["kind", "actor", "location", "cause", "credited_source"]) and _feedback_actor(value["actor"]) and _position(value["location"]) and _enum(value["cause"], ["physical", "poison", "fire", "other_magic", "hazard"]) and (value["credited_source"] == null or _feedback_actor(value["credited_source"]))
		"corpse":
			if not _fields(value, ["kind", "corpse_id", "origin", "location", "change"]) or not _printable_ascii(value["corpse_id"], 128) or not _sequence_id(value["corpse_id"], "corpse:") or (value["origin"] != null and not _feedback_actor(value["origin"])) or not _position(value["location"]): return false
			var change: Variant = value["change"]
			if typeof(change) != TYPE_DICTIONARY or not change.has("kind"): return _fail("corpse change must contain a kind")
			if change["kind"] == "created": return _fields(change, ["kind"])
			if change["kind"] == "removed": return _fields(change, ["kind", "method"]) and _enum(change["method"], ["gods", "priest", "thaumaturge"])
			return _fail("corpse change variant is invalid")
		"life_state": return _fields(value, ["kind", "actor", "from", "to"]) and _feedback_actor(value["actor"]) and _enum(value["from"], ["alive", "ghost", "awaiting_resurrection", "dead"]) and _enum(value["to"], ["alive", "ghost", "awaiting_resurrection", "dead"])
		"resurrection": return _fields(value, ["kind", "actor", "corpse_id", "method", "destination", "current_hp", "current_stamina"]) and _feedback_actor(value["actor"]) and (value["corpse_id"] == null or (_printable_ascii(value["corpse_id"], 128) and _sequence_id(value["corpse_id"], "corpse:"))) and _enum(value["method"], ["gods", "priest", "thaumaturge"]) and _position(value["destination"]) and _i32(value["current_hp"]) and _i32(value["current_stamina"])
	return _fail("feedback cue variant is invalid")


func _observed_event(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("observed event must contain a kind")
	match value["kind"]:
		"actor_moved": return _fields(value, ["kind", "actor_id", "from", "to", "navigation"]) and _actor_id(value["actor_id"]) and _position(value["from"]) and _position(value["to"]) and _navigation(value["navigation"])
		"inspected":
			return _fields(value, ["kind", "location", "tile", "tile_move_cost", "exits", "nearby_actors", "ground_items"]) \
				and _position(value["location"]) and _wire_label(value["tile"]) \
				and (value["tile_move_cost"] == null or _integer(value["tile_move_cost"], -2147483648, 2147483647)) \
				and _array_of(value["exits"], Callable(self, "_inspect_exit")) \
				and _array_of(value["nearby_actors"], Callable(self, "_inspect_actor")) \
				and _array_of(value["ground_items"], Callable(self, "_inspect_ground_item"))
		"group_changed": return _fields(value, ["kind", "group_id"]) and _decimal_u64(value["group_id"])
		"group_invitation_changed": return _fields(value, ["kind", "invitation_id"]) and _decimal_u64(value["invitation_id"])
		"group_presence_changed": return _fields(value, ["kind", "group_id", "character_id", "connected"]) and _decimal_u64(value["group_id"]) and _uuid(value["character_id"], true) and _boolean(value["connected"])
		"player_follow_changed":
			if not _fields(value, ["kind", "follower_character_id"], ["target_character_id"]): return false
			return _uuid(value["follower_character_id"], true) and (not value.has("target_character_id") or value["target_character_id"] == null or _uuid(value["target_character_id"], true))
		"communication_preferences_changed": return _fields(value, ["kind"])
		"item_offer_changed": return _fields(value, ["kind", "item_instance_id"]) and _printable_ascii(value["item_instance_id"], 128)
		"defeat_reward_share": return _fields(value, ["kind", "character_id", "amount"]) and _uuid(value["character_id"], true) and _integer(value["amount"], -2147483648, 2147483647)
		"feedback": return _fields(value, ["kind", "cue"]) and _feedback_cue(value["cue"])
	return _fail("observed event variant is invalid")


func _observed_events(value: Variant) -> bool:
	return typeof(value) == TYPE_ARRAY and value.size() <= MAX_OBSERVED_EVENTS \
		and _array_of(value, Callable(self, "_observed_event"))


func _spell_target(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("spell target must contain a kind")
	match value["kind"]:
		"none", "self_target": return _fields(value, ["kind"])
		"actor": return _fields(value, ["kind", "actor_id"]) and _actor_id(value["actor_id"])
		"path": return _fields(value, ["kind", "directions"]) and _direction_path(value["directions"])
		"coordinate": return _fields(value, ["kind", "position"]) and _position(value["position"])
		"area": return _fields(value, ["kind", "center"]) and _position(value["center"])
		"direction", "door": return _fields(value, ["kind", "direction"]) and _enum(value["direction"], DIRECTIONS)
		"item": return _fields(value, ["kind", "item_instance_id", "location"]) and _printable_ascii(value["item_instance_id"], 128) and _enum(value["location"], ["sack", "active_equipment", "ground_here"])
	return _fail("spell target variant is invalid")


func _item_move_destination(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("item destination must contain a kind")
	if value["kind"] == "ground_here": return _fields(value, ["kind"])
	if value["kind"] == "carried": return _fields(value, ["kind", "position"]) and _enum(value["position"], CARRIED_POSITIONS)
	return _fail("item destination variant is invalid")


func _gold_source(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("gold source must contain a kind")
	if value["kind"] == "carried": return _fields(value, ["kind", "position"]) and _enum(value["position"], ["left_hand", "right_hand", "sack"])
	if value["kind"] == "ground": return _fields(value, ["kind", "gold_pile_id"]) and _wire_label(value["gold_pile_id"]) and _sequence_id(value["gold_pile_id"], "gold:")
	return _fail("gold source variant is invalid")


func _gold_destination(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("gold destination must contain a kind")
	if value["kind"] == "ground_here": return _fields(value, ["kind"])
	if value["kind"] == "carried": return _fields(value, ["kind", "position"]) and _enum(value["position"], ["left_hand", "right_hand", "sack"])
	return _fail("gold destination variant is invalid")


func _gold_quantity(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("gold quantity must contain a kind")
	if value["kind"] == "all": return _fields(value, ["kind"])
	if value["kind"] == "exact":
		if not _fields(value, ["kind", "amount"]) or not _decimal_i64(value["amount"]): return false
		if value["amount"] == "0" or _decimal_i64_negative(value["amount"]): return _fail("exact gold quantity must be positive")
		return true
	return _fail("gold quantity variant is invalid")


func _positive_decimal_i64(value: Variant) -> bool:
	return _decimal_i64(value) and value != "0" and not _decimal_i64_negative(value) \
		or _fail("decimal i64 amount must be positive")


func _merchant_purchase_items(value: Variant) -> bool:
	if typeof(value) != TYPE_ARRAY: return _fail("merchant purchase items must be an array")
	if value.size() > MAX_MERCHANT_PURCHASE_ITEMS: return _fail("merchant purchase items exceed the bounded maximum")
	var seen: Dictionary = {}
	for item_instance_id: Variant in value:
		if not _printable_ascii(item_instance_id, 128): return false
		if seen.has(item_instance_id): return _fail("merchant purchase items must be unique")
		seen[item_instance_id] = true
	return true


func _intent(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind") or typeof(value["kind"]) != TYPE_STRING:
		return _fail("intent must contain a string kind")
	match value["kind"]:
		"move_path": return _fields(value, ["kind", "path"]) and _direction_path(value["path"])
		"traverse": return _fields(value, ["kind", "traversal"]) and _enum(value["traversal"], ["stairs_up", "stairs_down", "climb_up", "climb_down"])
		"open", "close": return _fields(value, ["kind", "direction"]) and _enum(value["direction"], DIRECTIONS)
		"inspect", "hide", "show_sack", "wait", "rest", "nock", "unload_bow", "fizzle_warmed_spell", "leave_group", "disband_group", "end_follow": return _fields(value, ["kind"])
		"physical_attack": return _fields(value, ["kind", "mode", "target_actor_id", "authorization"]) and _enum(value["mode"], ["fight", "kick", "jumpkick", "poke", "shoot", "throw"]) and _actor_id(value["target_actor_id"]) and _enum(value["authorization"], ["safe", "confirmed_unsafe"])
		"warm_spell": return _fields(value, ["kind", "spell_id"]) and _wire_label(value["spell_id"])
		"cast_spell":
			if not _fields(value, ["kind", "spell_id", "target", "authorization"]): return false
			return _wire_label(value["spell_id"]) and (value["target"] == null or _spell_target(value["target"])) and _enum(value["authorization"], ["safe", "confirmed_unsafe"])
		"cast_warmed_spell":
			if not _fields(value, ["kind", "target", "authorization"]): return false
			return (value["target"] == null or _spell_target(value["target"])) and _enum(value["authorization"], ["safe", "confirmed_unsafe"])
		"search_corpse": return _fields(value, ["kind", "corpse_id"]) and _printable_ascii(value["corpse_id"], 128) and _sequence_id(value["corpse_id"], "corpse:")
		"move_item": return _fields(value, ["kind", "item_instance_id", "destination"]) and _printable_ascii(value["item_instance_id"], 128) and _item_move_destination(value["destination"])
		"move_gold": return _fields(value, ["kind", "source", "destination", "quantity"]) and _gold_source(value["source"]) and _gold_destination(value["destination"]) and _gold_quantity(value["quantity"])
		"deposit_bank_gold": return _fields(value, ["kind", "service_id", "capability_id", "gold_pile_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["gold_pile_id"])
		"withdraw_bank_gold": return _fields(value, ["kind", "service_id", "capability_id", "amount"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _positive_decimal_i64(value["amount"])
		"deposit_locker_item": return _fields(value, ["kind", "service_id", "capability_id", "item_instance_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _printable_ascii(value["item_instance_id"], 128)
		"withdraw_locker_item": return _fields(value, ["kind", "service_id", "capability_id", "item_instance_id", "destination"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _printable_ascii(value["item_instance_id"], 128) and _enum(value["destination"], CARRIED_POSITIONS)
		"drink_item": return _fields(value, ["kind", "item_instance_id"]) and _printable_ascii(value["item_instance_id"], 128)
		"train": return _fields(value, ["kind", "service_id", "offered_gold"]) and _wire_label(value["service_id"]) and _positive_decimal_i64(value["offered_gold"])
		"critique": return _fields(value, ["kind", "service_id", "track_id"]) and _wire_label(value["service_id"]) and _wire_label(value["track_id"])
		"promote_class": return _fields(value, ["kind", "target_class_id"]) and _wire_label(value["target_class_id"])
		"learn_spell": return _fields(value, ["kind", "spell_id"]) and _wire_label(value["spell_id"])
		"commit_service_transaction": return _fields(value, ["kind", "service_id", "capability_id", "transaction_id", "item_instance_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["transaction_id"]) and (value["item_instance_id"] == null or _printable_ascii(value["item_instance_id"], 128))
		"buy_from_merchant": return _fields(value, ["kind", "service_id", "capability_id", "item_instance_ids"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _merchant_purchase_items(value["item_instance_ids"])
		"sell_to_merchant": return _fields(value, ["kind", "service_id", "capability_id", "item_instance_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _printable_ascii(value["item_instance_id"], 128)
		"use_item_service": return _fields(value, ["kind", "service_id", "capability_id", "operation", "item_instance_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _enum(value["operation"], ["appraise", "identify", "enchant_weapon"]) and _printable_ascii(value["item_instance_id"], 128)
		"use_restoration_service": return _fields(value, ["kind", "service_id", "capability_id", "operation_id", "item_instance_id", "corpse_id"]) and _wire_label(value["service_id"]) and _wire_label(value["capability_id"]) and _wire_label(value["operation_id"]) and (value["item_instance_id"] == null or _printable_ascii(value["item_instance_id"], 128)) and (value["corpse_id"] == null or (_printable_ascii(value["corpse_id"], 128) and _sequence_id(value["corpse_id"], "corpse:")))
		"interact_with_npc": return _fields(value, ["kind", "npc_actor_id", "interaction_id", "item_instance_id"]) and _actor_id(value["npc_actor_id"]) and _wire_label(value["interaction_id"]) and (value["item_instance_id"] == null or _printable_ascii(value["item_instance_id"], 128))
		"clear_self_defense": return _fields(value, ["kind", "attacker_character_id"]) and _uuid(value["attacker_character_id"], true)
		"invite": return _fields(value, ["kind", "target_character_id"]) and _uuid(value["target_character_id"], true)
		"accept_invite", "decline_invite", "cancel_invite": return _fields(value, ["kind", "invitation_id"]) and _decimal_u64(value["invitation_id"])
		"remove_member", "transfer_leadership": return _fields(value, ["kind", "member_character_id"]) and _uuid(value["member_character_id"], true)
		"begin_follow": return _fields(value, ["kind", "target_character_id"]) and _uuid(value["target_character_id"], true)
		"set_pages_enabled": return _fields(value, ["kind", "enabled"]) and _boolean(value["enabled"])
		"block", "unblock": return _fields(value, ["kind", "target_character_id"]) and _uuid(value["target_character_id"], true)
		"offer_item": return _fields(value, ["kind", "recipient_character_id", "item_instance_id"]) and _uuid(value["recipient_character_id"], true) and _printable_ascii(value["item_instance_id"], 128)
		"accept_item_offer": return _fields(value, ["kind", "item_instance_id", "destination"]) and _printable_ascii(value["item_instance_id"], 128) and _enum(value["destination"], CARRIED_POSITIONS)
		"refuse_item_offer", "withdraw_item_offer": return _fields(value, ["kind", "item_instance_id"]) and _printable_ascii(value["item_instance_id"], 128)
	return _fail("intent variant is invalid")


func _client_hello(value: Variant) -> bool:
	if not _fields(value, ["kind", "ticket", "supported_minors"]): return false
	if value["kind"] != "client_hello" or not _secret_token(value["ticket"]): return _fail("client hello variant is invalid")
	if typeof(value["supported_minors"]) != TYPE_ARRAY: return _fail("supported minors must be an array")
	var minors: Array = value["supported_minors"] as Array
	if minors.is_empty() or minors.size() > MAX_SUPPORTED_MINORS: return _fail("supported minors must contain one to eight values")
	var seen: Dictionary = {}
	for minor: Variant in minors:
		if not _integer(minor, 0, 65535): return false
		if seen.has(minor): return _fail("supported minors must be unique")
		seen[minor] = true
	if minors.size() != 1 or minors[0] != PROTOCOL_MINOR: return _fail("client hello must offer exactly the current protocol minor")
	return true


func _client_command(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("client envelope must contain a kind")
	if value["kind"] == "command":
		return _fields(value, ["kind", "command_id", "control_epoch", "client_sequence", "observed_world_revision", "actor_id", "intent"]) \
			and _uuid(value["command_id"], true) and _decimal_u64(value["control_epoch"]) and _decimal_u64(value["client_sequence"]) \
			and _decimal_u64(value["observed_world_revision"]) and _actor_id(value["actor_id"]) and _intent(value["intent"])
	if value["kind"] == "path_preview":
		return _fields(value, ["kind", "preview_id", "control_epoch", "observed_world_revision", "actor_id", "path"]) \
			and _uuid(value["preview_id"], true) and _decimal_u64(value["control_epoch"]) \
			and _decimal_u64(value["observed_world_revision"]) \
			and _actor_id(value["actor_id"]) and _direction_path(value["path"])
	if value["kind"] == "social_message":
		return _fields(value, ["kind", "message_id", "control_epoch", "actor_id", "scope", "body"]) \
			and _uuid(value["message_id"], true) and _decimal_u64(value["control_epoch"]) \
			and _actor_id(value["actor_id"]) and _social_scope(value["scope"]) and _social_body(value["body"])
	return _fail("client envelope variant is invalid")


func _social_scope(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("social scope must contain a kind")
	if value["kind"] in ["say", "shout", "group"]: return _fields(value, ["kind"])
	if value["kind"] == "page": return _fields(value, ["kind", "target_character_id"]) and _uuid(value["target_character_id"], true)
	return _fail("social scope variant is invalid")


func _command_disposition(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("command disposition must contain a kind")
	if value["kind"] in ["accepted", "command_result_expired"]: return _fields(value, ["kind"])
	if value["kind"] == "rejected": return _fields(value, ["kind", "code"]) and _enum(value["code"], ["wrong_actor", "stale_control_epoch", "future_world_revision", "out_of_order_client_sequence", "rules_rejected", "projection_failed"])
	return _fail("command disposition variant is invalid")


func _path_preview_disposition(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("path preview disposition must contain a kind")
	if value["kind"] == "previewed": return _fields(value, ["kind"])
	if value["kind"] == "rejected": return _fields(value, ["kind", "code"]) and _enum(value["code"], ["wrong_actor", "stale_control_epoch", "future_world_revision", "rules_rejected"])
	return _fail("path preview disposition variant is invalid")


func _path_preview_step_outcome(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("path preview step outcome must contain a kind")
	if value["kind"] == "moved": return _fields(value, ["kind", "navigation"]) and _navigation(value["navigation"])
	if value["kind"] == "transitioned": return _fields(value, ["kind", "navigation", "to"]) and _navigation(value["navigation"]) and _position(value["to"])
	if value["kind"] == "blocked": return _fields(value, ["kind", "reason"]) and _enum(value["reason"], ["suppressed_by_status", "out_of_bounds", "blocked_terrain", "insufficient_movement_points"])
	return _fail("path preview step outcome variant is invalid")


func _path_preview_step(value: Variant) -> bool:
	if not _fields(value, ["index", "direction", "from", "attempted", "opens_door", "terrain_name", "cost", "remaining_points_after", "outcome"]): return false
	if not _decimal_u64(value["index"]) or not _enum(value["direction"], DIRECTIONS) or not _position(value["from"]) or not _position(value["attempted"]): return false
	if not _boolean(value["opens_door"]) or not _path_preview_step_outcome(value["outcome"]): return false
	if value["terrain_name"] != null and not _wire_label(value["terrain_name"]): return false
	for key: String in ["cost", "remaining_points_after"]:
		if value[key] != null and not _integer(value[key], -2147483648, 2147483647): return false
	return true


func _burden(value: Variant) -> bool:
	if not _fields(value, ["item_burden", "coin_burden", "total_burden", "lightly_loaded_limit", "moderately_loaded_limit", "heavily_loaded_limit", "tier"]): return false
	for key: String in ["item_burden", "coin_burden", "total_burden"]:
		if not _decimal_u64(value[key]): return false
	for key: String in ["lightly_loaded_limit", "moderately_loaded_limit", "heavily_loaded_limit"]:
		if value[key] != null and not _decimal_u64(value[key]): return false
	return value["tier"] == null or _enum(value["tier"], ["lightly_loaded", "moderately_loaded", "heavily_loaded", "very_heavily_loaded"])


func _decimal_u64_sum_matches(left: String, right: String, total: String) -> bool:
	var left_index: int = left.length() - 1
	var right_index: int = right.length() - 1
	var carry: int = 0
	var result: String = ""
	while left_index >= 0 or right_index >= 0 or carry > 0:
		var digit: int = carry
		if left_index >= 0:
			digit += left.unicode_at(left_index) - 0x30
			left_index -= 1
		if right_index >= 0:
			digit += right.unicode_at(right_index) - 0x30
			right_index -= 1
		result = String.chr(0x30 + digit % 10) + result
		carry = digit / 10
	if result.length() > 20 or (result.length() == 20 and result > "18446744073709551615"):
		return _fail("path preview burden total overflows u64")
	return result == total or _fail("path preview burden total is invalid")


func _path_preview(value: Variant) -> bool:
	if not _fields(value, ["contract_version", "actor_id", "start", "pace", "requested_path", "available_path_points", "accepted_steps", "steps", "stop_reason", "final_position", "remaining_path_points", "burden", "movement_exertion", "stamina_before", "stamina_cost", "stamina_after"]): return false
	if not _exact_int(value["contract_version"], 8) or not _actor_id(value["actor_id"]) or not _position(value["start"]): return false
	if not _enum(value["pace"], ["walk", "run", "sprint"]) or not _direction_path(value["requested_path"]): return false
	if value["pace"] != ["walk", "run", "sprint"][value["requested_path"].size() - 1]: return _fail("path preview pace is invalid")
	if not _integer(value["available_path_points"], -2147483648, 2147483647) or not _decimal_u64(value["accepted_steps"]): return false
	if value["accepted_steps"] not in ["0", "1", "2", "3"] or int(value["accepted_steps"]) > value["requested_path"].size(): return _fail("path preview accepted step count is invalid")
	if not _array_of(value["steps"], Callable(self, "_path_preview_step")) or value["steps"].size() > value["requested_path"].size() or int(value["accepted_steps"]) > value["steps"].size(): return false
	if not _enum(value["stop_reason"], ["full_path_accepted", "blocked", "transitioned", "zero_stamina_limit"]): return false
	if not _position(value["final_position"]) or not _integer(value["remaining_path_points"], -2147483648, 2147483647) or not _burden(value["burden"]): return false
	if not _decimal_u64_sum_matches(value["burden"]["item_burden"], value["burden"]["coin_burden"], value["burden"]["total_burden"]): return false
	var burden_shape: Array[bool] = [value["burden"]["lightly_loaded_limit"] != null, value["burden"]["moderately_loaded_limit"] != null, value["burden"]["heavily_loaded_limit"] != null, value["burden"]["tier"] != null]
	if burden_shape.has(true) and burden_shape.has(false): return _fail("path preview burden classification is incomplete")
	if not _enum(value["movement_exertion"], ["none", "normal", "rapid"]): return false
	var stamina_shape: Array[bool] = [value["stamina_before"] != null, value["stamina_cost"] != null, value["stamina_after"] != null]
	if stamina_shape.has(true) and stamina_shape.has(false): return _fail("path preview stamina classification is incomplete")
	for key: String in ["stamina_before", "stamina_cost", "stamina_after"]:
		if value[key] != null and not _integer(value[key], -2147483648, 2147483647): return false
	var current: Dictionary = value["start"]
	var accepted: int = 0
	var blocked_count: int = 0
	var last_remaining: int = value["available_path_points"]
	for index: int in range(value["steps"].size()):
		var step: Dictionary = value["steps"][index]
		if step["index"] != str(index) or step["direction"] != value["requested_path"][index] or step["from"] != current: return _fail("path preview step ordering is invalid")
		match step["outcome"]["kind"]:
			"moved":
				if step["terrain_name"] == null or step["cost"] == null or step["remaining_points_after"] == null or step["opens_door"]: return _fail("path preview moved-step facts are invalid")
				accepted += 1
				current = step["attempted"]
				last_remaining = step["remaining_points_after"]
			"transitioned":
				if step["terrain_name"] == null or step["cost"] == null or step["remaining_points_after"] == null: return _fail("path preview transition-step facts are invalid")
				if step["opens_door"] and step["outcome"]["navigation"] != "door": return _fail("only a door transition can open a door")
				accepted += 1
				current = step["outcome"]["to"]
				last_remaining = step["remaining_points_after"]
			"blocked":
				if step["terrain_name"] != null or step["cost"] != null or step["remaining_points_after"] != null or step["opens_door"]: return _fail("path preview blocked-step facts are invalid")
				blocked_count += 1
	if accepted != int(value["accepted_steps"]) or value["final_position"] != current or value["remaining_path_points"] != last_remaining: return _fail("path preview accepted prefix is inconsistent")
	var last_kind: String = "" if value["steps"].is_empty() else value["steps"][-1]["outcome"]["kind"]
	match value["stop_reason"]:
		"full_path_accepted":
			if value["steps"].size() != value["requested_path"].size() or blocked_count != 0: return _fail("path preview stop reason is invalid")
		"blocked":
			if last_kind != "blocked" or blocked_count != 1: return _fail("path preview stop reason is invalid")
		"transitioned":
			if last_kind != "transitioned" or blocked_count != 0: return _fail("path preview stop reason is invalid")
		"zero_stamina_limit":
			if value["steps"].size() >= value["requested_path"].size() or blocked_count != 0: return _fail("path preview stop reason is invalid")
	return true


func _server_envelope(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY or not value.has("kind"): return _fail("server envelope must contain a kind")
	match value["kind"]:
		"server_welcome":
			return _fields(value, ["kind", "selected_major", "selected_minor", "connection_id", "actor_id", "control_epoch", "server_sequence", "world_revision", "static_scene_context", "frame"]) \
				and _exact_int(value["selected_major"], PROTOCOL_MAJOR) and _exact_int(value["selected_minor"], PROTOCOL_MINOR) \
				and _uuid(value["connection_id"], false) and _actor_id(value["actor_id"]) \
				and _decimal_u64(value["control_epoch"]) and _decimal_u64(value["server_sequence"]) and _decimal_u64(value["world_revision"]) \
				and _static_scene_context(value["static_scene_context"]) and _observer_frame(value["frame"])
		"command_result":
			if not _fields(value, ["kind", "command_id", "disposition", "replay_status", "events", "events_truncated"], ["server_sequence", "before_revision", "after_revision"]): return false
			if not _uuid(value["command_id"], true) or not _command_disposition(value["disposition"]) or not _enum(value["replay_status"], ["new", "replayed"]): return false
			for key: String in ["server_sequence", "before_revision", "after_revision"]:
				if value.has(key) and value[key] != null and not _decimal_u64(value[key]): return false
			return _observed_events(value["events"]) and _boolean(value["events_truncated"])
		"path_preview_result":
			if not _fields(value, ["kind", "preview_id", "disposition", "control_epoch", "actor_id", "world_revision", "preview"]): return false
			if not _uuid(value["preview_id"], true) or not _path_preview_disposition(value["disposition"]) or not _decimal_u64(value["control_epoch"]): return false
			if not _actor_id(value["actor_id"]) or not _decimal_u64(value["world_revision"]): return false
			if value["disposition"]["kind"] == "previewed": return value["preview"] != null and _path_preview(value["preview"])
			return value["preview"] == null
		"state_update":
			return _fields(value, ["kind", "server_sequence", "world_revision", "events", "events_truncated", "static_scene_context", "frame"]) \
				and _decimal_u64(value["server_sequence"]) and _decimal_u64(value["world_revision"]) \
				and _observed_events(value["events"]) and _boolean(value["events_truncated"]) \
				and _static_scene_context(value["static_scene_context"]) and _observer_frame(value["frame"])
		"social_message": return _fields(value, ["kind", "message_id", "scope", "sender_character_id", "sender_name", "body"]) and _uuid(value["message_id"], true) and _social_scope(value["scope"]) and _uuid(value["sender_character_id"], true) and _display_name(value["sender_name"]) and _social_body(value["body"])
		"message_result": return _fields(value, ["kind", "message_id", "disposition"]) and _uuid(value["message_id"], true) and _enum(value["disposition"], ["accepted", "unavailable", "not_grouped", "rate_limited", "malformed"])
		"server_draining": return _fields(value, ["kind", "reason", "reconnect_hint"]) and _enum(value["reason"], ["shutdown", "control_replaced", "session_ended"]) and _boolean(value["reconnect_hint"])
		"error": return _fields(value, ["kind", "code"]) and _enum(value["code"], ["malformed_protocol", "binary_message", "oversized_protocol", "unsupported_version", "invalid_ticket", "expired_ticket", "consumed_ticket", "origin_rejected", "host_rejected", "hello_timeout", "capacity", "rate_limited", "queue_pressure", "command_in_progress", "gameplay_mark_locked", "unavailable", "draining"])
	return _fail("server envelope variant is invalid")


func _fields(value: Variant, required: Array, optional: Array = []) -> bool:
	if typeof(value) != TYPE_DICTIONARY: return _fail("value must be an object")
	var object: Dictionary = value as Dictionary
	for key: Variant in object.keys():
		if typeof(key) != TYPE_STRING or (key not in required and key not in optional): return _fail("object contains an unknown field")
	for key: Variant in required:
		if not object.has(key): return _fail("object is missing a required field")
	return true


func _array_of(value: Variant, validator: Callable) -> bool:
	if typeof(value) != TYPE_ARRAY: return _fail("value must be an array")
	for item: Variant in value:
		if not validator.call(item): return false
	return true


func _array_of_enum(value: Variant, allowed: Array) -> bool:
	if typeof(value) != TYPE_ARRAY: return _fail("value must be an array")
	for item: Variant in value:
		if not _enum(item, allowed): return false
	return true


func _direction_path(value: Variant) -> bool:
	if typeof(value) != TYPE_ARRAY: return _fail("path must be an array")
	var path: Array = value as Array
	if path.is_empty() or path.size() > MAX_MOVE_PATH_STEPS: return _fail("path must contain one to three directions")
	for direction: Variant in path:
		if not _enum(direction, DIRECTIONS): return false
	return true


func _enum(value: Variant, options: Variant) -> bool:
	if typeof(value) != TYPE_STRING or value not in options: return _fail("finite enum value is invalid")
	return true


func _integer(value: Variant, minimum: int, maximum: int) -> bool:
	if typeof(value) != TYPE_INT or value < minimum or value > maximum: return _fail("integer value is out of range")
	return true


func _i32(value: Variant) -> bool:
	return _integer(value, -2147483648, 2147483647)


func _exact_int(value: Variant, expected: int) -> bool:
	return _integer(value, expected, expected)


func _boolean(value: Variant) -> bool:
	if typeof(value) != TYPE_BOOL: return _fail("value must be boolean")
	return true


func _uuid(value: Variant, non_nil: bool) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("UUID must be a string")
	var text: String = value as String
	if text.length() != 36: return _fail("UUID must be canonical lowercase hyphenated text")
	for index: int in range(36):
		if index in [8, 13, 18, 23]:
			if text[index] != "-": return _fail("UUID must be canonical lowercase hyphenated text")
		elif text[index] not in "0123456789abcdef":
			return _fail("UUID must be canonical lowercase hyphenated text")
	if non_nil and text == "00000000-0000-0000-0000-000000000000": return _fail("UUID must be non-nil")
	return true


func _non_nil_uuid(value: Variant) -> bool:
	return _uuid(value, true)


func _decimal_u64(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("counter must be a canonical decimal string")
	var text: String = value as String
	if text.is_empty() or (text.length() > 1 and text.begins_with("0")) or not _ascii_digits(text): return _fail("counter must be a canonical decimal string")
	if text.length() > 20 or (text.length() == 20 and text > "18446744073709551615"): return _fail("counter is outside u64")
	return true


func _decimal_i64(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("signed quantity must be a canonical decimal string")
	var text: String = value as String
	var negative: bool = text.begins_with("-")
	var digits: String = text.substr(1) if negative else text
	if digits.is_empty() or (digits.length() > 1 and digits.begins_with("0")) or not _ascii_digits(digits) or text == "-0": return _fail("signed quantity must be a canonical decimal string")
	var maximum: String = "9223372036854775808" if negative else "9223372036854775807"
	if digits.length() > 19 or (digits.length() == 19 and digits > maximum): return _fail("signed quantity is outside i64")
	return true


func _decimal_i64_negative(value: Variant) -> bool:
	return typeof(value) == TYPE_STRING and (value as String).begins_with("-")


func _ascii_digits(text: String) -> bool:
	for index: int in range(text.length()):
		if text.unicode_at(index) < 0x30 or text.unicode_at(index) > 0x39: return false
	return true


func _printable_ascii(value: Variant, maximum_bytes: int) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("bounded label must be a string")
	var text: String = value as String
	if text.is_empty() or text.to_utf8_buffer().size() > maximum_bytes: return _fail("bounded label has invalid length")
	for index: int in range(text.length()):
		var scalar: int = text.unicode_at(index)
		if scalar < 0x20 or scalar > 0x7e: return _fail("bounded label must be printable ASCII")
	return true


func _actor_id(value: Variant) -> bool: return _printable_ascii(value, 64)
func _wire_label(value: Variant) -> bool: return _printable_ascii(value, 64)
func _item_instance_id(value: Variant) -> bool: return _printable_ascii(value, 128)


func _username(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("username must be a string")
	var text: String = value as String
	if text.length() < 3 or text.length() > 32: return _fail("username is not canonical")
	for index: int in range(text.length()):
		var character: String = text[index]
		if character not in "abcdefghijklmnopqrstuvwxyz0123456789_": return _fail("username is not canonical")
	if text[0] == "_" or text[text.length() - 1] == "_": return _fail("username is not canonical")
	return true


func _password(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING or (value as String).to_utf8_buffer().size() > MAX_CONTROL_INPUT_BYTES: return _fail("password has invalid shape")
	return true


func _display_name(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("display name must be a string")
	var text: String = value as String
	if text.is_empty() or text.length() > 64: return _fail("display name has invalid length")
	for index: int in range(text.length()):
		var scalar: int = text.unicode_at(index)
		if scalar < 0x20 or (scalar >= 0x7f and scalar <= 0x9f): return _fail("display name contains a control scalar")
	return true


func _social_body(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("social body must be a string")
	var text: String = value as String
	if text.is_empty() or text.length() > MAX_SOCIAL_BODY_SCALARS or text.to_utf8_buffer().size() > MAX_SOCIAL_BODY_BYTES: return _fail("social body has invalid length")
	for index: int in range(text.length()):
		var scalar: int = text.unicode_at(index)
		if scalar < 0x20 or (scalar >= 0x7f and scalar <= 0x9f): return _fail("social body contains a control scalar")
	return true


func _feedback_text(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("feedback text must be a string")
	var text: String = value as String
	if text.is_empty() or text.length() > 280 or text.to_utf8_buffer().size() > 1024: return _fail("feedback text has invalid length")
	for index: int in range(text.length()):
		var scalar: int = text.unicode_at(index)
		if scalar < 0x20 or (scalar >= 0x7f and scalar <= 0x9f): return _fail("feedback text contains a control scalar")
	return true


func _secret_token(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("secret token must be a string")
	var text: String = value as String
	if text.length() != 43: return _fail("secret token has invalid length")
	for index: int in range(text.length()):
		if text[index] not in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_": return _fail("secret token is not base64url text")
	var last_value: int = _base64url_value(text[text.length() - 1])
	if last_value < 0 or last_value % 4 != 0: return _fail("secret token has noncanonical trailing bits")
	return true


func _base64url_value(character: String) -> int:
	var code: int = character.unicode_at(0)
	if code >= 0x41 and code <= 0x5a: return code - 0x41
	if code >= 0x61 and code <= 0x7a: return code - 0x61 + 26
	if code >= 0x30 and code <= 0x39: return code - 0x30 + 52
	if character == "-": return 62
	if character == "_": return 63
	return -1


func _sequence_id(value: Variant, prefix: String) -> bool:
	if typeof(value) != TYPE_STRING: return _fail("sequence identity must be a string")
	var text: String = value as String
	if not text.begins_with(prefix): return _fail("sequence identity has an invalid prefix")
	var suffix: String = text.substr(prefix.length())
	if suffix.is_empty() or suffix.begins_with("0") or not _ascii_digits(suffix): return _fail("sequence identity is not canonical")
	return true


func _fail(message: String) -> bool:
	_error = message
	return false


func _failure(message: String) -> Dictionary:
	return {"ok": false, "value": null, "error": message}
