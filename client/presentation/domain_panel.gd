class_name DomainPanel
extends PanelContainer

signal close_requested
signal intent_requested(intent: Dictionary, label: String, origin: Control)
signal drop_destination_requested(destination_position: String)

const MODES: Array[String] = ["Character", "Inventory", "Magic", "Services & Quests", "Social"]
const SPELL_PALETTE_SCENE: PackedScene = preload("res://presentation/SpellPalette.tscn")
const PAPER_DOLL_POSITIONS: Array[String] = [
	"left_hand", "right_hand",
	"head", "neck", "left_arm", "right_arm", "gloves", "inner_armor", "outer_armor", "boots",
	"belt_1", "belt_2", "belt_3", "belt_4", "belt_back",
	"left_finger_1", "left_finger_2", "left_finger_3", "left_finger_4",
	"right_finger_1", "right_finger_2", "right_finger_3", "right_finger_4",
]

@onready var mode_selector: OptionButton = %ModeSelector
@onready var title_label: Label = %DomainTitle
@onready var content: VBoxContainer = %DomainContent
@onready var close_button: Button = %CloseButton

var _frame: Dictionary = {}
var _online: bool = false
var _command_pending: bool = false
var _action_buttons: Array[Button] = []
var _rendered_option_keys: Dictionary = {}
var _mode_index: int = 0
var _selected_spell_id: String = ""
var _selected_actor_id: String = ""
var _selected_item_target: Dictionary = {}
var _selected_coordinate: Dictionary = {}
var _selected_path: Array[String] = []
var _target_source: String = "actor"
var _authorization: String = "safe"
var _drop_zone_buttons: Array[Button] = []
var _drop_status_label: Label
var _ground_drag_active: bool = false
var _dragged_item_name: String = ""
var _rebuild_queued: bool = false


func _ready() -> void:
	for mode: String in MODES:
		mode_selector.add_item(mode)
	mode_selector.item_selected.connect(_on_mode_selected)
	close_button.pressed.connect(func() -> void: close_requested.emit())
	_rebuild()


func open_panel() -> void:
	visible = true
	mode_selector.grab_focus()


func present_frame(frame: Dictionary) -> void:
	_frame = frame.duplicate(true)
	clear_target_selection(false)
	var spell_ids: Array[String] = known_spell_ids()
	if not _selected_spell_id in spell_ids:
		_selected_spell_id = spell_ids[0] if not spell_ids.is_empty() else ""
	_rebuild()


func set_connection_state(online: bool) -> void:
	_online = online
	_update_action_buttons()


func set_command_pending(pending: bool) -> void:
	_command_pending = pending
	_update_action_buttons()
	_update_drop_zones()


func set_ground_drag_active(active: bool, item_name: String = "") -> void:
	_ground_drag_active = active
	_dragged_item_name = item_name if active else ""
	_update_drop_zones()


func drop_zone_buttons() -> Array[Button]:
	return _drop_zone_buttons


func drop_destination_at(global_position: Vector2) -> String:
	for button: Button in _drop_zone_buttons:
		if not button.disabled and button.visible and button.get_global_rect().has_point(global_position):
			return str(button.get_meta("drop_destination", ""))
	return ""


func set_world_target(coordinate: Vector2i, path: Array[String]) -> void:
	_selected_coordinate = {"x": coordinate.x, "y": coordinate.y}
	_selected_path = path.duplicate()
	if current_mode() == "Magic":
		_request_rebuild()


func clear_target_selection(rebuild: bool = true) -> void:
	_selected_actor_id = ""
	_selected_item_target.clear()
	_selected_coordinate.clear()
	_selected_path.clear()
	_target_source = "actor"
	_authorization = "safe"
	if rebuild and is_node_ready() and current_mode() == "Magic":
		_request_rebuild()


func set_actor_target(actor_id: String) -> void:
	_selected_actor_id = actor_id if _projected_actor_ids().has(actor_id) else ""
	_target_source = "actor"
	if current_mode() == "Magic":
		_request_rebuild()


func set_item_target(item_instance_id: String) -> void:
	_selected_item_target = _projected_item_targets().get(item_instance_id, {}).duplicate(true)
	if current_mode() == "Magic":
		_request_rebuild()


func set_authorization(value: String) -> void:
	_authorization = value if value in ["safe", "confirmed_unsafe"] else "safe"
	if current_mode() == "Magic":
		_request_rebuild()


