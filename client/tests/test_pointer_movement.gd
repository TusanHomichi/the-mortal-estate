extends RefCounted

## Drafting a move by pointing, and what authority does to the draft.
##
## Every test here is about one question: when the player points at a square,
## what is proposed, when is it committed, and when is it thrown away. The
## authoritative preview is the only thing allowed to arm a commit, and a stale
## one may never do it.

const ShellSupport: Script = preload("res://tests/shell_test_support.gd")

var _support: TestSupport


func test_movement_batch_emits_one_to_three_steps_only() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var emitted: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: emitted.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	_support.expect(screen.queue_movement_action("tme_move_north", 1000), "first semantic direction queues")
	_support.expect(screen.queue_movement_action("tme_move_east", 1050), "second semantic direction queues")
	_support.expect(not screen.process_movement_batch(1099), "batch waits for the exact 100ms window")
	_support.expect(screen.process_movement_batch(1100), "100ms flush emits")
	_support.expect_equal(emitted[0], {"kind": "move_path", "path": ["north", "east"]}, "one batched move contains exact semantic directions")
	screen.queue_movement_action("tme_move_south", 2000)
	_support.expect(screen.flush_movement_batch(), "accept-style early flush emits one step")
	_support.expect_equal(emitted[1]["path"], ["south"], "early flush emits one step")
	screen.queue_movement_action("tme_move_northwest", 3000)
	screen.queue_movement_action("tme_move_west", 3010)
	screen.queue_movement_action("tme_move_southwest", 3020)
	_support.expect_equal(emitted[2]["path"].size(), 3, "third direction flushes at the maximum")
	screen.queue_movement_action("tme_move_north", 4000)
	screen.cancel_movement_batch()
	_support.expect_equal(screen.movement_batch(), [], "cancel clears without submitting")
	_support.expect_equal(emitted.size(), 3, "cancel emits no command")
	screen.free()


