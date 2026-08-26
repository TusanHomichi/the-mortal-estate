class_name FeedbackPresenter
extends RefCounted

const MAX_FEEDBACK_ENTRIES: int = 200
const MAX_CHAT_ENTRIES: int = 200
const MAX_CUE_QUEUE: int = 8
const MAX_PENDING_XP_PAIRS: int = 32

var feedback_entries: Array[Dictionary] = []
var chat_entries: Array[Dictionary] = []
var cue_queue: Array[Dictionary] = []

var _seen_event_ids: Dictionary = {}
var _seen_message_ids: Dictionary = {}
var _seen_message_results: Dictionary = {}
var _seen_command_results: Dictionary = {}
var _pending_xp_pairs: Dictionary = {}
var _pending_xp_order: Array[String] = []
var _active_xp_connection_id: String = ""
var _latest_xp_total: String = ""
var _feedback_gate: Callable


func configure_feedback_gate(gate: Callable) -> void:
	_feedback_gate = gate


func consume_server_envelope(envelope: Dictionary, connection_id: String = "") -> Dictionary:
	var result: Dictionary = {"feedback": [], "chat": []}
	_prepare_xp_connection(connection_id)
	(result["feedback"] as Array).append_array(_consume_defeat_reward_share(envelope, connection_id))
	match str(envelope.get("kind", "")):
		"command_result":
			var command_id: String = str(envelope.get("command_id", ""))
			var command_key: String = "command:" + command_id
			if not _seen_command_results.has(command_key):
				_seen_command_results[command_key] = true
				var disposition: Dictionary = envelope.get("disposition", {})
				var command_entry: Dictionary = _command_result_entry(disposition)
				_append_feedback(command_entry)
				(result["feedback"] as Array).append(command_entry)
			var server_sequence: String = str(envelope.get("server_sequence", ""))
			var prefix: String = _event_prefix(connection_id, server_sequence) if not server_sequence.is_empty() else command_key
			(result["feedback"] as Array).append_array(consume_observed_events(envelope.get("events", []), prefix))
		"state_update":
			var prefix: String = _event_prefix(connection_id, str(envelope.get("server_sequence", "")))
			(result["feedback"] as Array).append_array(consume_observed_events(envelope.get("events", []), prefix))
		"social_message":
			var entry: Dictionary = consume_social_message(envelope)
			if not entry.is_empty():
				(result["chat"] as Array).append(entry)
		"message_result":
			var entry: Dictionary = consume_message_result(envelope)
			if not entry.is_empty():
				(result["chat"] as Array).append(entry)
	_record_authoritative_xp_total(envelope)
	return result


func consume_observed_events(events: Array, identity_prefix: String = "") -> Array[Dictionary]:
	var added: Array[Dictionary] = []
	for index: int in range(events.size()):
		var event_value: Variant = events[index]
		if not event_value is Dictionary:
			continue
		var event: Dictionary = event_value as Dictionary
		if event.get("kind") != "feedback" or not event.get("cue") is Dictionary:
			continue
		var identity: String = "" if identity_prefix.is_empty() else "%s:%d" % [identity_prefix, index]
		if not identity.is_empty() and _seen_event_ids.has(identity):
			continue
		if not identity.is_empty():
			_seen_event_ids[identity] = true
		var cue: Dictionary = event["cue"] as Dictionary
		var entry: Dictionary = format_cue(cue)
		if _feedback_gate.is_valid() and not bool(_feedback_gate.call(entry.duplicate(true), cue.duplicate(true), identity)):
			continue
		_append_feedback(entry)
		added.append(entry)
	return added


func consume_social_message(envelope: Dictionary) -> Dictionary:
	var message_id: String = str(envelope.get("message_id", ""))
	if message_id.is_empty() or _seen_message_ids.has(message_id):
		return {}
	_seen_message_ids[message_id] = true
	var scope: Dictionary = envelope.get("scope", {})
	var scope_name: String = str(scope.get("kind", "message"))
	var entry: Dictionary = {
		"category": "chat",
		"icon": "[" + scope_name.to_upper() + "]",
		"text": "%s %s: %s" % ["[" + scope_name.to_upper() + "]", envelope.get("sender_name", "Unknown"), envelope.get("body", "")],
		"message_id": message_id,
		"scope": scope_name,
	}
	_append_chat(entry)
	return entry