func set_target_source(value: String) -> void:
	_target_source = value if value in ["actor", "path"] else "actor"
	if current_mode() == "Magic":
		_request_rebuild()


func select_spell(spell_id: String) -> void:
	_selected_spell_id = spell_id if known_spell_ids().has(spell_id) else ""
	if current_mode() == "Magic":
		_request_rebuild()


func select_mode(mode_name: String) -> void:
	var index: int = MODES.find(mode_name)
	if index < 0:
		return
	_mode_index = index
	if is_node_ready():
		mode_selector.select(index)
		_rebuild()


func current_mode() -> String:
	return MODES[_mode_index]


func known_spell_ids() -> Array[String]:
	var result: Array[String] = []
	for value: Variant in _frame.get("spell_actions", []):
		var spell_id: String = str((value as Dictionary).get("spell_id", ""))
		if not spell_id.is_empty():
			result.append(spell_id)
	return result


func build_selected_cast_intent() -> Dictionary:
	var spell: Dictionary = _spell_action(_selected_spell_id)
	var cast: Dictionary = spell.get("cast", {})
	if spell.is_empty() or not bool(cast.get("enabled", false)):
		return {}
	if cast.get("intent") is Dictionary:
		return (cast["intent"] as Dictionary).duplicate(true)
	if not bool(cast.get("requires_target_selection", false)):
		return {}
	var target: Dictionary = _selected_spell_target(spell)
	if target.is_empty():
		return {}
	if spell.get("casting_method") == "direct":
		return {"kind": "cast_spell", "spell_id": _selected_spell_id, "target": target, "authorization": _authorization}
	return {"kind": "cast_warmed_spell", "target": target, "authorization": _authorization}


func rendered_text() -> String:
	var values: Array[String] = []
	_collect_control_text(content, values)
	return "\n".join(values)


func action_button_texts() -> Array[String]:
	var values: Array[String] = []
	for button: Button in _action_buttons:
		values.append(button.text)
	return values


func first_focus_control() -> Control:
	return mode_selector


func _on_mode_selected(index: int) -> void:
	_mode_index = clampi(index, 0, MODES.size() - 1)
	_rebuild()


func _request_rebuild() -> void:
	if _rebuild_queued:
		return
	_rebuild_queued = true
	call_deferred("_run_queued_rebuild")


func _run_queued_rebuild() -> void:
	if not _rebuild_queued:
		return
	if not is_node_ready():
		return
	_rebuild_queued = false
	_rebuild()


func _rebuild() -> void:
	if not is_node_ready():
		return
	for child: Node in content.get_children():
		content.remove_child(child)
		child.free()
	_action_buttons.clear()
	_drop_zone_buttons.clear()
	_drop_status_label = null
	_rendered_option_keys.clear()
	title_label.text = current_mode()
	match current_mode():
		"Character":
			_render_character()
		"Inventory":
			_render_inventory()
		"Magic":
			_render_magic()
		"Services & Quests":
			_render_services_and_quests()
		"Social":
			_render_social()
	_update_action_buttons()


func _render_character() -> void:
	var character: Dictionary = _frame.get("character", {})
	if character.is_empty():
		_add_label("No authoritative character projection.")
		return
	var identity: Dictionary = character.get("identity", {})
	_add_heading(str(identity.get("display_class", "Character")))
	_add_label("Base %s · Current %s · Nationality %s" % [identity.get("base_class_id", "—"), identity.get("current_class_id", "—"), identity.get("nationality_id", "—")])
	_add_label("Alignment %s · Karma %s" % [character.get("alignment", "—"), character.get("karma_points", "—")])
	var attributes: Dictionary = character.get("attributes", {})
	_add_heading("Attributes")
	_add_label("STR %s · DEX %s · CON %s · INT %s · WIS %s · CHA %s" % [attributes.get("strength", "—"), attributes.get("dexterity", "—"), attributes.get("constitution", "—"), attributes.get("intelligence", "—"), attributes.get("wisdom", "—"), attributes.get("charisma", "—")])
	var resources: Dictionary = character.get("resources", {})
	_add_heading("Resources")
	_add_label("HP %s/%s (peak %s) · MP %s/%s · Stamina %s/%s" % [resources.get("hp", "—"), resources.get("max_hp", "—"), resources.get("peak_hp", "—"), resources.get("mp", "—"), resources.get("max_mp", "—"), resources.get("stamina", "—"), resources.get("max_stamina", "—")])
	var progression: Dictionary = character.get("progression", {})
	_add_heading("Progression")
	_add_label("Level %s · XP %s · Pending %s" % [progression.get("level", "—"), progression.get("experience", "—"), "—" if progression.get("pending_target_level") == null else progression.get("pending_target_level")])
	var adds: Dictionary = character.get("physical_attribute_adds", {})
	_add_label("Physical adds: STR %s · DEX %s" % [adds.get("strength_adds", "—"), adds.get("dexterity_adds", "—")])
	_add_heading("Skills")
	var skills: Array = character.get("skill_ledger", [])
	if skills.is_empty():
		_add_label("No learned skill rows.")
	for value: Variant in skills:
		var skill: Dictionary = value as Dictionary
		_add_label("%s · level %s %s · critique %s · practice %s · rate %s" % [skill.get("track_display", skill.get("track_id", "Skill")), skill.get("level", "—"), skill.get("level_title", ""), skill.get("critique_rank", "—"), skill.get("practice_points", "—"), skill.get("learning_rate", "—")])
	_add_mode_actions(["hide", "show_sack", "rest", "train", "critique", "promote_class"])