func test_pointer_first_activation_previews_and_same_target_repeat_commits_exact_request() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	var intents: Array[Dictionary] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	_support.expect(screen.activate_square(Vector2i(2, 1), 1000), "first semantic activation creates a draft immediately")
	_support.expect_equal(previews, [["southeast", "east"]], "draft is the exact direct geometric path without obstacle routing")
	_support.expect(not screen.has_current_preview() and intents.is_empty(), "single click never commits before server preview")
	_support.expect(screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"])), "current Path 8 preview is presented")
	_support.expect(screen.has_current_preview(), "current preview enables deliberate commit")
	_support.expect(screen.activate_square(Vector2i(2, 1), 1420), "same-target activation at the inclusive 420ms boundary commits")
	_support.expect_equal(intents, [{"kind": "move_path", "path": ["southeast", "east"]}], "commit submits the requested path rather than an inferred accepted prefix")
	_support.expect(not screen.has_current_preview() and screen.draft_path().is_empty(), "commit clears discardable draft state")
	screen.free()


func test_delayed_preview_and_intervening_frame_preserve_two_tap_movement_commit() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	var intents: Array[Dictionary] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)

	_support.expect(screen.activate_square(Vector2i(2, 1), 1000), "first tap authors one pending movement preview")
	screen.present_frame(ShellSupport.world_frame([]), 2, true, true)
	_support.expect_equal(previews, [["southeast", "east"]], "a generation-only frame does not replace the authority-current preview request")
	_support.expect_equal(screen.draft_path(), ["southeast", "east"], "the stable target draft survives the intervening frame")
	_support.expect(screen.activate_square(Vector2i(2, 1), 1150), "second tap is accepted on the player's input timeline")
	_support.expect(screen.pointer_commit_armed() and intents.is_empty(), "second tap arms commit without pretending an absent preview is legal")
	_support.expect(not screen.process_pointer_preview_timeout(1400), "a preview delayed 250 ms after the second tap remains inside the response wait")
	_support.expect(screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"])), "the delayed authoritative preview satisfies the armed commit")
	_support.expect_equal(intents, [{"kind": "move_path", "path": ["southeast", "east"]}], "the exact requested path commits once after delayed preview")
	_support.expect(not screen.pointer_commit_armed() and screen.draft_path().is_empty(), "commit clears the complete latency-wait state")
	screen.free()


func test_visible_movement_draft_confirms_on_a_later_tap_without_a_double_tap_deadline() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	_support.expect(screen.activate_square(Vector2i(2, 1), 1000), "the first tap opens a visible movement draft")
	_support.expect(screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"])), "the authoritative preview lands under the draft")
	_support.expect(screen.activate_square(Vector2i(2, 1), 4200), "a tap 3.2 s later still confirms the square the marker is sitting on")
	_support.expect_equal(intents, [{"kind": "move_path", "path": ["southeast", "east"]}], "a slow confirmation commits the exact requested path once")
	_support.expect(screen.draft_path().is_empty(), "confirmation clears the draft it consumed")
	screen.free()


func test_authority_pulse_refreshes_the_preview_without_dropping_the_marker() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.activate_square(Vector2i(2, 1), 1000)
	screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"]))
	_support.expect_equal(ShellSupport.draft_presentation(screen), "preview", "the returned preview is what the player is looking at")

	screen.present_frame(ShellSupport.world_frame([]), 2, true, false)
	_support.expect(not screen.has_current_preview(), "an advanced world revision retires the returned preview as commit authority")
	_support.expect_equal(previews.size(), 2, "the client reissues the request against current authority exactly once")
	_support.expect_equal(ShellSupport.draft_presentation(screen), "preview", "the presented preview survives the pulse instead of flicking back to pending")
	screen.free()


func test_retired_preview_never_commits_until_the_authority_current_result_lands() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	var intents: Array[Dictionary] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.activate_square(Vector2i(2, 1), 1000)
	screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"]), 1100)

	screen.present_frame(ShellSupport.world_frame([]), 2, true, false)
	_support.expect(not screen.has_current_preview(), "the pulse retires the rendered preview as commit authority")
	_support.expect_equal(ShellSupport.draft_presentation(screen), "preview", "the retired preview remains presented while its replacement is in flight")
	_support.expect(screen.activate_square(Vector2i(2, 1), 1200), "the drafted square still accepts the player's confirmation")
	_support.expect(screen.pointer_commit_armed(), "confirmation waits for current authority instead of trusting the rendered retired preview")
	_support.expect(intents.is_empty(), "a retired preview cannot commit movement")
	_support.expect(screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"]), 1300), "the authority-current replacement satisfies the armed draft")
	_support.expect_equal(intents, [{"kind": "move_path", "path": ["southeast", "east"]}], "movement commits exactly once when the fresh result lands")
	_support.expect_equal(previews, [["southeast", "east"], ["southeast", "east"]], "the authority pulse requests exactly one replacement preview")
	screen.free()


func test_reconcile_never_arms_commit_from_a_stale_or_path_changed_preview() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	var intents: Array[Dictionary] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.activate_square(Vector2i(2, 0), 1000)
	screen.apply_path_preview_result(ShellSupport.preview_result(["east", "east"]), 1100)
	screen.present_frame(ShellSupport.world_frame([]), 2, true, false)
	screen.activate_square(Vector2i(2, 0), 1200)
	_support.expect(screen.pointer_commit_armed() and intents.is_empty(), "the retired preview leaves the confirmed draft armed but uncommitted")

	var shifted_frame: Dictionary = ShellSupport.world_frame([])
	shifted_frame["observation_center"]["position"] = {"x": 1, "y": 0}
	shifted_frame["actors"][0]["position"]["position"] = {"x": 1, "y": 0}
	screen.present_frame(shifted_frame, 3, true, true)
	_support.expect_equal(screen.draft_path(), ["east"], "reconcile replaces the path from the new authoritative center")
	_support.expect(screen.pointer_commit_armed() and intents.is_empty(), "the armed reconcile branch does not commit the stale path")
	_support.expect_equal(previews, [["east", "east"], ["east", "east"], ["east"]], "the changed path is reissued against current authority")
	_support.expect(not screen.apply_path_preview_result(ShellSupport.preview_result(["east", "east"]), 1300), "the stale old-path result is refused")
	_support.expect(intents.is_empty(), "the refused stale result emits no movement")
	_support.expect(screen.apply_path_preview_result(ShellSupport.preview_result(["east"]), 1400), "the fresh changed-path result satisfies the armed draft")
	_support.expect_equal(intents, [{"kind": "move_path", "path": ["east"]}], "only the authority-current changed path commits")
	screen.free()


func test_failed_pointer_redirect_clears_the_armed_draft_and_old_result() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var previews: Array[Array] = []
	var intents: Array[Dictionary] = []
	screen.path_preview_requested.connect(func(path: Array[String]) -> void: previews.append(path.duplicate()))
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.activate_square(Vector2i(1, 0), 1000)
	screen.activate_square(Vector2i(1, 0), 1200)
	_support.expect(screen.pointer_commit_armed(), "the first target is armed while its preview is pending")

	screen.activate_square(Vector2i(4, 0), 1300)
	_support.expect(screen.draft_path().is_empty() and not screen.pointer_commit_armed(), "a deliberate beyond-range redirect kills the abandoned draft")
	_support.expect("Choose another visible square within three steps" in screen.action_indicator.text, "the failed redirect retains its actionable range feedback")
	_support.expect_equal(previews, [["east"]], "an invalid redirect emits no replacement preview request")
	_support.expect(not screen.apply_path_preview_result(ShellSupport.preview_result(["east"]), 1400), "the abandoned target's late preview is no longer accepted")
	_support.expect(intents.is_empty(), "the abandoned armed target never commits")
	screen.free()


func test_untouched_movement_draft_expires_instead_of_waiting_forever() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.activate_square(Vector2i(2, 1), 1000)
	screen.apply_path_preview_result(ShellSupport.preview_result(["southeast", "east"]))
	_support.expect(not screen.process_pointer_draft_idle_timeout(10999), "a draft stays live while the player may still be deciding")
	_support.expect(screen.process_pointer_draft_idle_timeout(11000), "an untouched draft expires at the idle bound")
	_support.expect(intents.is_empty() and screen.draft_path().is_empty(), "expiry never moves the player")
	_support.expect("expired" in screen.action_indicator.text and "no movement was sent" in screen.action_indicator.text, "expiry says what happened")
	screen.free()


func test_reissued_preview_times_out_from_total_unanswered_age() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	var created_msec: int = 123456789
	screen.begin_pointer_draft(Vector2i(1, 0), created_msec)
	screen.present_frame(ShellSupport.world_frame([]), 2, true, false)
	_support.expect(not screen.process_pointer_preview_timeout(created_msec + WorldShellScreen.PATH_PREVIEW_TIMEOUT_MSEC - 1), "a reissue does not shorten the original response window")
	_support.expect(screen.process_pointer_preview_timeout(created_msec + WorldShellScreen.PATH_PREVIEW_TIMEOUT_MSEC), "a reissuing draft still closes at five seconds total without an answer")
	_support.expect("timed out" in screen.action_indicator.text and "no movement was sent" in screen.action_indicator.text, "the total response timeout uses the explicit fail-closed copy")
	_support.expect(intents.is_empty() and screen.draft_path().is_empty(), "the total response timeout sends no movement and clears the draft")
	screen.free()


func test_armed_pointer_draft_never_idle_expires() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.activate_square(Vector2i(1, 0), 1000)
	screen.activate_square(Vector2i(1, 0), 1200)
	_support.expect(screen.pointer_commit_armed(), "the confirmed draft is armed while awaiting authority")
	_support.expect(not screen.process_pointer_draft_idle_timeout(11200), "live confirmed intent is never treated as an idle forgotten marker")
	_support.expect(screen.pointer_commit_armed() and screen.draft_path() == ["east"], "the armed draft remains governed by its response timeout")
	screen.free()


func test_missing_preview_times_out_with_feedback_and_never_moves() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	var intents: Array[Dictionary] = []
	screen.intent_requested.connect(func(intent: Dictionary) -> void: intents.append(intent.duplicate(true)))
	screen.set_connection_state("ONLINE", true)
	screen.present_frame(ShellSupport.world_frame([]), 1)
	screen.begin_pointer_draft(Vector2i(1, 0), 1000)
	_support.expect(not screen.process_pointer_preview_timeout(5999), "preview wait remains open immediately before the bounded timeout")
	_support.expect(screen.process_pointer_preview_timeout(6000), "missing preview closes at the bounded response timeout")
	_support.expect("timed out" in screen.action_indicator.text and "no movement was sent" in screen.action_indicator.text, "timeout gives explicit fail-closed player feedback")
	_support.expect(intents.is_empty() and screen.draft_path().is_empty(), "timeout never commits or retains a silent movement draft")
	screen.free()


func test_pointer_draft_routes_around_a_blocked_cell() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	screen.set_connection_state("ONLINE", true)
	var blocked: Array = [Vector2i(1, 1)]
	screen.present_frame(ShellSupport.world_frame_with_blocked(blocked), 1)
	_support.expect(screen.begin_pointer_draft(Vector2i(2, 1), 1000), "a reachable target drafts even when the direct line is blocked")
	var route: Array[String] = screen.draft_path()
	_support.expect(ShellSupport.route_end_if_legal(Vector2i.ZERO, route, blocked) == Vector2i(2, 1), "the proposal detours to the target without touching the blocked cell: %s" % [route])
	_support.expect_equal(ShellSupport.draft_presentation(screen), "pending", "the routed draft is presented as awaiting authority")
	screen.free()


func test_pointer_draft_never_proposes_a_corner_squeeze() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	screen.set_connection_state("ONLINE", true)
	var blocked: Array = [Vector2i(1, 0)]
	screen.present_frame(ShellSupport.world_frame_with_blocked(blocked), 1)
	_support.expect(screen.begin_pointer_draft(Vector2i(1, 1), 1000), "the diagonal neighbor still drafts when its direct squeeze is illegal")
	var route: Array[String] = screen.draft_path()
	_support.expect(route.size() == 2, "the squeeze is replaced by a two-step detour: %s" % [route])
	_support.expect(ShellSupport.route_end_if_legal(Vector2i.ZERO, route, blocked) == Vector2i(1, 1), "the detour is legal and reaches the target: %s" % [route])
	screen.free()


func test_unreachable_target_is_not_a_draftable_reach_square() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	screen.set_connection_state("ONLINE", true)
	var blocked: Array = [
		Vector2i(1, -1), Vector2i(1, 0), Vector2i(1, 1),
		Vector2i(2, -1), Vector2i(2, 1),
		Vector2i(3, -1), Vector2i(3, 0), Vector2i(3, 1),
	]
	screen.present_frame(ShellSupport.world_frame_with_blocked(blocked), 1)
	_support.expect(not screen.begin_pointer_draft(Vector2i(2, 0), 1000), "an enclosed target is absent from the reach-grid and rejects locally")
	_support.expect(screen.draft_path().is_empty(), "rejected non-member target leaves no draft path")
	screen.free()


## The reach set is authored by [ClientReachability] from the frame, not by
## whatever draws it. This proves membership and draftability agree at that
## owner, which is what survived the presentation scaffold's retirement.


## The reach set is authored by [ClientReachability] from the frame, not by
## whatever draws it. This proves membership and draftability agree at that
## owner, which is what survived the presentation scaffold's retirement.
func test_every_reachable_square_drafts_and_nonmember_adjacent_rejects() -> void:
	var screen: WorldShellScreen = ShellSupport.add_screen("res://scenes/WorldShellScreen.tscn") as WorldShellScreen
	screen.set_connection_state("ONLINE", true)
	var frame: Dictionary = ShellSupport.world_frame([])
	screen.present_frame(frame, 1)
	var reachable: Array[Vector2i] = []
	reachable.assign(ClientReachability.frame_facts(frame)["reach_cells"])
	_support.expect_equal(reachable.size(), 48, "an open radius-three reach set has exactly forty-eight positive-length destinations")
	for coordinate: Vector2i in reachable:
		screen.clear_pointer_draft(false)
		_support.expect(screen.begin_pointer_draft(coordinate, 1000), "every reachable square begins a draft: %s" % [coordinate])
		_support.expect(not screen.draft_path().is_empty() and screen.draft_path().size() <= InputActions.MAX_MOVE_PATH_STEPS, "every reachable square produces a bounded route: %s" % [coordinate])

	var blocked: Dictionary = ShellSupport.world_frame_with_blocked([Vector2i(1, 0)])
	screen.present_frame(blocked, 2, false, false)
	_support.expect(Vector2i(1, 0) not in ClientReachability.frame_facts(blocked)["reach_cells"], "a blocked adjacent square is not in the reach set")
	_support.expect(not screen.begin_pointer_draft(Vector2i(1, 0), 2000), "a blocked adjacent non-member rejects")
	_support.expect(screen.begin_pointer_draft(Vector2i(1, 1), 2001), "a reachable diagonal target still drafts by rerouting around the blocked side")
	_support.expect_equal(screen.draft_path().size(), 2, "the diagonal corner rule produces the shared two-step reroute")
	screen.free()
