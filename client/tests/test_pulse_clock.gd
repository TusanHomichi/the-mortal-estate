extends RefCounted

## [PulseClock] and [PulseMeter] — the beat made visible.
##
## The claim under test is not "a bar moves". It is that everything the beat
## meter asserts came out of an authoritative frame, and that the one thing it
## derives locally — how far into the current beat presentation has got — is
## measured from the frame's own arrivals, is never extrapolated past what was
## measured, and can never turn into a readiness claim. Ruling D5 forbids a
## second gameplay clock and forbids inferring readiness from elapsed seconds;
## these are the cases that would catch either creeping in.

const SPAN: int = 3000

var _support: TestSupport


func test_the_beat_comes_from_the_frame_and_nowhere_else() -> void:
	var clock: PulseClock = PulseClock.new()
	_support.expect(not clock.has_authority(), "a clock with no frame claims no beat")
	_support.expect(not clock.is_ready(), "a clock with no frame is not ready")
	_support.expect_equal(clock.beat_fill(1000), 0.0, "a clock with no frame has no fill")

	clock.note_frame(_frame("40", "43", false), 10000)
	_support.expect(clock.has_authority(), "a frame installs the beat")
	_support.expect_equal(clock.logical_time(), "40", "logical time is the frame's own")
	_support.expect_equal(clock.ready_at(), "43", "readiness time is the frame's own")
	_support.expect_equal(clock.beats_until_ready(), 3, "the wait is the whole-round distance between the two")
	_support.expect(not clock.is_ready(), "readiness is the frame's can_act")

	clock.note_frame(_frame("41", "41", true), 10000 + SPAN)
	_support.expect_equal(clock.beats_until_ready(), 0, "a wait of nothing is nothing")
	_support.expect(clock.is_ready(), "the frame's can_act is the whole readiness answer")


func test_ready_at_before_the_frames_own_clock_is_the_ordinary_ready_state() -> void:
	# The rules make an actor ready when `ready_at <= now`, so a character who
	# has been idle for four rounds carries a readiness time four rounds behind
	# the world's. That is not a contradiction to refuse; it is what standing
	# still looks like, and a meter that rejected it would blank itself during
	# ordinary play.
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("99", "95", true), 500)
	_support.expect(clock.has_authority(), "a readiness time behind the world's clock is an ordinary frame")
	_support.expect(clock.is_ready(), "it is the ready state")
	_support.expect_equal(clock.beats_until_ready(), 0, "there are no beats left to wait")


func test_no_fill_is_claimed_before_a_beat_has_been_measured() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("7", "8", false), 1000)
	_support.expect(not clock.has_measured_span(), "one arrival is not an interval")
	_support.expect_equal(clock.beat_fill(1000 + SPAN / 2), 0.0, "an unmeasured beat has no fill to show")
	_support.expect_equal(clock.beat_deadline_msec(), -1, "an unmeasured beat has no deadline")

	var meter: PulseMeter = PulseMeter.new()
	meter.present_pulse(clock.state(1000 + SPAN / 2))
	var segments: Array[Dictionary] = meter.segments()
	_support.expect_equal(segments.size(), 1, "one beat of wait draws one segment")
	_support.expect(not bool(segments[0]["measured"]), "the segment says the beat is unmeasured rather than empty")
	_support.expect(meter.meter_text().contains("beat unmeasured"), "the words say so too")
	_support.expect(not meter.meter_text().contains("%"), "and never quote a fill it has not measured")
	meter.free()


func test_the_fill_is_the_measured_beat_and_resets_on_the_authoritative_one() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("7", "9", false), 1000)
	clock.note_frame(_frame("8", "9", false), 1000 + SPAN)
	_support.expect(clock.has_measured_span(), "two consecutive rounds are one interval")
	_support.expect_equal(clock.span_msec(), SPAN, "the span is the interval that was observed, not one that was assumed")
	_support.expect_equal(clock.beat_fill(1000 + SPAN), 0.0, "the beat starts empty")
	_support.expect(absf(clock.beat_fill(1000 + SPAN + SPAN / 2) - 0.5) < 0.01, "half a beat is half a fill")
	_support.expect_equal(clock.beat_deadline_msec(), 1000 + SPAN + SPAN, "the beat is due one measured span after it started")

	clock.note_frame(_frame("9", "9", true), 1000 + SPAN + SPAN)
	_support.expect_equal(clock.beat_fill(1000 + SPAN + SPAN), 0.0, "the authoritative beat resets the fill")