func _render_inventory() -> void:
	var occupied: Dictionary = {}
	var sack: Array[String] = []
	for value: Variant in _frame.get("carried", {}).get("items", []):
		var row: Dictionary = value as Dictionary
		var position: String = str(row.get("position", ""))
		var item: Dictionary = row.get("item", {})
		if position.begins_with("sack_item_"):
			sack.append("%s · %s ×%s" % [
				position.replace("_", " ").capitalize(),
				item.get("name", "Unknown item"),
				item.get("quantity", 1),
			])
		else:
			occupied[position] = item.duplicate(true)
	_add_heading("Sack dock")
	var sack_button: Button = _add_drop_zone(
		content,
		"sack",
		"Sack · %d occupied%s" % [
		sack.size(),
		"" if _frame.get("carried", {}).get("sack_capacity") == null else " / %s capacity" % str(_frame.get("carried", {}).get("sack_capacity")),
		],
		{},
	)
	sack_button.name = "SackDropZone"
	for card: String in sack:
		_add_label(card)
	_drop_status_label = _add_label("")
	_add_heading("Paper doll · drop zones")
	var doll: GridContainer = GridContainer.new()
	doll.name = "PaperDollDropZones"
	doll.columns = 4
	doll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_child(doll)
	for position: String in PAPER_DOLL_POSITIONS:
		_add_drop_zone(doll, position, _position_label(position), occupied.get(position, {}))
	_update_drop_zones()
	var gold: Dictionary = _frame.get("carried", {}).get("gold", {})
	_add_heading("Gold")
	_add_label("Left hand %s · Right hand %s · Sack %s" % [gold.get("left_hand", "0"), gold.get("right_hand", "0"), gold.get("sack", "0")])
	var burden: Dictionary = _frame.get("burden", {})
	_add_heading("Burden")
	_add_label("%s · items %s + coins %s = %s · limits %s/%s/%s" % [str(burden.get("tier", "unknown")).replace("_", " "), burden.get("item_burden", "—"), burden.get("coin_burden", "—"), burden.get("total_burden", "—"), burden.get("lightly_loaded_limit", "—"), burden.get("moderately_loaded_limit", "—"), burden.get("heavily_loaded_limit", "—")])
	_render_item_summary("Ground here", _frame.get("ground_items", []))
	_render_offer_items("Incoming offers", _frame.get("incoming_item_offers", []))
	_render_offer_items("Outgoing offers", _frame.get("outgoing_item_offers", []))
	_add_heading("Inventory and storage")
	_add_mode_actions(["move_item", "move_gold", "search_corpse", "drink_item", "deposit_bank_gold", "withdraw_bank_gold", "deposit_locker_item", "withdraw_locker_item", "offer_item", "accept_item_offer", "refuse_item_offer", "withdraw_item_offer"])
	for service_value: Variant in _frame.get("services_here", []):
		_collect_and_add_options(service_value, ["merchant", "bank", "locker", "item_service"])
	for offer_value: Variant in _frame.get("incoming_item_offers", []):
		_collect_and_add_options(offer_value)
	for offer_value: Variant in _frame.get("outgoing_item_offers", []):
		_collect_and_add_options(offer_value)


