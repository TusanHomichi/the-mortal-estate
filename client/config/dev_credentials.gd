class_name DevCredentials
extends RefCounted

## The successor stores no credential. Private development supplies one through
## the environment, in the same `TME_EX_*` namespace the endpoint resolver and
## the server's own credential files already use; anything not supplied there is
## typed once per run and never leaves memory.
##
## There is deliberately no saved-username file, no plaintext opt-in, no forget
## button, and no read of any predecessor credential path. A durable credential
## model, if one is ever wanted, is a separately accepted design and not a
## fallback behind this one.
const USERNAME_VARIABLE: String = "TME_EX_USERNAME"
const PASSWORD_VARIABLE: String = "TME_EX_PASSWORD"


## Returns the sign-in prefill for this run. The password is returned for the
## masked field alone: no caller writes it to disk, to a log, or to any label.
static func resolve() -> Dictionary:
	var username: String = OS.get_environment(USERNAME_VARIABLE).strip_edges()
	var password: String = OS.get_environment(PASSWORD_VARIABLE)
	return {
		"username": username,
		"password": password,
		"username_from_environment": not username.is_empty(),
		"password_from_environment": not password.is_empty(),
	}