func test_the_fill_holds_at_full_rather_than_inventing_beats_that_never_arrived() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("7", "9", false), 1000)
	clock.note_frame(_frame("8", "9", false), 1000 + SPAN)
	_support.expect_equal(clock.beat_fill(1000 + SPAN + SPAN), 1.0, "the beat fills to full")
	_support.expect_equal(
		clock.beat_fill(1000 + SPAN + SPAN * 40),
		1.0,
		"a client told nothing for forty beats freezes at the end of the beat it was told about",
	)
	_support.expect_equal(clock.logical_time(), "8", "and keeps saying the last thing authority said")


func test_readiness_is_never_inferred_from_elapsed_time() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("7", "12", false), 1000)
	clock.note_frame(_frame("8", "12", false), 1000 + SPAN)
	for elapsed: int in [0, SPAN / 2, SPAN, SPAN * 10]:
		_support.expect(
			not clock.is_ready(),
			"elapsed time does not make an actor ready at +%d ms" % elapsed,
		)
		_support.expect_equal(
			clock.beats_until_ready(),
			4,
			"the wait is the frame's arithmetic and does not count down locally at +%d ms" % elapsed,
		)
		var _fill: float = clock.beat_fill(1000 + SPAN + elapsed)


func test_authority_that_runs_backwards_is_refused_rather_than_smoothed() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("40", "41", false), 1000)
	clock.note_frame(_frame("41", "41", true), 1000 + SPAN)
	_support.expect(clock.has_measured_span(), "the ordinary case measured a span")
	clock.note_frame(_frame("39", "41", false), 1000 + SPAN * 2)
	_support.expect(not clock.has_authority(), "a frame from before the one held is not a beat")
	_support.expect(not clock.has_measured_span(), "and the measurement it would have contradicted is dropped")


func test_a_beat_that_skipped_a_round_is_not_measured_as_one() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("7", "7", true), 1000)
	clock.note_frame(_frame("9", "9", true), 1000 + SPAN * 2)
	_support.expect(
		not clock.has_measured_span(),
		"two rounds in one interval means an update went unseen; timing it would record a beat twice as long",
	)
	_support.expect_equal(clock.logical_time(), "9", "the frame is still installed — only the measurement is refused")


func test_an_interval_outside_the_accepted_bounds_is_not_a_span() -> void:
	for interval: int in [PulseClock.MINIMUM_SPAN_MSEC - 1, PulseClock.MAXIMUM_SPAN_MSEC + 1]:
		var clock: PulseClock = PulseClock.new()
		clock.note_frame(_frame("7", "7", true), 1000)
		clock.note_frame(_frame("8", "8", true), 1000 + interval)
		_support.expect(
			not clock.has_measured_span(),
			"an interval of %d ms is not a beat this meter will draw" % interval,
		)


func test_a_malformed_or_absent_frame_clears_the_beat() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("7", "8", false), 1000)
	clock.note_frame({}, 1100)
	_support.expect(not clock.has_authority(), "an empty frame is absence of authority, not a held beat")

	for bad: Dictionary in [
		_frame("", "8", false),
		_frame("07", "8", false),
		_frame("7", "-1", false),
		_frame("7", "eight", false),
	]:
		var refusing: PulseClock = PulseClock.new()
		refusing.note_frame(_frame("6", "6", true), 900)
		refusing.note_frame(bad, 1000)
		_support.expect(
			not refusing.has_authority(),
			"a non-canonical decimal is refused rather than coerced: " + str(bad.get("logical_time")) + "/" + str(bad.get("ready_at")),
		)