func _render_magic() -> void:
	_add_heading("Ready spells")
	var palette: SpellPalette = SPELL_PALETTE_SCENE.instantiate() as SpellPalette
	palette.name = "MagicSpellPalette"
	palette.custom_minimum_size.y = 150.0
	content.add_child(palette)
	palette.present_rows(_frame.get("spell_actions", []), _selected_spell_id, "", _command_pending)
	palette.spell_primary_activated.connect(select_spell)
	var warmed: Variant = _frame.get("warmed_spell")
	_add_heading("Warmed spell")
	if warmed is Dictionary:
		_add_label("%s · %s · warmed T%s · ready T%s" % [warmed.get("spell_id", "—"), warmed.get("status", "—"), warmed.get("warmed_at", "—"), warmed.get("ready_at", "—")])
	else:
		_add_label("None")
	_add_heading("Intensity")
	var intensity: Button = Button.new()
	intensity.text = "Intensity / Power [Unavailable: no current rules contract]"
	intensity.disabled = true
	intensity.tooltip_text = "Unavailable: FD owns unresolved power and chant evidence."
	intensity.accessibility_description = intensity.tooltip_text
	content.add_child(intensity)
	var spell: Dictionary = _spell_action(_selected_spell_id)
	if spell.is_empty():
		_add_label("Choose a ready spell.")
		_add_heading("Known spellbook")
		_render_known_spell_details()
		return
	_add_heading(str(spell.get("spell_name", _selected_spell_id)))
	_add_label("%s · %s · target %s · MP %s · stamina %s" % [str(spell.get("casting_method", "—")).replace("_", " "), str(spell.get("cast_class", "—")).replace("_", " "), "—" if spell.get("target_kind") == null else spell.get("target_kind"), "—" if spell.get("mp_cost") == null else spell.get("mp_cost"), "—" if spell.get("stamina_cost") == null else spell.get("stamina_cost")])
	_add_label("Hostile act: %s · Town-law violation: %s" % ["yes" if spell.get("hostile_act", false) else "no", "yes" if spell.get("town_law_violation", false) else "no"])
	_add_spell_target_controls(spell)
	_add_named_state_option("Warm", spell.get("warm", {}))
	var cast: Dictionary = spell.get("cast", {})
	if cast.get("intent") is Dictionary:
		_add_named_state_option("Cast", cast)
	elif bool(cast.get("enabled", false)) and bool(cast.get("requires_target_selection", false)):
		var intent: Dictionary = build_selected_cast_intent()
		_add_authored_cast_button(intent)
	else:
		_add_named_state_option("Cast", cast)
	_add_mode_actions(["fizzle_warmed_spell"])
	_add_heading("Known spellbook")
	_render_known_spell_details()


func _render_known_spell_details() -> void:
	var known: Array = _frame.get("character", {}).get("known_spells", [])
	if known.is_empty():
		_add_label("No detailed known-spell rows.")
		return
	for value: Variant in known:
		var row: Dictionary = value as Dictionary
		_add_label("%s · %s · learned level %s" % [
			row.get("spell_id", "Unknown spell"),
			str(row.get("lane", "unknown")).replace("_", " "),
			row.get("learned_at_level", "—"),
		])


func _render_services_and_quests() -> void:
	var services: Array = _frame.get("services_here", [])
	if services.is_empty():
		_add_label("No services at the current authoritative coordinate.")
	for service_value: Variant in services:
		var service: Dictionary = service_value as Dictionary
		_add_heading("%s (%s)" % [service.get("name", "Service"), service.get("service_id", "?")])
		for capability_value: Variant in service.get("capabilities", []):
			var capability: Dictionary = capability_value as Dictionary
			_render_capability(capability)
	var npcs: Array = _frame.get("npcs_here", [])
	_add_heading("NPCs here")
	if npcs.is_empty():
		_add_label("None")
	for npc_value: Variant in npcs:
		var npc: Dictionary = npc_value as Dictionary
		_add_label("%s (%s) · follows %s" % [npc.get("name", "NPC"), npc.get("actor_id", "?"), "nobody" if npc.get("following_character_id") == null else npc.get("following_character_id")])
		for interaction_value: Variant in npc.get("interactions", []):
			var interaction: Dictionary = interaction_value as Dictionary
			_add_label("Interaction: %s (%s)" % [interaction.get("label", "Interaction"), interaction.get("interaction_id", "?")])
			_collect_and_add_options(interaction)
	var quests: Array = _frame.get("quest_log", [])
	_add_heading("Quest log")
	if quests.is_empty():
		_add_label("No active quest rows.")
	for quest_value: Variant in quests:
		var quest: Dictionary = quest_value as Dictionary
		_add_label("%s · %s: %s%s" % [quest.get("quest_title", quest.get("quest_id", "Quest")), quest.get("stage_id", "?"), quest.get("stage_label", ""), " · terminal" if quest.get("terminal", false) else ""])