func consume_message_result(envelope: Dictionary) -> Dictionary:
	var message_id: String = str(envelope.get("message_id", ""))
	if message_id.is_empty() or _seen_message_results.has(message_id):
		return {}
	_seen_message_results[message_id] = true
	var disposition: String = str(envelope.get("disposition", "malformed"))
	var description: String = {
		"accepted": "Accepted for routing; delivery is not confirmed.",
		"unavailable": "Message unavailable; nothing was delivered.",
		"not_grouped": "Group message rejected: you are not grouped.",
		"rate_limited": "Message rejected: rate limited.",
		"malformed": "Message rejected as malformed.",
	}.get(disposition, "Message result: " + disposition)
	var entry: Dictionary = {
		"category": "system",
		"icon": "[SEND]",
		"text": "[SEND] " + description,
		"message_id": message_id,
		"disposition": disposition,
	}
	_append_chat(entry)
	return entry


func format_cue(cue: Dictionary) -> Dictionary:
	var kind: String = str(cue.get("kind", "unknown"))
	var category: String = _category_for_kind(kind)
	var icon: String = _icon_for_kind(kind)
	var text_value: String
	match kind:
		"physical_combat": text_value = _physical_combat_text(cue)
		"weapon_fumbled": text_value = _weapon_fumble_text(cue)
		"spell_lifecycle": text_value = _spell_lifecycle_text(cue)
		"spell_impact": text_value = _spell_impact_text(cue)
		"actor_effect": text_value = _actor_effect_text(cue)
		"tile_effect": text_value = _tile_effect_text(cue)
		"effect_damage": text_value = _effect_damage_text(cue)
		"resource": text_value = _resource_text(cue)
		"transaction": text_value = _transaction_text(cue)
		"quest": text_value = _quest_text(cue)
		"npc_message": text_value = _npc_message_text(cue)
		"defeat": text_value = _defeat_text(cue)
		"corpse": text_value = _corpse_text(cue)
		"life_state": text_value = _life_state_text(cue)
		"resurrection": text_value = _resurrection_text(cue)
		_: text_value = "Unknown feedback cue: " + kind
	return {
		"category": category,
		"icon": icon,
		"text": "%s %s" % [icon, text_value],
		"high_salience": kind in ["physical_combat", "weapon_fumbled", "spell_lifecycle", "spell_impact", "quest", "npc_message", "defeat", "life_state", "resurrection"],
		"kind": kind,
	}


func discard() -> void:
	feedback_entries.clear()
	chat_entries.clear()
	cue_queue.clear()
	_seen_event_ids.clear()
	_seen_message_ids.clear()
	_seen_message_results.clear()
	_seen_command_results.clear()
	_pending_xp_pairs.clear()
	_pending_xp_order.clear()
	_active_xp_connection_id = ""
	_latest_xp_total = ""


func take_cue_of_kind(kind: String) -> Dictionary:
	for index: int in range(cue_queue.size() - 1, -1, -1):
		var entry: Dictionary = cue_queue[index]
		if entry.get("kind") == kind:
			cue_queue.remove_at(index)
			return entry
	return {}


func append_deferred_feedback(entry: Dictionary) -> void:
	_append_feedback(entry.duplicate(true))


func _append_feedback(entry: Dictionary) -> void:
	feedback_entries.append(entry)
	while feedback_entries.size() > MAX_FEEDBACK_ENTRIES:
		feedback_entries.pop_front()
	if bool(entry.get("high_salience", false)):
		cue_queue.append(entry)
		while cue_queue.size() > MAX_CUE_QUEUE:
			cue_queue.pop_front()


func _append_chat(entry: Dictionary) -> void:
	chat_entries.append(entry)
	while chat_entries.size() > MAX_CHAT_ENTRIES:
		chat_entries.pop_front()


func _event_prefix(connection_id: String, server_sequence: String) -> String:
	return "event:%s:%s" % [connection_id, server_sequence]


func _prepare_xp_connection(connection_id: String) -> void:
	if connection_id.is_empty() or connection_id == _active_xp_connection_id:
		return
	_pending_xp_pairs.clear()
	_pending_xp_order.clear()
	_latest_xp_total = ""
	_active_xp_connection_id = connection_id


func _record_authoritative_xp_total(envelope: Dictionary) -> void:
	var frame_fact: Dictionary = _xp_frame_fact(envelope)
	if not frame_fact.is_empty():
		_latest_xp_total = str(frame_fact["total"])