func test_wide_round_counts_never_pass_through_a_float() -> void:
	# 2^53 and its neighbours are exactly where a double stops being able to
	# tell integers apart, and the shared wire corpus carries them for that
	# reason. The beat arithmetic works on the digits, so it still counts.
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("9007199254740991", "9007199254740993", false), 1000)
	_support.expect_equal(clock.beats_until_ready(), 2, "two rounds apart at the precision boundary is still two rounds")
	_support.expect_equal(clock.logical_time(), "9007199254740991", "and the value itself is carried verbatim")

	var far: PulseClock = PulseClock.new()
	far.note_frame(_frame("1", "18446744073709551615", false), 1000)
	_support.expect_equal(
		far.beats_until_ready(),
		PulseClock.MAXIMUM_COUNTED_BEATS,
		"a wait wider than the meter counts saturates instead of overflowing",
	)


func test_the_preparation_band_prefers_the_world_to_the_clients_own_claim() -> void:
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("10", "11", false), 1000)
	_support.expect(clock.state().get("prepared", {}).is_empty(), "nothing is prepared to begin with")

	clock.note_prepared_intent({"kind": "physical_attack"})
	var local: Dictionary = clock.state().get("prepared", {})
	_support.expect_equal(local.get("kind"), "command", "an installed intent is the local half of the band")
	_support.expect(not bool(local.get("authoritative", true)), "and is labelled as the client's own claim")

	var warming: Dictionary = _frame("10", "11", false)
	warming["warmed_spell"] = {"spell_id": "fire_dart", "warmed_at": "10", "ready_at": "13", "status": "warming"}
	clock.note_frame(warming, 1000 + SPAN)
	var authoritative: Dictionary = clock.state().get("prepared", {})
	_support.expect_equal(authoritative.get("kind"), "warmed_spell", "the world's own preparation outranks the local one")
	_support.expect(bool(authoritative.get("authoritative", false)), "and is labelled as authority's")
	_support.expect_equal(authoritative.get("beats_remaining"), 3, "the band counts the rounds the frame named")

	clock.note_prepared_intent({"kind": "cast_warmed_spell"})
	_support.expect_equal(
		clock.state().get("prepared", {}).get("kind"),
		"warmed_spell",
		"a local intent never displaces the authoritative band",
	)
	clock.note_frame(_frame("11", "12", false), 1000 + SPAN * 2)
	_support.expect(clock.state().get("prepared", {}).is_empty(), "a frame without the warmed spell drops the band")


func test_the_meter_draws_one_segment_per_beat_of_the_wait() -> void:
	var meter: PulseMeter = PulseMeter.new()
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("10", "13", false), 1000)
	clock.note_frame(_frame("11", "13", false), 1000 + SPAN)

	meter.present_pulse(clock.state(1000 + SPAN))
	var waiting: Array[Dictionary] = meter.segments()
	_support.expect_equal(waiting.size(), 2, "two beats of wait draw two segments")
	_support.expect_equal(waiting[0]["fill"], 0.0, "the leading segment starts the beat empty")
	_support.expect_equal(waiting[1]["fill"], 0.0, "a beat still entirely ahead is empty")
	_support.expect_equal(waiting[0]["kind"], "beat", "a wait is drawn as a wait")

	meter.present_pulse(clock.state(1000 + SPAN + SPAN / 2))
	_support.expect(absf(float(meter.segments()[0]["fill"]) - 0.5) < 0.01, "the leading segment fills as the beat elapses")

	clock.note_frame(_frame("12", "12", true), 1000 + SPAN * 2)
	meter.present_pulse(clock.state(1000 + SPAN * 2))
	var ready: Array[Dictionary] = meter.segments()
	_support.expect_equal(ready.size(), 1, "an idle ready observer still shows the beat passing")
	_support.expect_equal(ready[0]["kind"], "ready", "and shows it as the ready beat")
	meter.free()