func _render_social() -> void:
	var social: Dictionary = _frame.get("social", {})
	_add_heading("Current character")
	_add_label(str(social.get("character_id", "No social projection")))
	var group: Variant = social.get("group")
	_add_heading("Group")
	if group is Dictionary:
		_add_label("Group %s · leader %s" % [group.get("group_id", "?"), group.get("leader_character_id", "?")])
		for member_value: Variant in group.get("members", []):
			var member: Dictionary = member_value as Dictionary
			_add_label("%s · %s%s" % [member.get("character_id", "?"), "connected" if member.get("connected", false) else "absent", " · leader" if member.get("character_id") == group.get("leader_character_id") else ""])
	else:
		_add_label("Not grouped")
	_add_heading("Invitations")
	_add_label("Incoming %d · Outgoing %d" % [social.get("incoming_invitations", []).size(), social.get("outgoing_invitations", []).size()])
	for invitation_value: Variant in social.get("incoming_invitations", []):
		var invitation: Dictionary = invitation_value as Dictionary
		_add_label("From %s · invite %s · expires T%s" % [invitation.get("issuer_character_id", "?"), invitation.get("invitation_id", "?"), invitation.get("expires_at", "?")])
	_add_heading("Presence and preferences")
	_add_label("Following %s · Pages %s" % ["nobody" if social.get("following_character_id") == null else social.get("following_character_id"), "enabled" if social.get("pages_enabled", false) else "disabled"])
	_add_label("Blocked: %s" % [", ".join(PackedStringArray(social.get("blocked_character_ids", []))) if not social.get("blocked_character_ids", []).is_empty() else "none"])
	_add_heading("Visible player characters")
	var visible_players: Array[String] = []
	for actor_value: Variant in _frame.get("actors", []):
		var actor: Dictionary = actor_value as Dictionary
		if actor.get("character_id") != null:
			visible_players.append("%s (%s)" % [actor.get("name", "Player"), actor.get("character_id")])
	_add_label(", ".join(PackedStringArray(visible_players)) if not visible_players.is_empty() else "None")
	_render_offer_items("Incoming offers", _frame.get("incoming_item_offers", []))
	_render_offer_items("Outgoing offers", _frame.get("outgoing_item_offers", []))
	_add_heading("Exact social actions")
	_add_mode_actions(["invite", "accept_invite", "decline_invite", "cancel_invite", "leave_group", "remove_member", "disband_group", "transfer_leadership", "begin_follow", "end_follow", "set_pages_enabled", "block", "unblock", "offer_item", "accept_item_offer", "refuse_item_offer", "withdraw_item_offer"])
	for offer_value: Variant in _frame.get("incoming_item_offers", []):
		_collect_and_add_options(offer_value)
	for offer_value: Variant in _frame.get("outgoing_item_offers", []):
		_collect_and_add_options(offer_value)


func _render_capability(capability: Dictionary) -> void:
	var kind: String = str(capability.get("kind", "capability"))
	_add_label("◆ %s (%s)" % [kind.replace("_", " ").capitalize(), capability.get("capability_id", "?")])
	match kind:
		"merchant":
			for listing_value: Variant in capability.get("listings", []):
				var listing: Dictionary = listing_value as Dictionary
				var item: Dictionary = listing.get("item", {})
				_add_label("%s ×%s · %s gold · %s" % [item.get("name", "Item"), item.get("quantity", 1), listing.get("price_gold", "?"), listing.get("origin", "unknown")])
			for sale_value: Variant in capability.get("sales", []):
				var sale: Dictionary = sale_value as Dictionary
				_add_label("Sale option: %s" % sale.get("label", "Sell"))
		"bank":
			_add_label("Bank %s · balance %s · cap %s" % [capability.get("bank_id", "?"), capability.get("balance_gold", "?"), capability.get("transaction_cap_gold", "?")])
		"locker":
			_add_label("Locker %s · %s/%s items" % [capability.get("vault_id", "?"), capability.get("item_count", "?"), capability.get("capacity", "?")])
			_render_item_summary("Locker contents", capability.get("items", []))
		"spell_teaching":
			_add_label("Spells: %s" % ", ".join(PackedStringArray(capability.get("spell_ids", []))))
		"skill_training":
			_add_label("Tracks: %s · selected %s" % [", ".join(PackedStringArray(capability.get("offered_track_ids", []))), capability.get("selected_track_id", "—")])
	_collect_and_add_options(capability)