func _xp_frame_fact(envelope: Dictionary) -> Dictionary:
	var frame_value: Variant = envelope.get("frame")
	if not frame_value is Dictionary:
		return {}
	var frame: Dictionary = frame_value as Dictionary
	var social: Dictionary = frame.get("social", {})
	var character: Dictionary = frame.get("character", {})
	var progression: Dictionary = character.get("progression", {})
	var total_value: Variant = progression.get("experience")
	if total_value == null:
		return {}
	return {
		"character_id": str(social.get("character_id", "")),
		"total": str(total_value),
	}


func _consume_defeat_reward_share(envelope: Dictionary, connection_id: String) -> Array[Dictionary]:
	var added: Array[Dictionary] = []
	var kind: String = str(envelope.get("kind", ""))
	var server_sequence: String = str(envelope.get("server_sequence", ""))
	if kind not in ["command_result", "state_update"] or connection_id.is_empty() or server_sequence.is_empty():
		return added
	var pair_key: String = "%s:%s" % [connection_id, server_sequence]
	var prefix: String = _event_prefix(connection_id, server_sequence)
	var share: Dictionary = {}
	var events: Array = envelope.get("events", [])
	for index: int in range(events.size()):
		var event_value: Variant = events[index]
		if not event_value is Dictionary:
			continue
		var event: Dictionary = event_value as Dictionary
		if event.get("kind") != "defeat_reward_share":
			continue
		var event_identity: String = "%s:%d" % [prefix, index]
		if _seen_event_ids.has(event_identity):
			continue
		_seen_event_ids[event_identity] = true
		share = {
			"character_id": str(event.get("character_id", "")),
			"amount": int(event.get("amount", 0)),
		}
		break

	if share.is_empty() and not _pending_xp_pairs.has(pair_key):
		return added
	var pair: Dictionary = _pending_xp_pairs.get(pair_key, {})
	if pair.is_empty():
		_pending_xp_order.append(pair_key)
	if not share.is_empty():
		pair["share"] = share
	var frame_fact: Dictionary = _xp_frame_fact(envelope)
	if not frame_fact.is_empty():
		pair["frame"] = frame_fact
	_pending_xp_pairs[pair_key] = pair
	_trim_pending_xp_pairs()

	if not pair.has("share") or not pair.has("frame"):
		return added
	var paired_share: Dictionary = pair["share"] as Dictionary
	var paired_frame: Dictionary = pair["frame"] as Dictionary
	_remove_pending_xp_pair(pair_key)
	if paired_share.get("character_id") != paired_frame.get("character_id"):
		return added
	var amount: int = int(paired_share.get("amount", 0))
	if amount > 0 and not _latest_xp_total.is_empty() and str(paired_frame.get("total")) == _latest_xp_total:
		return added
	var amount_text: String = ("+" if amount > 0 else "") + str(amount)
	var entry: Dictionary = {
		"category": "system",
		"icon": "[XP]",
		"text": "[XP] %s XP; total %s." % [amount_text, paired_frame.get("total")],
		"high_salience": false,
		"kind": "experience",
	}
	_append_feedback(entry)
	added.append(entry)
	return added


func _trim_pending_xp_pairs() -> void:
	while _pending_xp_order.size() > MAX_PENDING_XP_PAIRS:
		var expired_key: String = _pending_xp_order.pop_front()
		_pending_xp_pairs.erase(expired_key)


func _remove_pending_xp_pair(pair_key: String) -> void:
	_pending_xp_pairs.erase(pair_key)
	_pending_xp_order.erase(pair_key)


func _command_result_entry(disposition: Dictionary) -> Dictionary:
	var kind: String = str(disposition.get("kind", "rejected"))
	var detail: String = kind.replace("_", " ")
	if kind == "rejected":
		detail += ": " + str(disposition.get("code", "unknown"))
	return {"category": "system", "icon": "[COMMAND]", "text": "[COMMAND] " + detail.capitalize(), "high_salience": kind == "rejected", "kind": "command_result"}


func _category_for_kind(kind: String) -> String:
	if kind in ["physical_combat", "weapon_fumbled", "spell_lifecycle", "spell_impact", "actor_effect", "tile_effect", "effect_damage", "defeat"]:
		return "combat"
	if kind in ["quest", "npc_message"]:
		return "quest"
	return "system"


