extends RefCounted

var _support: TestSupport


func test_credentials_come_from_the_environment_and_nowhere_else() -> void:
	var original_username: String = OS.get_environment(DevCredentials.USERNAME_VARIABLE)
	var original_password: String = OS.get_environment(DevCredentials.PASSWORD_VARIABLE)

	OS.set_environment(DevCredentials.USERNAME_VARIABLE, "  deck_user  ")
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, "  spaced secret  ")
	var supplied: Dictionary = DevCredentials.resolve()
	_support.expect_equal(supplied["username"], "deck_user", "a username is trimmed")
	_support.expect_equal(supplied["password"], "  spaced secret  ", "a password is taken exactly, since its edges may be meaningful")
	_support.expect(supplied["username_from_environment"] and supplied["password_from_environment"], "supplied credentials report their source")

	OS.set_environment(DevCredentials.USERNAME_VARIABLE, "")
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, "")
	var absent: Dictionary = DevCredentials.resolve()
	_support.expect_equal(absent["username"], "", "an unsupplied username is empty")
	_support.expect_equal(absent["password"], "", "an unsupplied password is empty")
	_support.expect(not absent["username_from_environment"] and not absent["password_from_environment"], "absent credentials report no source")

	OS.set_environment(DevCredentials.USERNAME_VARIABLE, original_username)
	OS.set_environment(DevCredentials.PASSWORD_VARIABLE, original_password)


func test_the_login_screen_offers_no_way_to_persist_a_credential() -> void:
	var screen: LoginScreen = (load("res://scenes/LoginScreen.tscn") as PackedScene).instantiate() as LoginScreen
	(Engine.get_main_loop() as SceneTree).root.add_child(screen)
	var names: Array[String] = []
	for node: Node in screen.find_children("*", "", true, false):
		names.append(node.name)
	for retired: String in ["RememberPassword", "CredentialStatus", "ForgetSavedLogin"]:
		_support.expect(not names.has(retired), "the login screen has no " + retired + " control")
	_support.expect(names.has("CredentialSource"), "the login screen states where its credentials come from")

	var emitted: Array[Array] = []
	screen.login_requested.connect(func(username: String, login_value: String) -> void: emitted.append([username, login_value]))
	screen.username_edit.text = "deck_user"
	screen.login_value_edit.text = "synthetic-sign-in-password"
	screen.login_button.pressed.emit()
	_support.expect_equal(emitted, [["deck_user", "synthetic-sign-in-password"]], "sign-in carries the typed credential and no persistence choice")
	_support.expect_equal(screen.login_value_edit.text, "", "the password field is cleared the moment it is submitted")

	screen.apply_credential_prefill(DevCredentials.resolve())
	_support.expect("saves no credential" in screen.credential_source.text or "supplied by the environment" in screen.credential_source.text, "the source line states the model in plain words")
	screen.free()
