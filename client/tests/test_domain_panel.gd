extends RefCounted

var _support: TestSupport


func test_occupied_inventory_cards_and_authoritative_character_render() -> void:
	var panel: DomainPanel = _add_panel()
	panel.present_frame(_domain_frame())
	panel.select_mode("Character")
	var text: String = panel.rendered_text()
	_support.expect("Fighter" in text and "HP 9/10" in text and "Sword · level 2" in text, "character/resources/skill facts render from the frame")
	_support.expect(not "foreign_secret" in text, "unprojected foreign data never appears")
	panel.select_mode("Inventory")
	text = panel.rendered_text()
	_support.expect("Paper doll · drop zones" in text and "Left Hand" in text and "Test Blade ×1" in text, "inventory presents occupied paper-doll cards")
	_support.expect("Right Hand: Empty" not in text and "sack_item_20" not in text, "inventory no longer enumerates empty positions as prose")
	_support.expect("Sack dock" in text and "Sack · 0 occupied" in text, "sack is a compact occupied-card dock")
	_support.expect("Left hand 3 · Right hand 0 · Sack 12" in text and "lightly loaded" in text, "positioned gold and burden render exactly")
	_support.expect_equal(panel.drop_zone_buttons().size(), 24, "paper doll plus one sack dock expose exact focusable destinations without twenty empty sack rows")
	var destinations: Dictionary = {}
	for button: Button in panel.drop_zone_buttons():
		_support.expect(button.focus_mode == Control.FOCUS_ALL and "Empty" not in button.text, "every drop zone is focusable without textual empty-slot prose")
		destinations[button.get_meta("drop_destination")] = true
	_support.expect_equal(destinations.size(), 24, "drop destinations are unique and include the one semantic sack dock")
	panel.free()


func test_magic_palette_uses_only_current_spell_action_rows_and_keeps_known_details() -> void:
	var panel: DomainPanel = _add_panel()
	var frame: Dictionary = _domain_frame()
	frame["character"]["known_spells"].append({"spell_id": "known_but_not_ready", "lane": "utility", "learned_at_level": 2})
	var second: Dictionary = frame["spell_actions"][0].duplicate(true)
	second["spell_id"] = "ward"
	second["spell_name"] = "Ward"
	frame["spell_actions"].append(second)
	panel.present_frame(frame)
	_support.expect_equal(panel.known_spell_ids(), ["spark", "ward"], "ready collection follows exact spell_actions order")
	panel.select_mode("Magic")
	var palette: SpellPalette = panel.content.get_node("MagicSpellPalette") as SpellPalette
	_support.expect_equal(palette.ready_control_count(), 2, "Magic renders exactly one control per projected spell action")
	var text: String = panel.rendered_text()
	_support.expect("Ready spells" in text and "Warmed spell" in text, "dynamic ready collection and warmed state are visible")
	_support.expect("known_but_not_ready · utility · learned level 2" in text, "known spellbook detail remains separate from ready controls")
	_support.expect(("Five" + " pages") not in text and "Empty" not in text, "no page-size prose or padding remains")
	_support.expect("Intensity / Power [Unavailable: no current rules contract]" in text, "intensity is explicitly unavailable")
	panel.free()