func _icon_for_kind(kind: String) -> String:
	return {
		"physical_combat": "[COMBAT]",
		"weapon_fumbled": "[FUMBLE]",
		"spell_lifecycle": "[SPELL]",
		"spell_impact": "[MAGIC]",
		"actor_effect": "[STATUS]",
		"tile_effect": "[AREA]",
		"effect_damage": "[DAMAGE]",
		"resource": "[RESOURCE]",
		"transaction": "[TRADE]",
		"quest": "[QUEST]",
		"npc_message": "[NPC]",
		"defeat": "[DEFEAT]",
		"corpse": "[CORPSE]",
		"life_state": "[LIFE]",
		"resurrection": "[RECOVERY]",
	}.get(kind, "[SYSTEM]")


func _physical_combat_text(cue: Dictionary) -> String:
	var source: String = _actor_name(cue.get("source"), "Unseen source")
	var target: String = _actor_name(cue.get("target"), "Unknown target")
	var mode: String = _words(cue.get("mode", "attack"))
	var outcome: Dictionary = cue.get("outcome", {})
	match str(outcome.get("kind", "")):
		"hit":
			return "%s hits %s with %s for %s damage; armor absorbs %s; HP %s; %s to %s." % [source, target, mode, outcome.get("damage", 0), outcome.get("armor_reduction", 0), outcome.get("target_hp", 0), _words(outcome.get("wound_before", "unknown")), _words(outcome.get("wound_after", "unknown"))]
		"missed": return "%s misses %s with %s." % [source, target, mode]
		"blocked": return "%s's %s against %s is blocked." % [source, mode, target]
		"no_sight": return "%s cannot see %s for %s." % [source, target, mode]
		"not_ready": return "%s is not ready for %s against %s; world T%s, ready T%s." % [source, mode, target, outcome.get("current_time", "?"), outcome.get("ready_at", "?")]
	return "%s attempts %s against %s." % [source, mode, target]


func _weapon_fumble_text(cue: Dictionary) -> String:
	return "%s fumbles %s: %s." % [_actor_name(cue.get("actor")), _words(cue.get("mode", "attack")), _words(cue.get("result", "fumbled"))]


func _spell_lifecycle_text(cue: Dictionary) -> String:
	var actor: String = _actor_name(cue.get("actor"))
	var spell: String = str(cue.get("spell_name", cue.get("spell_id", "spell")))
	var state: Dictionary = cue.get("state", {})
	match str(state.get("kind", "")):
		"warmed": return "%s warms %s at T%s; ready T%s." % [actor, spell, state.get("warmed_at", "?"), state.get("ready_at", "?")]
		"ready": return "%s has %s ready at T%s." % [actor, spell, state.get("ready_at", "?")]
		"cast": return "%s casts %s%s." % [actor, spell, _cost_suffix(state)]
		"fizzled": return "%s's %s fizzles: %s." % [actor, spell, _words(state.get("reason", "unknown"))]
		"failed": return "%s's %s fails: %s%s." % [actor, spell, _words(state.get("reason", "unknown")), _cost_suffix(state)]
	return "%s changes %s state." % [actor, spell]


func _spell_impact_text(cue: Dictionary) -> String:
	var source: String = _actor_name(cue.get("source"), "Unseen caster")
	var target: String = _actor_name(cue.get("target"), "Unknown target")
	var spell: String = str(cue.get("spell_name", cue.get("spell_id", "spell")))
	var outcome: Dictionary = cue.get("outcome", {})
	if outcome.get("kind") == "damaged":
		return "%s's %s damages %s for %s; HP %s." % [source, spell, target, outcome.get("damage", 0), outcome.get("target_hp", 0)]
	if outcome.get("kind") == "healed":
		return "%s's %s heals %s for %s; HP %s." % [source, spell, target, outcome.get("amount", 0), outcome.get("target_hp", 0)]
	return "%s's %s affects %s." % [source, spell, target]


func _actor_effect_text(cue: Dictionary) -> String:
	return "%s status %s %s%s." % [_actor_name(cue.get("actor")), _words(cue.get("effect_kind", "effect")), _effect_change_verb(cue.get("change", {})), _remaining_suffix(cue.get("change", {}))]


