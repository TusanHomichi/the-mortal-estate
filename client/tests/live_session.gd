extends RefCounted

## The part of a live proof that is the same for every live proof: mount the
## shipped `ClientRoot.tscn`, sign in, select a character, and wait until the
## client actually holds authority.
##
## It asserts nothing. Each proof decides for itself what has to be true along
## the way — one walks the whole session and checks every step, another only
## needs a real frame to photograph — so this owns the sequence and they own the
## claims. Two copies of the sequence would be two chances to drift from what
## the shipped scene does.

const STEP_TIMEOUT_MSEC: int = 30000

var tree: SceneTree
var client: ClientRoot


func _init(scene_tree: SceneTree) -> void:
	tree = scene_tree


## Environment-supplied credentials, or an empty dictionary when any is absent.
static func credentials() -> Dictionary:
	var username: String = OS.get_environment(DevCredentials.USERNAME_VARIABLE)
	var password: String = OS.get_environment(DevCredentials.PASSWORD_VARIABLE)
	var character_id: String = OS.get_environment("TME_EX_CHARACTER_ID").strip_edges()
	if username.is_empty() or password.is_empty() or character_id.is_empty():
		return {}
	return {"username": username, "password": password, "character_id": character_id}


## Mounts the shipped client scene and returns it.
func mount() -> ClientRoot:
	client = (load("res://scenes/ClientRoot.tscn") as PackedScene).instantiate() as ClientRoot
	tree.root.add_child(client)
	await tree.process_frame
	return client


## Signs in and waits for the session bootstrap. Returns false on timeout.
func sign_in(username: String, password: String) -> bool:
	client._on_login_requested(username, password)
	return await wait_until(func() -> bool:
		return client.state_machine.current == ConnectionStateMachine.BOOTSTRAPPED
	, STEP_TIMEOUT_MSEC)


## Selects a character and waits until the client is online **and** holding an
## authoritative frame. Both halves matter: an online socket with no installed
## frame is a client with nothing to present.
func select_character(character_id: String) -> bool:
	client._on_character_selected(character_id)
	return await wait_until(func() -> bool:
		return (
			client.state_machine.current == ConnectionStateMachine.ONLINE
			and client.authoritative_state.has_authority()
		), STEP_TIMEOUT_MSEC)


func wait_until(predicate: Callable, timeout_msec: int) -> bool:
	var deadline: int = Time.get_ticks_msec() + timeout_msec
	while Time.get_ticks_msec() < deadline:
		await tree.process_frame
		if bool(predicate.call()):
			return true
	return false


func release() -> void:
	if is_instance_valid(client):
		client.free()
	client = null