func test_magic_emitters_survive_until_domain_rebuild_flushes() -> void:
	var panel: DomainPanel = _add_panel()
	var frame: Dictionary = _domain_frame()
	panel.present_frame(frame)
	panel.select_mode("Magic")

	var palette: SpellPalette = panel.content.get_node("MagicSpellPalette") as SpellPalette
	var palette_button: Button = palette.buttons()[0]
	var content_count: int = panel.content.get_child_count()
	palette_button.pressed.emit()
	_support.expect(
		is_instance_valid(palette_button) and palette_button.get_parent() == palette.spell_grid,
		"DomainPanel does not free its emitting spell button synchronously",
	)
	_support.expect_equal(panel.content.get_child_count(), content_count, "spell activation leaves the complete Magic body intact until the deferred flush")
	_support.expect_equal(panel.get("_rebuild_queued"), true, "spell activation queues one DomainPanel rebuild")
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(not is_instance_valid(palette), "the old Magic palette is released by the real deferred frame flush")
	_support.expect_equal(panel.content.get_children().filter(func(child: Node) -> bool: return child is SpellPalette).size(), 1, "the deferred flush leaves exactly one parented Magic palette")

	var actor_picker: OptionButton = _option_with_first_item(panel, "Choose projected actor…")
	actor_picker.item_selected.emit(2)
	_support.expect(is_instance_valid(actor_picker) and actor_picker.get_parent() == panel.content, "actor dropdown survives its item_selected emission")
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(not is_instance_valid(actor_picker), "actor dropdown is replaced only after its emission returns")

	var authorization: OptionButton = _option_with_first_item(panel, "Authorization: Safe")
	authorization.item_selected.emit(1)
	_support.expect(is_instance_valid(authorization) and authorization.get_parent() == panel.content, "authorization dropdown survives its item_selected emission")
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(not is_instance_valid(authorization), "authorization dropdown is replaced by the deferred flush")

	frame = _domain_frame()
	frame["spell_actions"][0]["cast_class"] = "path_or_character"
	panel.present_frame(frame)
	var source_picker: OptionButton = _option_with_first_item(panel, "Projected actor")
	source_picker.item_selected.emit(1)
	_support.expect(is_instance_valid(source_picker) and source_picker.get_parent() == panel.content, "target-source dropdown survives its item_selected emission")
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(not is_instance_valid(source_picker), "target-source dropdown is replaced by the deferred flush")

	frame = _domain_frame()
	frame["spell_actions"][0]["target_kind"] = "item"
	frame["spell_actions"][0]["cast_class"] = "not_applicable"
	panel.present_frame(frame)
	var item_picker: OptionButton = _option_with_first_item(panel, "Choose projected item…")
	item_picker.item_selected.emit(1)
	_support.expect(is_instance_valid(item_picker) and item_picker.get_parent() == panel.content, "item dropdown survives its item_selected emission")
	await (Engine.get_main_loop() as SceneTree).process_frame
	_support.expect(not is_instance_valid(item_picker), "item dropdown is replaced by the deferred flush")
	panel.free()


func test_magic_actor_targets_exclude_dead_but_keep_ghost_players() -> void:
	var panel: DomainPanel = _add_panel()
	var frame: Dictionary = _domain_frame()
	frame["actors"].append({"actor_id": "dead-creature", "name": "Dead Creature", "kind": "monster", "character_id": null, "life_state": "dead"})
	frame["actors"].append({"actor_id": "ghost-player", "name": "Ghost Player", "kind": "player", "character_id": "char:ghost", "life_state": "ghost"})
	panel.present_frame(frame)
	var actor_ids: Array[String] = panel.call("_projected_actor_ids")
	_support.expect(not "dead-creature" in actor_ids, "Magic actor targets exclude exact dead rows")
	_support.expect("ghost-player" in actor_ids, "Magic actor targets retain projected ghost players")
	panel.set_actor_target("dead-creature")
	_support.expect(panel.build_selected_cast_intent().is_empty(), "a dead actor ID cannot survive finite target validation")
	panel.set_actor_target("ghost-player")
	_support.expect_equal(panel.build_selected_cast_intent().get("target"), {"kind": "actor", "actor_id": "ghost-player"}, "a projected ghost remains a finite actor target where the server owns spell legality")
	panel.select_mode("Social")
	_support.expect("Ghost Player (char:ghost)" in panel.rendered_text(), "the visible-player social surface keeps the projected ghost row")
	panel.free()


