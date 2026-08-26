class_name LoginScreen
extends Control

signal login_requested(username: String, login_value: String)

@onready var endpoint_identity: Label = %EndpointIdentity
@onready var username_edit: LineEdit = %Username
@onready var login_value_edit: LineEdit = %LoginValue
@onready var credential_source: Label = %CredentialSource
@onready var error_label: Label = %ErrorLabel
@onready var login_button: Button = %LoginButton

var text_entry_keyboard: TextEntryKeyboard = TextEntryKeyboard.new()


func _ready() -> void:
	login_button.pressed.connect(_on_login_pressed)
	text_entry_keyboard.bind(username_edit)
	text_entry_keyboard.bind(login_value_edit, DisplayServer.KEYBOARD_TYPE_PASSWORD)
	username_edit.focus_neighbor_bottom = username_edit.get_path_to(login_value_edit)
	login_value_edit.focus_neighbor_top = login_value_edit.get_path_to(username_edit)
	login_value_edit.focus_neighbor_bottom = login_value_edit.get_path_to(login_button)
	login_button.focus_neighbor_top = login_button.get_path_to(login_value_edit)
	login_button.focus_neighbor_bottom = login_button.get_path_to(username_edit)
	username_edit.focus_neighbor_top = username_edit.get_path_to(login_button)


func configure_endpoint(config: EndpointConfig) -> void:
	endpoint_identity.text = "Endpoint: " + config.origin


## Applies this run's environment-supplied prefill. The password reaches the
## masked field and nothing else — no status line ever quotes a credential.
func apply_credential_prefill(prefill: Dictionary) -> void:
	username_edit.text = str(prefill.get("username", ""))
	login_value_edit.text = str(prefill.get("password", ""))
	if prefill.get("password_from_environment", false) or prefill.get("username_from_environment", false):
		credential_source.text = "Sign-in supplied by the environment for this run. Select a field to edit it."
	else:
		credential_source.text = "This client saves no credential. Sign in each run."
	if text_entry_keyboard.strategy() == TextEntryKeyboard.STRATEGY_STEAM_OVERLAY:
		credential_source.text += " Steam's keyboard opens for text fields."


func show_error(message: String) -> void:
	error_label.text = "Error: " + message if not message.is_empty() else ""
	error_label.visible = not message.is_empty()


func set_busy(busy: bool) -> void:
	username_edit.editable = not busy
	login_value_edit.editable = not busy
	login_button.disabled = busy
	login_button.text = "Signing in…" if busy else "Sign In"


func first_focus_control() -> Control:
	if username_edit.text.is_empty():
		return username_edit
	if login_value_edit.text.is_empty():
		return login_value_edit
	return login_button


func apply_text_scale(percent: int) -> void:
	var scaled_theme: Theme = Theme.new()
	scaled_theme.default_font_size = 16 * percent / 100
	theme = scaled_theme


func _on_login_pressed() -> void:
	var username: String = username_edit.text
	var login_value: String = login_value_edit.text
	text_entry_keyboard.hide()
	login_requested.emit(username, login_value)
	login_value_edit.clear()