func _tile_effect_text(cue: Dictionary) -> String:
	return "%s area effect %s%s at %s." % [_words(cue.get("effect_kind", "effect")), _effect_change_verb(cue.get("change", {})), _remaining_suffix(cue.get("change", {})), _position_text(cue.get("location", {}))]


func _effect_damage_text(cue: Dictionary) -> String:
	return "%s takes %s %s damage; HP %s." % [_actor_name(cue.get("actor")), cue.get("damage", 0), _words(cue.get("effect_kind", "effect")), cue.get("actor_hp", 0)]


func _resource_text(cue: Dictionary) -> String:
	var current: String = "current unknown" if cue.get("current") == null else "%s/%s" % [cue.get("current"), cue.get("maximum", "?")]
	return "%s %s changes by %s from %s; %s." % [_actor_name(cue.get("actor")), str(cue.get("resource", "resource")).to_upper(), cue.get("amount", 0), _words(cue.get("reason", "unknown")), current]


func _transaction_text(cue: Dictionary) -> String:
	var costs: Array[String] = []
	for value: Variant in cue.get("costs", []):
		costs.append(_transaction_cost_text(value as Dictionary))
	var rewards: Array[String] = []
	for value: Variant in cue.get("rewards", []):
		rewards.append(_transaction_reward_text(value as Dictionary))
	return "%s completes %s. Costs: %s. Rewards: %s." % [_actor_name(cue.get("actor")), _transaction_source_text(cue.get("source", {})), "none" if costs.is_empty() else ", ".join(costs), "none" if rewards.is_empty() else ", ".join(rewards)]


func _quest_text(cue: Dictionary) -> String:
	var completion: String = " completed" if bool(cue.get("terminal", false)) else " advanced"
	return "%s%s to %s: %s." % [cue.get("quest_title", cue.get("quest_id", "Quest")), completion, cue.get("after_stage_id", "?"), cue.get("after_stage_label", "")]


func _npc_message_text(cue: Dictionary) -> String:
	return "%s: %s" % [cue.get("npc_name", cue.get("npc_actor_id", "NPC")), cue.get("response", "")]


func _defeat_text(cue: Dictionary) -> String:
	var credit: String = _actor_name(cue.get("credited_source"), "no visible credited source")
	return "%s is defeated by %s; credit: %s." % [_actor_name(cue.get("actor")), _words(cue.get("cause", "unknown")), credit]


func _corpse_text(cue: Dictionary) -> String:
	var change: Dictionary = cue.get("change", {})
	if change.get("kind") == "removed":
		return "Corpse %s is removed by %s at %s." % [cue.get("corpse_id", "?"), _words(change.get("method", "unknown")), _position_text(cue.get("location", {}))]
	return "%s's corpse is created at %s." % [_actor_name(cue.get("origin"), "Unknown actor"), _position_text(cue.get("location", {}))]


func _life_state_text(cue: Dictionary) -> String:
	return "%s changes from %s to %s." % [_actor_name(cue.get("actor")), _words(cue.get("from", "unknown")), _words(cue.get("to", "unknown"))]


func _resurrection_text(cue: Dictionary) -> String:
	return "%s returns by %s at %s with HP %s and stamina %s." % [_actor_name(cue.get("actor")), _words(cue.get("method", "unknown")), _position_text(cue.get("destination", {})), cue.get("current_hp", 0), cue.get("current_stamina", 0)]


func _transaction_source_text(source_value: Variant) -> String:
	var source: Dictionary = source_value as Dictionary
	match str(source.get("kind", "transaction")):
		"skill_training": return "training in " + str(source.get("track_id", "skill"))
		"spell_learning": return "learning " + str(source.get("spell_id", "spell"))
		"class_promotion": return "promotion to " + str(source.get("target_class_id", "class"))
		"service_transaction": return "service " + str(source.get("transaction_id", "transaction"))
		"merchant_purchase": return "merchant purchase"
		"merchant_sale": return "merchant sale"
		"item_service": return _words(source.get("operation", "item service"))
		"restoration_service": return "restoration " + str(source.get("operation_id", "service"))
		"npc_interaction": return "NPC interaction " + str(source.get("interaction_id", "interaction"))
		"bank_deposit": return "bank deposit"
		"bank_withdrawal": return "bank withdrawal"
	return _words(source.get("kind", "transaction"))