func test_selection_required_spell_targets_author_only_current_finite_inputs() -> void:
	var panel: DomainPanel = _add_panel()
	var frame: Dictionary = _domain_frame()
	panel.present_frame(frame)
	panel.select_mode("Magic")
	panel.set_actor_target("monster")
	_support.expect_equal(panel.build_selected_cast_intent(), {"kind": "cast_spell", "spell_id": "spark", "target": {"kind": "actor", "actor_id": "monster"}, "authorization": "safe"}, "actor cast uses the current projected actor ID")
	panel.set_authorization("confirmed_unsafe")
	_support.expect_equal(panel.build_selected_cast_intent()["authorization"], "confirmed_unsafe", "unsafe authorization is explicit finite player input")

	var cases: Array[Dictionary] = [
		{"target_kind": null, "cast_class": "path", "expected": {"kind": "path", "directions": ["east", "north"]}},
		{"target_kind": "coordinate", "cast_class": "not_applicable", "expected": {"kind": "coordinate", "position": {"x": 2, "y": -1}}},
		{"target_kind": "area", "cast_class": "not_applicable", "expected": {"kind": "area", "center": {"x": 2, "y": -1}}},
		{"target_kind": "direction", "cast_class": "not_applicable", "expected": {"kind": "direction", "direction": "east"}},
		{"target_kind": "door", "cast_class": "not_applicable", "expected": {"kind": "door", "direction": "east"}},
	]
	for value: Dictionary in cases:
		frame = _domain_frame()
		frame["spell_actions"][0]["target_kind"] = value["target_kind"]
		frame["spell_actions"][0]["cast_class"] = value["cast_class"]
		panel.present_frame(frame)
		panel.set_world_target(Vector2i(2, -1), ["east", "north"])
		_support.expect_equal(panel.build_selected_cast_intent()["target"], value["expected"], "finite selected target shape is exact")

	frame = _domain_frame()
	frame["spell_actions"][0]["target_kind"] = "item"
	frame["spell_actions"][0]["cast_class"] = "not_applicable"
	panel.present_frame(frame)
	panel.set_item_target("item:left")
	_support.expect_equal(panel.build_selected_cast_intent()["target"], {"kind": "item", "item_instance_id": "item:left", "location": "active_equipment"}, "item cast uses current projected instance and finite location")
	panel.free()


func test_complete_frame_clears_stale_target_and_rebuilds_without_local_ledger() -> void:
	var panel: DomainPanel = _add_panel()
	var frame: Dictionary = _domain_frame()
	panel.present_frame(frame)
	panel.select_mode("Magic")
	panel.set_actor_target("monster")
	panel.set_world_target(Vector2i(1, 0), ["east"])
	_support.expect(not panel.build_selected_cast_intent().is_empty(), "current target can author one attempt")
	var replacement: Dictionary = _domain_frame()
	replacement["actors"] = [replacement["actors"][0]]
	replacement["character"]["resources"]["hp"] = 4
	panel.present_frame(replacement)
	_support.expect(panel.build_selected_cast_intent().is_empty(), "complete frame clears actor and geometry selections")
	panel.select_mode("Character")
	_support.expect("HP 4/10" in panel.rendered_text(), "replacement frame rather than a patched local ledger owns the display")
	panel.free()


func test_projected_actions_submit_exactly_and_respect_disabled_pending_and_offline_gates() -> void:
	var panel: DomainPanel = _add_panel()
	var emitted: Array[Dictionary] = []
	panel.intent_requested.connect(func(intent: Dictionary, _label: String, _origin: Control) -> void: emitted.append(intent.duplicate(true)))
	panel.present_frame(_domain_frame())
	panel.select_mode("Services & Quests")
	panel.set_connection_state(true)
	var buy: Button = _button_containing(panel, "Buy Test Item")
	_support.expect(buy != null and not buy.disabled, "enabled projected merchant action is reachable")
	buy.pressed.emit()
	_support.expect_equal(emitted, [{"kind": "buy_from_merchant", "service_id": "service", "capability_id": "merchant", "item_instance_ids": ["merchant:item"]}], "merchant action emits exact projected intent")
	var sold_out: Button = _button_containing(panel, "Buy All")
	_support.expect(sold_out != null and sold_out.disabled and "sold_out" in sold_out.text, "disabled action retains exact reason and emits nothing")
	panel.set_command_pending(true)
	_support.expect(buy.disabled, "pending command disables every exact action")
	panel.set_command_pending(false)
	panel.set_connection_state(false)
	_support.expect(buy.disabled, "offline state disables every exact action")
	_support.expect_equal(emitted.size(), 1, "disabled states never emit")
	panel.free()


