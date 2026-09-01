extends SceneTree

const TestSupportScript: Script = preload("res://tests/test_support.gd")
const StrictJsonSuite: Script = preload("res://tests/test_strict_json.gd")
const WireCodecSuite: Script = preload("res://tests/test_wire_codec.gd")
const StateReducerSuite: Script = preload("res://tests/test_state_reducer.gd")
const EpochCursorSuite: Script = preload("res://tests/test_epoch_cursor.gd")
const PathPreviewStateSuite: Script = preload("res://tests/test_path_preview_state.gd")
const SecretRedactionSuite: Script = preload("res://tests/test_secret_redaction.gd")
const ControlFacadeSuite: Script = preload("res://tests/test_control_facade.gd")
const ConnectionStateMachineSuite: Script = preload("res://tests/test_connection_state_machine.gd")
const ClientRootLivePlaySuite: Script = preload("res://tests/test_client_root_live_play.gd")
const InputBindingsSuite: Script = preload("res://tests/test_input_bindings.gd")
const PointerMovementSuite: Script = preload("res://tests/test_pointer_movement.gd")
const WorldShellActionsSuite: Script = preload("res://tests/test_world_shell_actions.gd")
const DomainPanelSuite: Script = preload("res://tests/test_domain_panel.gd")
const EndpointResolutionSuite: Script = preload("res://tests/test_endpoint_resolution.gd")
const TextEntryKeyboardSuite: Script = preload("res://tests/test_text_entry_keyboard.gd")
const FeedbackPresenterSuite: Script = preload("res://tests/test_feedback_presenter.gd")
const FullHudSuite: Script = preload("res://tests/test_full_hud.gd")
const CombatFeelDirectorSuite: Script = preload("res://tests/test_combat_feel_director.gd")
const AudioManifestLoaderSuite: Script = preload("res://tests/test_audio_manifest_loader.gd")
const SpellPaletteSuite: Script = preload("res://tests/test_spell_palette.gd")
const PulseClockSuite: Script = preload("res://tests/test_pulse_clock.gd")
const WorldTargetsSuite: Script = preload("res://tests/test_world_targets.gd")
const GridWorldViewSuite: Script = preload("res://tests/test_grid_world_view.gd")
const InteractionDirectorSuite: Script = preload("res://tests/test_interaction_director.gd")
const GroundTraySuite: Script = preload("res://tests/test_ground_tray.gd")
const DevCredentialsSuite: Script = preload("res://tests/test_dev_credentials.gd")
const FeelSceneSuite: Script = preload("res://tests/test_feel_scene.gd")

class ScriptErrorMonitor extends Logger:
	var _mutex: Mutex = Mutex.new()
	var _errors: Array[String] = []

	func _log_message(_message: String, _error: bool) -> void:
		pass

	func _log_error(
		function: String,
		file: String,
		line: int,
		code: String,
		rationale: String,
		_editor_notify: bool,
		error_type: int,
		_script_backtraces: Array[ScriptBacktrace],
	) -> void:
		if error_type != Logger.ERROR_TYPE_SCRIPT:
			return
		var message: String = rationale if not rationale.is_empty() else code
		_mutex.lock()
		_errors.append("%s:%d in %s: %s" % [file, line, function, message])
		_mutex.unlock()

	func errors() -> Array[String]:
		_mutex.lock()
		var result: Array[String] = _errors.duplicate()
		_mutex.unlock()
		return result


var _script_error_monitor: ScriptErrorMonitor


func _initialize() -> void:
	_script_error_monitor = ScriptErrorMonitor.new()
	OS.add_logger(_script_error_monitor)
	call_deferred("_run")


func _run() -> void:
	var support: TestSupport = TestSupportScript.new()
	var executed: Array[String] = []
	support.current_test = "test_support/test_all_client_scripts_parse"
	support.test_all_client_scripts_parse()
	executed.append(support.current_test)
	support.current_test = ""
	var suites: Dictionary = {
		"strict_json": StrictJsonSuite,
		"wire_codec": WireCodecSuite,
		"state_reducer": StateReducerSuite,
		"epoch_cursor": EpochCursorSuite,
		"path_preview_state": PathPreviewStateSuite,
		"secret_redaction": SecretRedactionSuite,
		"control_facade": ControlFacadeSuite,
		"connection_state_machine": ConnectionStateMachineSuite,
		"client_root_live_play": ClientRootLivePlaySuite,
		"input_bindings": InputBindingsSuite,
		"pointer_movement": PointerMovementSuite,
		"world_shell_actions": WorldShellActionsSuite,
		"domain_panel": DomainPanelSuite,
		"endpoint_resolution": EndpointResolutionSuite,
		"text_entry_keyboard": TextEntryKeyboardSuite,
		"feedback_presenter": FeedbackPresenterSuite,
		"full_hud": FullHudSuite,
		"combat_feel_director": CombatFeelDirectorSuite,
		"audio_manifest_loader": AudioManifestLoaderSuite,
		"spell_palette": SpellPaletteSuite,
		"pulse_clock": PulseClockSuite,
		"world_targets": WorldTargetsSuite,
		"grid_world_view": GridWorldViewSuite,
		"interaction_director": InteractionDirectorSuite,
		"ground_tray": GroundTraySuite,
		"dev_credentials": DevCredentialsSuite,
		"feel_scene": FeelSceneSuite,
	}
	var selected_suites: PackedStringArray = _selected_suites()
	if _script_error_probe_requested():
		executed.append("run_all/script_error_guard_probe")
		await _run_script_error_probe()
	else:
		for suite_name: String in suites.keys():
			if not selected_suites.is_empty() and suite_name not in selected_suites:
				continue
			executed.append_array(await support.run_suite((suites[suite_name] as Script).new(), suite_name))
	OS.remove_logger(_script_error_monitor)
	for script_error: String in _script_error_monitor.errors():
		support.failures.append("GDScript runtime error: " + script_error)
	for test_name: String in executed:
		print(test_name)
	for failure: String in support.failures:
		printerr(failure)
	if executed.is_empty() or support.assertion_count == 0 or not support.failures.is_empty():
		quit(1)
	else:
		quit(0)


func _selected_suites() -> PackedStringArray:
	var selected: PackedStringArray = PackedStringArray()
	for argument: String in OS.get_cmdline_user_args():
		if not argument.begins_with("--suite="):
			continue
		var suite_name: String = argument.trim_prefix("--suite=")
		if not suite_name.is_empty() and suite_name not in selected:
			selected.append(suite_name)
	selected.sort()
	return selected


func _script_error_probe_requested() -> bool:
	return "--probe-script-error-guard" in OS.get_cmdline_user_args()


func _run_script_error_probe() -> void:
	await process_frame
	var missing: Dictionary = {}
	var _unreachable: Variant = missing.intent
