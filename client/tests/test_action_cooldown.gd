extends RefCounted
var _support: TestSupport

func test_an_offset_action_gets_its_full_duration_immediately() -> void:
	var clock: ActionCooldown = ActionCooldown.new()
	clock.note_frame(_frame("4127", "7127", false), 100)
	_support.expect_equal(clock.duration_msec(), 3000, "the first frame supplies a complete individual interval")
	_support.expect_equal(clock.cooldown_fill(1600), 0.5, "half the cooldown has elapsed")
	_support.expect_equal(clock.cooldown_fill(9000), 1.0, "presentation stops at the deadline")
	_support.expect(not clock.is_ready(), "elapsed time cannot grant readiness")
	clock.note_frame(_frame("7127", "7127", true), 9000)
	_support.expect(clock.is_ready(), "the server grants readiness")
	_support.expect_equal(clock.cooldown_deadline_msec(), -1, "completed actions cannot constrain a later presentation cue")

func test_other_frames_do_not_restart_the_same_cooldown() -> void:
	var clock: ActionCooldown = ActionCooldown.new()
	clock.note_frame(_frame("4127", "7127", false), 100)
	clock.note_frame(_frame("5627", "7127", false), 1600)
	_support.expect_equal(clock.duration_msec(), 3000, "other updates preserve the original duration")
	_support.expect_equal(clock.cooldown_fill(1600), 0.5, "other updates preserve progress")
	clock.note_frame(_frame("5627", "7127", false), 1800)
	_support.expect_equal(clock.cooldown_deadline_msec(), 3100, "duplicate frame cannot extend the interval")

func test_two_characters_have_independent_offsets_and_costs() -> void:
	var first: ActionCooldown = ActionCooldown.new()
	var second: ActionCooldown = ActionCooldown.new()
	first.note_frame(_frame("4127", "7127", false), 100)
	second.note_frame(_frame("5300", "11300", false), 1273)
	_support.expect_equal(first.cooldown_deadline_msec(), 3100, "first individual deadline")
	_support.expect_equal(second.cooldown_deadline_msec(), 7273, "second action has its own longer cost")

func test_invalid_or_backwards_frames_remove_authority() -> void:
	for invalid: Dictionary in [{}, _frame("01", "3001", false), _frame("-1", "3000", false)]:
		var clock: ActionCooldown = ActionCooldown.new()
		clock.note_frame(_frame("4127", "7127", false), 100)
		clock.note_frame(invalid, 200)
		_support.expect(not clock.has_authority(), "invalid timing clears presentation")
	var backwards: ActionCooldown = ActionCooldown.new()
	backwards.note_frame(_frame("4127", "7127", false), 100)
	backwards.note_frame(_frame("4000", "7127", false), 200)
	_support.expect(not backwards.has_authority(), "backwards time is refused")

func test_wide_deadlines_keep_precise_small_differences() -> void:
	var clock: ActionCooldown = ActionCooldown.new()
	clock.note_frame(_frame("9007199254740993", "9007199254743993", false), 100)
	_support.expect_equal(clock.duration_msec(), 3000, "wire timing never passes through a float")

func test_meter_stops_when_ready_and_labels_cooldown_without_a_shared_beat() -> void:
	var clock: ActionCooldown = ActionCooldown.new()
	var meter: CooldownMeter = CooldownMeter.new()
	clock.note_frame(_frame("4127", "7127", false), 100)
	meter.present_cooldown(clock.state(1600))
	_support.expect_equal(meter.segments().size(), 1, "one bar presents one action")
	_support.expect_equal(meter.meter_text(), "◇ Action cooldown", "the label describes individual readiness")
	clock.note_frame(_frame("7127", "7127", true), 3100)
	meter.present_cooldown(clock.state(6000))
	_support.expect_equal(meter.meter_text(), "◆ Ready", "idle characters show ready")
	_support.expect_equal(meter.segments()[0]["fill"], 1.0, "idle characters do not keep pulsing")
	meter.free()

func _frame(now: String, ready: String, can_act: bool) -> Dictionary:
	return {"logical_time": now, "ready_at": ready, "can_act": can_act, "warmed_spell": null}
