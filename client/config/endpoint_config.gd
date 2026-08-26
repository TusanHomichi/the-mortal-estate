class_name EndpointConfig
extends Resource

@export var https_base_url: String
@export var websocket_url: String
@export var origin: String
@export var integration_test_mode: bool = false
@export var integration_test_ca_path: String = ""


## Returns the endpoint an exported build should use.
##
## The tracked default is a deliberate placeholder, so a shipped client needs a
## way to name its server. `--server https://host` wins; `TME_EX_HTTPS_BASE_URL`
## and its siblings are the scripted path. Neither present means the caller gets
## `fallback` unchanged, and its validation errors surface as they always did.
static func resolve(fallback: EndpointConfig) -> EndpointConfig:
	var base: String = _server_argument()
	if base.is_empty():
		base = OS.get_environment("TME_EX_HTTPS_BASE_URL").strip_edges()
	if base.is_empty():
		return fallback

	base = base.rstrip("/")
	var resolved: EndpointConfig = EndpointConfig.new()
	resolved.https_base_url = base

	var websocket: String = OS.get_environment("TME_EX_WEBSOCKET_URL").strip_edges()
	resolved.websocket_url = websocket if not websocket.is_empty() else _websocket_for(base)

	var origin_value: String = OS.get_environment("TME_EX_ORIGIN").strip_edges()
	resolved.origin = origin_value if not origin_value.is_empty() else base

	var ca_path: String = OS.get_environment("TME_EX_CA_PATH").strip_edges()
	resolved.integration_test_ca_path = ca_path
	resolved.integration_test_mode = not ca_path.is_empty()
	return resolved


static func _server_argument() -> String:
	var arguments: PackedStringArray = OS.get_cmdline_user_args()
	for index in arguments.size():
		var argument: String = arguments[index]
		if argument.begins_with("--server="):
			return argument.substr("--server=".length()).strip_edges()
		if argument == "--server" and index + 1 < arguments.size():
			return arguments[index + 1].strip_edges()
	return ""


static func _websocket_for(https_base: String) -> String:
	if not https_base.begins_with("https://"):
		return ""
	return "wss://" + https_base.substr("https://".length()) + "/v3/socket"


func validation_errors() -> PackedStringArray:
	var errors: PackedStringArray = PackedStringArray()
	var https_authority: String = _authority_for(https_base_url, "https")
	var websocket_authority: String = _authority_for(websocket_url, "wss")
	var origin_authority: String = _origin_authority(origin)

	if https_authority.is_empty():
		errors.append("HTTPS endpoint must be an https URL without userinfo, query, or fragment")
	if websocket_authority.is_empty():
		errors.append("WebSocket endpoint must be a wss URL without userinfo, query, or fragment")
	if origin_authority.is_empty():
		errors.append("Origin must contain only the canonical https scheme and authority")
	if not https_authority.is_empty() and not websocket_authority.is_empty() and https_authority != websocket_authority:
		errors.append("HTTPS and WebSocket authorities must match")
	if not https_authority.is_empty() and not origin_authority.is_empty() and https_authority != origin_authority:
		errors.append("Endpoint and Origin authorities must match")

	if integration_test_ca_path.is_empty():
		return errors
	if not integration_test_mode:
		errors.append("A test CA path requires integration test mode")
	elif not integration_test_ca_path.is_absolute_path():
		errors.append("The integration test CA path must be absolute")
	return errors


func build_tls_options() -> TLSOptions:
	if not validation_errors().is_empty():
		return null
	if integration_test_ca_path.is_empty():
		return TLSOptions.client()
	var trusted_chain: X509Certificate = X509Certificate.new()
	var load_error: Error = trusted_chain.load(integration_test_ca_path)
	if load_error != OK:
		return null
	return TLSOptions.client(trusted_chain)


func _authority_for(value: String, scheme: String) -> String:
	if value.contains("?") or value.contains("#") or value.contains("@"):
		return ""
	var prefix: String = scheme + "://"
	if not value.begins_with(prefix):
		return ""
	var remainder: String = value.substr(prefix.length())
	var slash_index: int = remainder.find("/")
	var authority: String = remainder if slash_index < 0 else remainder.substr(0, slash_index)
	if authority.is_empty() or authority.strip_edges() != authority:
		return ""
	return authority


func _origin_authority(value: String) -> String:
	var authority: String = _authority_for(value, "https")
	if authority.is_empty() or value != "https://" + authority:
		return ""
	return authority