func _add_spell_target_controls(spell: Dictionary) -> void:
	var cast: Dictionary = spell.get("cast", {})
	if not bool(cast.get("requires_target_selection", false)):
		return
	_add_heading("Current target")
	var cast_class: String = str(spell.get("cast_class", ""))
	var target_kind: String = str(spell.get("target_kind", ""))
	if cast_class == "path_or_character":
		var source: OptionButton = OptionButton.new()
		source.add_item("Projected actor")
		source.add_item("Clicked path")
		source.select(1 if _target_source == "path" else 0)
		source.item_selected.connect(func(index: int) -> void: set_target_source("path" if index == 1 else "actor"))
		content.add_child(source)
	if target_kind == "actor" or cast_class in ["character", "path_or_character"]:
		var actors: OptionButton = OptionButton.new()
		actors.add_item("Choose projected actor…")
		var actor_ids: Array[String] = _projected_actor_ids()
		for actor_id: String in actor_ids:
			actors.add_item(_actor_label(actor_id))
		if actor_ids.has(_selected_actor_id):
			actors.select(actor_ids.find(_selected_actor_id) + 1)
		actors.item_selected.connect(func(index: int) -> void: set_actor_target(actor_ids[index - 1] if index > 0 else ""))
		content.add_child(actors)
	if target_kind == "item":
		var items: OptionButton = OptionButton.new()
		items.add_item("Choose projected item…")
		var item_targets: Dictionary = _projected_item_targets()
		var item_ids: Array[String] = []
		for key: Variant in item_targets.keys():
			item_ids.append(str(key))
		item_ids.sort()
		for item_id: String in item_ids:
			items.add_item("%s (%s)" % [item_targets[item_id].get("name", "Item"), item_id])
		if item_targets.has(_selected_item_target.get("item_instance_id", "")):
			items.select(item_ids.find(_selected_item_target["item_instance_id"]) + 1)
		items.item_selected.connect(func(index: int) -> void: set_item_target(item_ids[index - 1] if index > 0 else ""))
		content.add_child(items)
	if target_kind in ["coordinate", "area"]:
		_add_label("Clicked visible square: %s" % ("%s,%s" % [_selected_coordinate.get("x"), _selected_coordinate.get("y")] if not _selected_coordinate.is_empty() else "none"))
	if target_kind in ["direction", "door"] or cast_class in ["path", "path_or_character"]:
		_add_label("Clicked direct path: %s" % (" → ".join(PackedStringArray(_selected_path)) if not _selected_path.is_empty() else "none"))
	var authorization: OptionButton = OptionButton.new()
	authorization.add_item("Authorization: Safe")
	authorization.add_item("Authorization: Confirm unsafe")
	authorization.select(1 if _authorization == "confirmed_unsafe" else 0)
	authorization.item_selected.connect(func(index: int) -> void: set_authorization("confirmed_unsafe" if index == 1 else "safe"))
	content.add_child(authorization)


func _selected_spell_target(spell: Dictionary) -> Dictionary:
	var target_kind: String = str(spell.get("target_kind", ""))
	var cast_class: String = str(spell.get("cast_class", ""))
	if cast_class == "path_or_character":
		if _target_source == "path":
			return {"kind": "path", "directions": _selected_path.duplicate()} if not _selected_path.is_empty() else {}
		return {"kind": "actor", "actor_id": _selected_actor_id} if not _selected_actor_id.is_empty() else {}
	if cast_class == "path":
		return {"kind": "path", "directions": _selected_path.duplicate()} if not _selected_path.is_empty() else {}
	if cast_class == "character" and target_kind.is_empty():
		return {"kind": "actor", "actor_id": _selected_actor_id} if not _selected_actor_id.is_empty() else {}
	match target_kind:
		"actor":
			return {"kind": "actor", "actor_id": _selected_actor_id} if not _selected_actor_id.is_empty() else {}
		"coordinate":
			return {"kind": "coordinate", "position": _selected_coordinate.duplicate()} if not _selected_coordinate.is_empty() else {}
		"area":
			return {"kind": "area", "center": _selected_coordinate.duplicate()} if not _selected_coordinate.is_empty() else {}
		"direction", "door":
			return {"kind": target_kind, "direction": _selected_path[0]} if not _selected_path.is_empty() else {}
		"item":
			if _selected_item_target.is_empty():
				return {}
			return {"kind": "item", "item_instance_id": _selected_item_target["item_instance_id"], "location": _selected_item_target["location"]}
		"self":
			return {"kind": "self"}
		"none":
			return {"kind": "none"}
	return {}