func test_services_quests_social_and_offer_surfaces_use_only_projected_facts() -> void:
	var panel: DomainPanel = _add_panel()
	panel.present_frame(_domain_frame())
	panel.select_mode("Services & Quests")
	var text: String = panel.rendered_text()
	_support.expect("Town Services (service)" in text and "Merchant" in text and "Test Item ×1 · 5 gold" in text, "typed service and merchant facts render")
	_support.expect("Guide (guide)" in text and "Welcome · begin: Speak to the guide" in text, "NPC and quest rows render")
	panel.select_mode("Social")
	text = panel.rendered_text()
	_support.expect("Group 1 · leader char:self" in text and "Following char:other" in text and "Pages enabled" in text, "group/follow/page facts render")
	_support.expect("Other (char:other)" in text and "Offered Token ×1" in text, "visible character identity and rich offers render")
	_support.expect(not "message body" in text and not "chat" in text.to_lower(), "FA chat presentation is absent")
	panel.free()


func _add_panel() -> DomainPanel:
	var panel: DomainPanel = (load("res://presentation/DomainPanel.tscn") as PackedScene).instantiate() as DomainPanel
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	tree.root.size = Vector2i(1280, 720)
	tree.root.add_child(panel)
	panel.visible = true
	return panel


func _button_containing(panel: DomainPanel, fragment: String) -> Button:
	for button: Button in panel._action_buttons:
		if fragment in button.text:
			return button
	return null


func _option_with_first_item(panel: DomainPanel, first_item: String) -> OptionButton:
	for child: Node in panel.content.get_children():
		if child is not OptionButton:
			continue
		var option: OptionButton = child as OptionButton
		if option.item_count > 0 and option.get_item_text(0) == first_item:
			return option
	return null


