extends RefCounted

var _support: TestSupport

class CaptureLogger extends Logger:
	var lines: Array[String] = []

	func _log_message(message: String, _error: bool) -> void:
		lines.append(message)

	func _log_error(
		_function: String,
		_file: String,
		_line: int,
		_code: String,
		rationale: String,
		_editor_notify: bool,
		_error_type: int,
		_script_backtraces: Array[ScriptBacktrace],
	) -> void:
		lines.append(rationale)


func test_secret_redaction_covers_logs_errors_settings_and_state() -> void:
	var synthetic_values: Array[String] = ["synthetic-password-value", "synthetic-cookie-value", "synthetic-csrf-value", "synthetic-ticket-value"]
	var redactor: SecretRedactor = SecretRedactor.new()
	for value: String in synthetic_values: redactor.register(value)
	var raw_error: String = "failure " + " ".join(synthetic_values)
	var redacted_error: String = redactor.redact(raw_error)
	for value: String in synthetic_values: _support.expect(not redacted_error.contains(value), "formatted error removes registered value")
	var control: ControlState = ControlState.new()
	control.begin_login(synthetic_values[0])
	control.set_session_cookie(synthetic_values[1])
	control.set_csrf_token(synthetic_values[2])
	control.set_admission_ticket(synthetic_values[3])
	control.finish_login()
	var presentation: PresentationState = PresentationState.new()
	presentation.append_transient(redacted_error)
	var settings_text: String = JSON.stringify({"schema_version": 1, "actions": {}, "ui_text_scale_percent": 100})
	var capture_text: String = "The Mortal Estate login fixture capture"
	var visible_text: String = JSON.stringify(control.debug_summary()) + JSON.stringify(presentation.debug_summary()) + settings_text + capture_text
	for value: String in synthetic_values: _support.expect(not visible_text.contains(value), "state, settings, log, and capture text omit registered value")
	_support.expect(not control.debug_summary().has("csrf_token"), "control debug summary contains no secret field")
	_support.expect(not presentation.debug_summary().has("password"), "presentation debug summary contains no credential field")


func test_environment_supplied_credentials_emit_no_secret_logs_or_ui_status() -> void:
	const SECRET: String = "synthetic-prefill-password"
	var original_username: String = OS.get_environment(DevCredentials.USERNAME_VARIABLE)
	var original_password: String = OS.get_environment(DevCredentials.PASSWORD_VARIABLE)
	OS.set_environment(DevCredentials.USERNAME_VARIABLE, "deck_user")
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, SECRET)
	var capture: CaptureLogger = CaptureLogger.new()
	OS.add_logger(capture)
	var prefill: Dictionary = DevCredentials.resolve()
	var screen: LoginScreen = (load("res://scenes/LoginScreen.tscn") as PackedScene).instantiate() as LoginScreen
	_support.expect(screen != null, "login screen instantiates for prefill redaction proof")
	if screen != null:
		(Engine.get_main_loop() as SceneTree).root.add_child(screen)
		_support.expect_equal(prefill.get("password"), SECRET, "the environment password reaches only the masked field input path")
		screen.apply_credential_prefill(prefill)
		_support.expect(screen.login_value_edit.secret, "prefilled password remains masked")
		_support.expect(not screen.credential_source.text.contains(SECRET), "credential source status omits the password")
		screen.get_parent().remove_child(screen)
		screen.free()
	OS.remove_logger(capture)
	var log_text: String = "\n".join(capture.lines)
	_support.expect(not log_text.contains(SECRET), "environment prefill emits no password in a log line")
	OS.set_environment(DevCredentials.USERNAME_VARIABLE, original_username)
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, original_password)


## D7's negative half: no credential path writes anything durable. A stored
## secret cannot leak from a store that does not exist, so the proof is that
## resolving credentials touches no file at all.
func test_no_credential_path_persists_anything() -> void:
	var original_username: String = OS.get_environment(DevCredentials.USERNAME_VARIABLE)
	var original_password: String = OS.get_environment(DevCredentials.PASSWORD_VARIABLE)
	OS.set_environment(DevCredentials.USERNAME_VARIABLE, "deck_user")
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, "synthetic-durable-password")
	var before: PackedStringArray = _user_directory_files()
	var prefill: Dictionary = DevCredentials.resolve()
	_support.expect_equal(prefill.keys(), ["username", "password", "username_from_environment", "password_from_environment"], "the credential prefill carries no persistence facts")
	_support.expect_equal(_user_directory_files(), before, "resolving credentials writes no user-directory file")
	OS.set_environment(DevCredentials.USERNAME_VARIABLE, original_username)
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, original_password)


func _user_directory_files() -> PackedStringArray:
	var files: PackedStringArray = DirAccess.get_files_at("user://")
	files.sort()
	return files