func _add_authored_cast_button(intent: Dictionary) -> void:
	var option: Dictionary = {"label": "Cast at current target", "enabled": not intent.is_empty(), "blocked_reason": null if not intent.is_empty() else "current target required", "intent": intent if not intent.is_empty() else null}
	_add_option(option)


func _add_named_state_option(label: String, state: Dictionary) -> void:
	_add_option({"label": label, "enabled": bool(state.get("enabled", false)), "blocked_reason": state.get("blocked_reason"), "intent": state.get("intent")})


func _add_mode_actions(kinds: Array[String]) -> void:
	for value: Variant in _frame.get("action_options", []):
		var option: Dictionary = value as Dictionary
		var intent: Variant = option.get("intent")
		if intent is Dictionary and kinds.has(str(intent.get("kind", ""))):
			_add_option(option)


func _collect_and_add_options(value: Variant, capability_kinds: Array[String] = []) -> void:
	if value is Dictionary:
		var dictionary: Dictionary = value as Dictionary
		if not capability_kinds.is_empty() and dictionary.has("kind") and dictionary.has("capability_id") and not capability_kinds.has(str(dictionary["kind"])):
			return
		if dictionary.has("label") and dictionary.has("enabled") and dictionary.has("blocked_reason") and dictionary.has("intent"):
			_add_option(dictionary)
			return
		for child: Variant in dictionary.values():
			_collect_and_add_options(child, capability_kinds)
	elif value is Array:
		for child: Variant in value:
			_collect_and_add_options(child, capability_kinds)


func _add_option(option: Dictionary) -> void:
	var option_key: String = "%s|%s|%s|%s" % [option.get("label", "Action"), JSON.stringify(option.get("intent")), option.get("enabled", false), option.get("blocked_reason")]
	if _rendered_option_keys.has(option_key):
		return
	_rendered_option_keys[option_key] = true
	var button: Button = Button.new()
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	var server_enabled: bool = bool(option.get("enabled", false)) and option.get("intent") is Dictionary
	button.set_meta("server_enabled", server_enabled)
	button.text = str(option.get("label", "Action"))
	if not server_enabled:
		var reason: String = str(option.get("blocked_reason", "server unavailable"))
		button.text += " [Unavailable: %s]" % reason
		button.tooltip_text = "Unavailable: " + reason
		button.accessibility_description = button.tooltip_text
	button.disabled = not _online or _command_pending or not server_enabled
	button.pressed.connect(_emit_option.bind(option.duplicate(true), button))
	content.add_child(button)
	_action_buttons.append(button)


func _emit_option(option: Dictionary, origin: Control) -> void:
	if not _online or _command_pending or not bool(option.get("enabled", false)) or option.get("intent") is not Dictionary:
		return
	intent_requested.emit((option["intent"] as Dictionary).duplicate(true), str(option.get("label", "Action")), origin)


func _update_action_buttons() -> void:
	for button: Button in _action_buttons:
		button.disabled = not _online or _command_pending or not bool(button.get_meta("server_enabled", false))
	_update_drop_zones()


func _add_drop_zone(parent: Control, destination: String, label: String, item: Dictionary) -> Button:
	var button: Button = Button.new()
	button.text = "◇ " + label
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	button.custom_minimum_size = Vector2(112.0, 48.0)
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button.focus_mode = Control.FOCUS_ALL
	button.set_meta("drop_destination", destination)
	if not item.is_empty():
		button.text = "◆ %s\n%s ×%s" % [label, item.get("name", "Unknown item"), item.get("quantity", 1)]
	button.tooltip_text = "%s drop zone%s" % [label, "; occupied by " + str(item.get("name", "item")) if not item.is_empty() else ""]
	button.accessibility_description = button.tooltip_text
	button.pressed.connect(_request_drop_destination.bind(destination))
	button.gui_input.connect(_on_drop_zone_input.bind(destination))
	parent.add_child(button)
	_drop_zone_buttons.append(button)
	return button