func _domain_frame() -> Dictionary:
	return {
		"logical_time": "10", "ready_at": "10", "can_act": true,
		"observer_actor_id": "player",
		"observation_center": {"realm": "test", "level": "surface", "position": {"x": 0, "y": 0}},
		"observation_radius": 7,
		"tiles": [],
			"actors": [
				{"actor_id": "player", "name": "Self", "kind": "player", "character_id": "char:self", "life_state": "alive"},
				{"actor_id": "monster", "name": "Monster", "kind": "monster", "character_id": null, "life_state": "alive"},
				{"actor_id": "other", "name": "Other", "kind": "player", "character_id": "char:other", "life_state": "alive"},
		],
		"corpses": [], "ground_items": [{"item_instance_id": "ground:item", "name": "Ground Token", "quantity": 1}], "gold_piles": [],
		"action_options": [
			{"id": "hide", "label": "Hide", "enabled": true, "blocked_reason": null, "intent": {"kind": "hide"}},
			{"id": "invite", "label": "Invite Other", "enabled": true, "blocked_reason": null, "intent": {"kind": "invite", "target_character_id": "char:other"}},
		],
		"social": {"character_id": "char:self", "group": {"group_id": "1", "leader_character_id": "char:self", "members": [{"character_id": "char:self", "connected": true}, {"character_id": "char:other", "connected": true}]}, "incoming_invitations": [], "outgoing_invitations": [], "following_character_id": "char:other", "pages_enabled": true, "blocked_character_ids": []},
		"incoming_item_offers": [{"item": {"item_instance_id": "offer:item", "name": "Offered Token", "quantity": 1}, "sender_character_id": "char:other", "recipient_character_id": "char:self", "actions": [{"id": "refuse", "label": "Refuse Offer", "enabled": true, "blocked_reason": null, "intent": {"kind": "refuse_item_offer", "item_instance_id": "offer:item"}}]}],
		"outgoing_item_offers": [],
		"character": {
			"identity": {"base_class_id": "fighter", "current_class_id": "fighter", "display_class": "Fighter", "nationality_id": "northreach", "sex_or_gender_display": null},
			"alignment": "neutral", "karma_points": 2,
			"attributes": {"strength": 10, "dexterity": 9, "constitution": 8, "intelligence": 7, "wisdom": 6, "charisma": 5},
			"resources": {"hp": 9, "max_hp": 10, "peak_hp": 10, "mp": 5, "max_mp": 6, "stamina": 7, "max_stamina": 8},
			"progression": {"level": 2, "experience": "12", "pending_target_level": null},
			"physical_attribute_adds": {"strength_adds": 1, "dexterity_adds": 0}, "promotion_history": [],
			"known_spells": [{"spell_id": "spark", "lane": "attack", "learned_at_level": 1}],
			"skill_ledger": [{"track_id": "sword", "level": 2, "critique_rank": 1, "practice_points": "3", "learning_rate": "4", "track_display": "Sword", "level_title": "Apprentice"}],
		},
		"carried": {"items": [{"position": "left_hand", "item": {"item_instance_id": "item:left", "name": "Test Blade", "quantity": 1}, "valid_placements": ["hand"]}], "gold": {"left_hand": "3", "right_hand": "0", "sack": "12"}},
		"burden": {"item_burden": "2", "coin_burden": "1", "total_burden": "3", "lightly_loaded_limit": "10", "moderately_loaded_limit": "20", "heavily_loaded_limit": "30", "tier": "lightly_loaded"},
		"warmed_spell": {"spell_id": "spark", "warmed_at": "8", "ready_at": "9", "status": "ready"},
		"spell_actions": [{"spell_id": "spark", "spell_name": "Spark", "casting_method": "direct", "cast_class": "character", "target_kind": "actor", "mp_cost": 1, "stamina_cost": null, "hostile_act": true, "town_law_violation": false, "warm": {"enabled": true, "blocked_reason": null, "requires_target_selection": false, "intent": {"kind": "warm_spell", "spell_id": "spark"}}, "cast": {"enabled": true, "blocked_reason": null, "requires_target_selection": true, "intent": null}}],
		"services_here": [
			{
				"service_id": "service",
				"name": "Town Services",
				"capabilities": [
					{
						"kind": "merchant",
						"capability_id": "merchant",
						"listings": [
							{
								"item": {"item_instance_id": "merchant:item", "name": "Test Item", "quantity": 1},
								"origin": "authored_stock",
								"price_gold": "5",
								"purchase": {"id": "buy", "label": "Buy Test Item", "enabled": true, "blocked_reason": null, "intent": {"kind": "buy_from_merchant", "service_id": "service", "capability_id": "merchant", "item_instance_ids": ["merchant:item"]}},
							},
						],
						"buy_all": {"id": "buy_all", "label": "Buy All", "enabled": false, "blocked_reason": "sold_out", "intent": {"kind": "buy_from_merchant", "service_id": "service", "capability_id": "merchant", "item_instance_ids": []}},
						"sales": [],
					},
				],
			},
		],
		"npcs_here": [{"actor_id": "guide", "name": "Guide", "following_character_id": null, "interactions": [{"interaction_id": "speak", "label": "Speak", "actions": [{"id": "speak", "label": "Speak", "enabled": true, "blocked_reason": null, "intent": {"kind": "interact_with_npc", "npc_actor_id": "guide", "interaction_id": "speak", "item_instance_id": null}}]}]}],
		"quest_log": [{"quest_id": "welcome", "quest_title": "Welcome", "stage_id": "begin", "stage_label": "Speak to the guide", "terminal": false}],
	}
