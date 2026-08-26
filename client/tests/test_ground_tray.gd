extends RefCounted

var _support: TestSupport


func test_complete_pinned_tray_fan_overflow_and_unsearched_truth() -> void:
	var tray: GroundTray = _add_tray()
	var frame: Dictionary = _frame()
	tray.present_frame(frame, 4)
	tray.pin_target(_target(Vector2i.ZERO))
	_support.expect_equal(tray.projected_items().size(), 6, "tray retains every projected loose item")
	_support.expect_equal(tray.fan_items().size(), 4, "world fan is deterministically bounded at four")
	_support.expect_equal(tray.overflow_count(), 2, "overflow token count is exact")
	_support.expect("Contents not searched" in _row_text(tray), "unsearched corpse never claims empty contents")
	_support.expect("12 gold" in _row_text(tray), "projected gold remains distinct and visible")
	tray.free()


func test_exact_search_lowest_sack_and_specific_destination_fail_closed() -> void:
	var tray: GroundTray = _add_tray()
	var frame: Dictionary = _frame()
	tray.present_frame(frame, 7)
	tray.pin_target(_target(Vector2i.ZERO))
	_support.expect_equal(tray.exact_search_option("corpse:1")["intent"], {"kind": "search_corpse", "corpse_id": "corpse:1"}, "corpse search copies the one exact option")
	_support.expect_equal(tray.exact_item_destination_option("item:0")["intent"]["destination"]["position"], "sack_item_1", "double activation chooses the lowest exact enabled sack row")
	_support.expect_equal(tray.exact_item_destination_option("item:0", "right_hand")["intent"]["destination"]["position"], "right_hand", "specific drop uses only its exact destination")
	_support.expect(tray.exact_item_destination_option("item:0", "left_hand").is_empty(), "missing specific destination does not redirect to sack")
	frame["action_options_truncated"] = true
	tray.present_frame(frame, 8)
	_support.expect(tray.exact_item_destination_option("item:0").is_empty(), "truncation disables direct transfer")
	_support.expect(GroundTray.INCOMPLETE_MESSAGE in tray.status_label.text, "truncation explains fail-closed state exactly")
	tray.free()


func test_distant_ground_is_view_only() -> void:
	var tray: GroundTray = _add_tray()
	tray.present_frame(_frame(), 3)
	tray.pin_target(_target(Vector2i(1, 0)))
	_support.expect(tray.is_view_only(), "distant observed tile is view-only")
	_support.expect(tray.exact_search_option("corpse:1").is_empty(), "remote corpse search is never constructed")
	_support.expect("Stand on this tile to take or search" in tray.status_label.text, "remote state states the co-location requirement")
	tray.free()


func _add_tray() -> GroundTray:
	var tray: GroundTray = (load("res://presentation/GroundTray.tscn") as PackedScene).instantiate() as GroundTray
	(Engine.get_main_loop() as SceneTree).root.add_child(tray)
	return tray


func _target(coordinate: Vector2i) -> Dictionary:
	return {"identity": "tile:%d:%d" % [coordinate.x, coordinate.y], "kind": "tile", "source_identity": "tile:%d:%d" % [coordinate.x, coordinate.y], "coordinate": coordinate, "generation": 4}


func _row_text(tray: GroundTray) -> String:
	var values: Array[String] = []
	for button: Button in tray.get("_buttons"):
		values.append(button.text)
	return "\n".join(values)


func _frame() -> Dictionary:
	var items: Array[Dictionary] = []
	for index: int in range(6):
		items.append({"item_instance_id": "item:%d" % index, "name": "Item %d" % index, "quantity": 1, "location": _position(0, 0), "loot_claim": null})
	return {
		"observer_actor_id": "player",
		"observation_center": _position(0, 0),
		"actors": [{
			"actor_id": "player",
			"position": _position(0, 0),
		}],
		"corpses": [{"corpse_id": "corpse:1", "origin_name": "Kobold", "searched": false, "location": _position(0, 0)}],
		"ground_items": items,
		"gold_piles": [{"gold_pile_id": "gold:1", "amount": "12", "location": _position(0, 0)}],
		"action_options_truncated": false,
		"ground_items_truncated": false,
		"action_options": [
			{"id": "search", "label": "Search corpse", "enabled": true, "blocked_reason": null, "intent": {"kind": "search_corpse", "corpse_id": "corpse:1"}},
			{"id": "sack2", "label": "Stow", "enabled": true, "blocked_reason": null, "intent": {"kind": "move_item", "item_instance_id": "item:0", "destination": {"kind": "carried", "position": "sack_item_2"}}},
			{"id": "hand", "label": "Equip", "enabled": true, "blocked_reason": null, "intent": {"kind": "move_item", "item_instance_id": "item:0", "destination": {"kind": "carried", "position": "right_hand"}}},
			{"id": "sack1", "label": "Stow", "enabled": true, "blocked_reason": null, "intent": {"kind": "move_item", "item_instance_id": "item:0", "destination": {"kind": "carried", "position": "sack_item_1"}}},
		],
	}


func _position(x: int, y: int) -> Dictionary:
	return {"realm": "synthetic", "level": "surface", "position": {"x": x, "y": y}}
