extends RefCounted

var _support: TestSupport


func test_same_epoch_consuming_result_and_replay_advance_once() -> void:
	var state: ControlState = ControlState.new()
	state.accept_welcome("7")
	_support.expect_equal(state.active_next_sequence(), "1", "epoch 7 starts at one")
	var pending: PendingCommand = _pending("command-1", "7", "1")
	_support.expect(state.install_pending(pending), "one pending command installs")
	var first: Dictionary = state.settle_pending("command-1", {"kind": "accepted"})
	_support.expect(first["settled"] and first["consumed"], "accepted result settles and consumes")
	_support.expect_equal(state.active_next_sequence(), "2", "epoch 7 advances once")
	var replay: Dictionary = state.settle_pending("command-1", {"kind": "accepted"})
	_support.expect(replay["duplicate"], "terminal replay is recognized")
	_support.expect_equal(state.active_next_sequence(), "2", "terminal replay does not advance twice")


func test_lost_result_reconnect_old_epoch_does_not_advance_new_cursor() -> void:
	var state: ControlState = ControlState.new()
	state.accept_welcome("7")
	state.install_pending(_pending("command-1", "7", "1"))
	state.accept_welcome("8")
	_support.expect_equal(state.active_next_sequence(), "1", "fresh epoch 8 starts at one")
	var receipt: Dictionary = state.settle_pending("command-1", {"kind": "accepted"})
	_support.expect(receipt["settled"], "old-epoch receipt settles retained command")
	_support.expect_equal(state.next_sequence_by_epoch["7"], "2", "retained epoch consumes once")
	_support.expect_equal(state.active_control_epoch, "8", "active epoch remains 8")
	_support.expect_equal(state.active_next_sequence(), "1", "old receipt never advances new cursor")
	_support.expect(state.install_pending(_pending("command-2", "8", "1")), "next fresh intent uses epoch 8 sequence one")


func test_consuming_and_nonconsuming_dispositions_keep_sequence_contract() -> void:
	for disposition: Dictionary in [{"kind": "accepted"}, {"kind": "rejected", "code": "rules_rejected"}]:
		var consuming: ControlState = ControlState.new()
		consuming.accept_welcome("9")
		consuming.install_pending(_pending("consume-" + str(disposition), "9", "1"))
		consuming.settle_pending("consume-" + str(disposition), disposition)
		_support.expect_equal(consuming.active_next_sequence(), "2", "consuming disposition advances")
	for code: String in ["wrong_actor", "stale_control_epoch", "future_world_revision", "out_of_order_client_sequence", "projection_failed"]:
		var rejected: ControlState = ControlState.new()
		rejected.accept_welcome("9")
		rejected.install_pending(_pending("reject-" + code, "9", "1"))
		rejected.settle_pending("reject-" + code, {"kind": "rejected", "code": code})
		_support.expect_equal(rejected.active_next_sequence(), "1", code + " does not consume")
		_support.expect(rejected.install_pending(_pending("corrected-" + code, "9", "1")), "corrected command gets a new ID at same sequence")
	var expired: ControlState = ControlState.new()
	expired.accept_welcome("9")
	expired.install_pending(_pending("expired", "9", "1"))
	expired.settle_pending("expired", {"kind": "command_result_expired"})
	_support.expect_equal(expired.active_next_sequence(), "1", "expired result does not consume")


func _pending(command_id: String, epoch: String, sequence: String) -> PendingCommand:
	return PendingCommand.create("synthetic-envelope".to_utf8_buffer(), command_id, epoch, sequence, "1", "player", {"kind": "wait"})
