extends RefCounted

var _support: TestSupport


func test_an_unconfigured_build_keeps_the_tracked_default() -> void:
	_clear_environment()
	var fallback := _fallback()
	var resolved := EndpointConfig.resolve(fallback)
	_support.expect(resolved == fallback, "an unconfigured build keeps the tracked default endpoint")


func test_the_environment_names_the_server_and_derives_the_rest() -> void:
	_clear_environment()
	OS.set_environment("TME_EX_HTTPS_BASE_URL", "https://tme.invalid")
	var resolved := EndpointConfig.resolve(_fallback())
	_support.expect(
		resolved.https_base_url == "https://tme.invalid",
		"the environment names the server",
	)
	_support.expect(
		resolved.websocket_url == "wss://tme.invalid/v3/socket",
		"the socket URL is derived from the base URL",
	)
	_support.expect(resolved.origin == "https://tme.invalid", "the origin is derived from the base URL")
	_support.expect(resolved.validation_errors().is_empty(), "the derived endpoint validates")
	_clear_environment()


func test_explicit_socket_and_origin_survive_resolution() -> void:
	_clear_environment()
	OS.set_environment("TME_EX_HTTPS_BASE_URL", "https://tme.invalid")
	OS.set_environment("TME_EX_WEBSOCKET_URL", "wss://tme.invalid/v3/socket")
	OS.set_environment("TME_EX_ORIGIN", "https://tme.invalid")
	var resolved := EndpointConfig.resolve(_fallback())
	_support.expect(
		resolved.websocket_url == "wss://tme.invalid/v3/socket",
		"an explicit socket URL is preserved",
	)
	_support.expect(resolved.validation_errors().is_empty(), "explicitly supplied values validate together")
	_clear_environment()


func test_a_trailing_slash_does_not_corrupt_the_derived_socket_url() -> void:
	_clear_environment()
	OS.set_environment("TME_EX_HTTPS_BASE_URL", "https://tme.invalid/")
	var resolved := EndpointConfig.resolve(_fallback())
	_support.expect(
		resolved.websocket_url == "wss://tme.invalid/v3/socket",
		"a trailing slash on the base URL does not produce a doubled path",
	)
	_support.expect(resolved.validation_errors().is_empty(), "the trimmed endpoint validates")
	_clear_environment()


func test_a_test_ca_path_turns_on_integration_mode() -> void:
	_clear_environment()
	OS.set_environment("TME_EX_HTTPS_BASE_URL", "https://localhost:8443")
	OS.set_environment("TME_EX_CA_PATH", "/tmp/tme-test-ca.pem")
	var resolved := EndpointConfig.resolve(_fallback())
	_support.expect(resolved.integration_test_mode, "a test CA path turns on integration mode")
	_support.expect(
		resolved.integration_test_ca_path == "/tmp/tme-test-ca.pem",
		"the test CA path is carried through resolution",
	)
	_clear_environment()


func _fallback() -> EndpointConfig:
	var fallback := EndpointConfig.new()
	fallback.https_base_url = "https://tme.invalid"
	fallback.websocket_url = "wss://tme.invalid/v3/socket"
	fallback.origin = "https://tme.invalid"
	return fallback


func _clear_environment() -> void:
	for name: String in [
		"TME_EX_HTTPS_BASE_URL",
		"TME_EX_WEBSOCKET_URL",
		"TME_EX_ORIGIN",
		"TME_EX_CA_PATH",
	]:
		OS.set_environment(name, "")