func _on_drop_zone_input(event: InputEvent, destination: String) -> void:
	if _ground_drag_active and event.is_action_released("tme_world_primary"):
		get_viewport().set_input_as_handled()
		_request_drop_destination(destination)


func _request_drop_destination(destination: String) -> void:
	if not _ground_drag_active:
		if is_instance_valid(_drop_status_label):
			_drop_status_label.text = "Drag a projected loose item onto a destination."
		return
	drop_destination_requested.emit(destination)


func _update_drop_zones() -> void:
	for button: Button in _drop_zone_buttons:
		button.disabled = not _online or _command_pending
		button.modulate = Color(1.0, 0.88, 0.58, 1.0) if _ground_drag_active and not button.disabled else Color.WHITE
	if is_instance_valid(_drop_status_label):
		_drop_status_label.text = (
			"Drop %s on Sack or one exact paper-doll destination." % _dragged_item_name
			if _ground_drag_active
			else "Drag a projected loose item to place it; no slot picker or fallback."
		)


func _add_heading(value: String) -> void:
	var label: Label = _add_label(value)
	label.add_theme_font_size_override("font_size", 18)


func _add_label(value: String) -> Label:
	var label: Label = Label.new()
	label.text = value
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_child(label)
	return label


func _render_item_summary(heading: String, values: Array) -> void:
	_add_heading(heading)
	if values.is_empty():
		_add_label("None")
	for value: Variant in values:
		var item: Dictionary = value as Dictionary
		if item.has("item"):
			item = item.get("item", {})
		_add_label("%s ×%s (%s)" % [item.get("name", "Unknown item"), item.get("quantity", 1), item.get("item_instance_id", "?")])


func _render_offer_items(heading: String, values: Array) -> void:
	_add_heading(heading)
	if values.is_empty():
		_add_label("None")
	for value: Variant in values:
		var offer: Dictionary = value as Dictionary
		var item: Dictionary = offer.get("item", {})
		_add_label("%s ×%s · %s → %s" % [item.get("name", "Item"), item.get("quantity", 1), offer.get("sender_character_id", "?"), offer.get("recipient_character_id", "?")])


func _spell_action(spell_id: String) -> Dictionary:
	for value: Variant in _frame.get("spell_actions", []):
		var row: Dictionary = value as Dictionary
		if row.get("spell_id") == spell_id:
			return row
	return {}


func _projected_actor_ids() -> Array[String]:
	var result: Array[String] = []
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if not PresentationState.actor_is_present(actor):
			continue
		var actor_id: String = str(actor.get("actor_id", ""))
		if not actor_id.is_empty():
			result.append(actor_id)
	return result


func _actor_label(actor_id: String) -> String:
	for value: Variant in _frame.get("actors", []):
		var actor: Dictionary = value as Dictionary
		if actor.get("actor_id") == actor_id:
			return "%s (%s)" % [actor.get("name", "Actor"), actor_id]
	return actor_id


func _projected_item_targets() -> Dictionary:
	var result: Dictionary = {}
	for value: Variant in _frame.get("carried", {}).get("items", []):
		var row: Dictionary = value as Dictionary
		var item: Dictionary = row.get("item", {})
		var position: String = str(row.get("position", ""))
		var location: String = "sack" if position.begins_with("sack_item_") else "active_equipment"
		result[item.get("item_instance_id", "")] = {"item_instance_id": item.get("item_instance_id", ""), "name": item.get("name", "Item"), "location": location}
	for value: Variant in _frame.get("ground_items", []):
		var item: Dictionary = value as Dictionary
		result[item.get("item_instance_id", "")] = {"item_instance_id": item.get("item_instance_id", ""), "name": item.get("name", "Item"), "location": "ground_here"}
	result.erase("")
	return result


func _position_label(position: String) -> String:
	var words: PackedStringArray = position.split("_")
	for index: int in range(words.size()):
		words[index] = words[index].capitalize()
	return " ".join(words)


func _collect_control_text(node: Node, values: Array[String]) -> void:
	if node is Label:
		values.append((node as Label).text)
	elif node is Button:
		values.append((node as Button).text)
	for child: Node in node.get_children():
		_collect_control_text(child, values)
