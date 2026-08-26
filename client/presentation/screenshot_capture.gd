extends RefCounted

## Writes a PNG of the current frame to a stable, discoverable location.
##
## Godot captures its own viewport, so this works where compositor-level tools
## cannot — notably Steam Deck Game Mode, whose gamescope build does not
## implement the Wayland screencopy protocol. That gap is the whole reason this
## exists: without it there is no way to see what the client is drawing on a
## Deck short of pointing a camera at the screen.

const DIRECTORY: String = "user://screenshots"
const RETAINED: int = 40


## Saves the viewport and returns the absolute path written, or "" on failure.
static func capture(viewport: Viewport) -> String:
	if viewport == null:
		push_error("screenshot: no viewport")
		return ""
	var texture: ViewportTexture = viewport.get_texture()
	if texture == null:
		push_error("screenshot: viewport has no texture")
		return ""
	var image: Image = texture.get_image()
	if image == null:
		push_error("screenshot: viewport texture produced no image")
		return ""

	if not DirAccess.dir_exists_absolute(DIRECTORY):
		var make_error: Error = DirAccess.make_dir_recursive_absolute(DIRECTORY)
		if make_error != OK:
			push_error("screenshot: could not create %s (%d)" % [DIRECTORY, make_error])
			return ""

	var path: String = "%s/tme-%s.png" % [DIRECTORY, _stamp()]
	var save_error: Error = image.save_png(path)
	if save_error != OK:
		push_error("screenshot: could not write %s (%d)" % [path, save_error])
		return ""

	_prune()
	var absolute: String = ProjectSettings.globalize_path(path)
	# Printed deliberately: on a headless-managed device the operator finds the
	# file by reading the log, not by browsing a filesystem they cannot see.
	print("screenshot: %s" % absolute)
	return absolute


static func _stamp() -> String:
	var now: Dictionary = Time.get_datetime_dict_from_system()
	return "%04d%02d%02d-%02d%02d%02d" % [
		now["year"], now["month"], now["day"], now["hour"], now["minute"], now["second"],
	]


## Keeps the directory bounded so a held key cannot fill the device.
static func _prune() -> void:
	var directory: DirAccess = DirAccess.open(DIRECTORY)
	if directory == null:
		return
	var names: PackedStringArray = PackedStringArray()
	directory.list_dir_begin()
	var name: String = directory.get_next()
	while not name.is_empty():
		if not directory.current_is_dir() and name.ends_with(".png"):
			names.append(name)
		name = directory.get_next()
	directory.list_dir_end()
	if names.size() <= RETAINED:
		return
	names.sort()
	for index in names.size() - RETAINED:
		directory.remove(names[index])