func test_the_preparation_band_never_squeezes_the_beat_out_of_the_meter() -> void:
	# The meter is short — it lives in a HUD rail — so a band sized as a share of
	# it came out about three pixels tall at exactly the height the HUD uses.
	# The band takes a fixed height and the beat keeps the rest.
	var meter: PulseMeter = PulseMeter.new()
	meter.size = PulseMeter.DEFAULT_MINIMUM_SIZE
	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("10", "11", false), 1000)
	meter.present_pulse(clock.state(1000))
	var without_band: Dictionary = meter.layout_rows()
	_support.expect(not without_band.has("band"), "nothing prepared draws no band")
	_support.expect_equal(
		(without_band["beat"] as Rect2).size.y,
		PulseMeter.DEFAULT_MINIMUM_SIZE.y,
		"and the beat row takes the whole height",
	)

	clock.note_prepared_intent({"kind": "physical_attack"})
	meter.present_pulse(clock.state(1000))
	var with_band: Dictionary = meter.layout_rows()
	_support.expect_equal((with_band["band"] as Rect2).size.y, PulseMeter.BAND_HEIGHT, "the band keeps its fixed height")
	_support.expect(
		(with_band["beat"] as Rect2).size.y >= PulseMeter.BAND_HEIGHT,
		"and the beat row is never left thinner than the band under it, at %.0f px" % (with_band["beat"] as Rect2).size.y,
	)
	_support.expect(
		(with_band["band"] as Rect2).position.y + (with_band["band"] as Rect2).size.y <= PulseMeter.DEFAULT_MINIMUM_SIZE.y,
		"and neither row is drawn outside the control",
	)
	meter.free()


func test_the_meter_states_in_words_what_it_draws_in_shape() -> void:
	var meter: PulseMeter = PulseMeter.new()
	_support.expect(meter.meter_text().contains("no authoritative frame"), "absence of authority is stated, not drawn blank")

	var clock: PulseClock = PulseClock.new()
	clock.note_frame(_frame("10", "12", false), 1000)
	clock.note_frame(_frame("11", "12", false), 1000 + SPAN)
	meter.present_pulse(clock.state(1000 + SPAN + SPAN / 2))
	var waiting: String = meter.meter_text()
	_support.expect(waiting.contains("◇") and waiting.contains("Ready in 1 beat"), "the wait carries an icon and words, never colour alone")
	_support.expect(
		not waiting.contains("%"),
		"the live fill is drawn, not restated in words: a per-frame string would relayout the rail on every frame",
	)
	_support.expect(waiting.contains("world T11") and waiting.contains("ready T12"), "the frame's own times stay on the line")
	_support.expect_equal(meter.accessibility_description, waiting, "the accessibility description is the same sentence")

	clock.note_frame(_frame("12", "12", true), 1000 + SPAN * 2)
	meter.present_pulse(clock.state(1000 + SPAN * 2))
	_support.expect(meter.meter_text().contains("◆ Ready"), "the ready state carries its own icon and word")
	meter.free()


func test_canonical_decimals_are_compared_and_differenced_on_the_digits() -> void:
	_support.expect(CanonicalDecimal.less("9", "10"), "length orders before lexicography")
	_support.expect(not CanonicalDecimal.less("10", "9"), "and the other way round")
	_support.expect(CanonicalDecimal.at_least("10", "10"), "equal is at least")
	_support.expect(not CanonicalDecimal.at_least("9", ""), "an absent bound is never met")
	_support.expect_equal(CanonicalDecimal.rounds_between("10", "13"), 3, "a small gap counts exactly")
	_support.expect_equal(CanonicalDecimal.rounds_between("13", "10"), 0, "a backwards gap is zero, not negative")
	_support.expect_equal(CanonicalDecimal.rounds_between("999", "1000"), 1, "a carry boundary counts exactly")
	_support.expect_equal(
		CanonicalDecimal.rounds_between("9007199254740991", "9007199254740993"),
		2,
		"the precision boundary a double loses is counted exactly",
	)
	_support.expect_equal(CanonicalDecimal.increment("999"), "1000", "increment carries")
	_support.expect_equal(
		CanonicalDecimal.increment("18446744073709551615"),
		"18446744073709551616",
		"increment has no width limit",
	)
	for value: String in ["", "07", "-1", "1.0", "eight", "+2"]:
		_support.expect(not CanonicalDecimal.is_canonical(value), "%s is not a canonical decimal" % value)
	for value: String in ["0", "7", "18446744073709551615"]:
		_support.expect(CanonicalDecimal.is_canonical(value), "%s is a canonical decimal" % value)


func _frame(logical_time: String, ready_at: String, can_act: bool) -> Dictionary:
	return {
		"logical_time": logical_time,
		"ready_at": ready_at,
		"can_act": can_act,
		"observer_actor_id": "player",
	}