func _transaction_cost_text(cost: Dictionary) -> String:
	match str(cost.get("kind", "cost")):
		"carried_gold": return "%s carried gold (%s to %s)" % [cost.get("amount", "?"), cost.get("before", "?"), cost.get("after", "?")]
		"ground_gold_pile": return "%s ground gold" % cost.get("amount", "?")
		"bank_balance": return "%s bank gold (%s to %s)" % [cost.get("amount", "?"), cost.get("before", "?"), cost.get("after", "?")]
		"selected_carried_item": return "%s x %s" % [cost.get("consumed_quantity", "?"), cost.get("item_definition_id", "item")]
		"merchant_item": return "%s x %s" % [cost.get("quantity", "?"), cost.get("item_definition_id", "item")]
	return _words(cost.get("kind", "cost"))


func _transaction_reward_text(reward: Dictionary) -> String:
	match str(reward.get("kind", "reward")):
		"learning_rate": return "%s learning %s to %s" % [reward.get("track_id", "skill"), reward.get("before", "?"), reward.get("after", "?")]
		"experience": return "%s XP; total %s" % [reward.get("amount", 0), reward.get("total_xp", "?")]
		"item": return "%s x %s" % [reward.get("quantity", "?"), reward.get("item_definition_id", "item")]
		"class": return "%s to %s" % [reward.get("from_class_display", "class"), reward.get("to_class_display", "class")]
		"spell": return "spell " + str(reward.get("spell_id", "?"))
		"carried_gold": return "%s carried gold (%s to %s)" % [reward.get("amount", "?"), reward.get("before", "?"), reward.get("after", "?")]
		"bank_balance": return "%s bank gold (%s to %s)" % [reward.get("amount", "?"), reward.get("before", "?"), reward.get("after", "?")]
		"ground_gold_pile": return "%s ground gold" % reward.get("amount", "?")
		"merchant_item": return "%s x %s listed" % [reward.get("quantity", "?"), reward.get("item_definition_id", "item")]
		"item_appraised": return "%s appraised at %s each" % [reward.get("item_definition_id", "item"), reward.get("unit_value_gold", "?")]
		"item_identified": return str(reward.get("item_definition_id", "item")) + " identified"
		"item_enchanted": return str(reward.get("item_definition_id", "item")) + " enchanted"
		"resource_restored": return "%s restored %s to %s/%s" % [str(reward.get("resource", "resource")).to_upper(), reward.get("before", "?"), reward.get("after", "?"), reward.get("maximum", "?")]
		"status_cured": return "%s cured x%s" % [_words(reward.get("status", "status")), reward.get("removed_count", 0)]
		"priest_resurrection": return "resurrection by " + _words(reward.get("method", "priest"))
		"npc_interaction": return "NPC interaction " + str(reward.get("interaction_id", "interaction"))
		"quest_stage": return "%s to %s" % [reward.get("quest_id", "quest"), reward.get("after_stage_id", "stage")]
	return _words(reward.get("kind", "reward"))


func _cost_suffix(state: Dictionary) -> String:
	var costs: Array[String] = []
	if state.get("mp_cost") != null:
		costs.append("%s MP" % state.get("mp_cost"))
	if state.get("stamina_cost") != null:
		costs.append("%s stamina" % state.get("stamina_cost"))
	return "" if costs.is_empty() else " for " + " and ".join(costs)


func _effect_change_verb(change_value: Variant) -> String:
	return {
		"applied": "is applied",
		"ticked": "ticks",
		"expired": "expires",
		"removed": "is removed",
	}.get(str((change_value as Dictionary).get("kind", "")), "changes")


func _remaining_suffix(change_value: Variant) -> String:
	var change: Dictionary = change_value as Dictionary
	if not change.has("remaining_rounds") or change.get("remaining_rounds") == null:
		return ""
	return "; %s rounds remain" % change.get("remaining_rounds")


func _actor_name(value: Variant, fallback: String = "Unknown actor") -> String:
	if value is Dictionary:
		return str((value as Dictionary).get("name", fallback))
	return fallback


func _position_text(value: Variant) -> String:
	if not value is Dictionary:
		return "unknown location"
	var position: Dictionary = (value as Dictionary).get("position", {})
	return "%s/%s (%s,%s)" % [(value as Dictionary).get("realm", "?"), (value as Dictionary).get("level", "?"), position.get("x", "?"), position.get("y", "?")]


func _words(value: Variant) -> String:
	return str(value).replace("_", " ")
