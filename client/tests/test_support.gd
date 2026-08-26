class_name TestSupport
extends RefCounted

## The wire fixture corpus is one contract shared with `tme-protocol`, which
## asserts the same exact file inventory from the Rust side. The client reads
## that one corpus instead of keeping a second copy, because two copies of a
## contract drift and the drift shows up as a passing test on each side.
##
## It has to be reached as an absolute path. Godot normalises `res://..` away
## inside `DirAccess` while `FileAccess` follows it, so a `res://`-relative path
## silently lists the Godot project directory and the inventory assertion below
## would pass against the wrong directory. Tests never ship — the export presets
## exclude `tests/*` — so a source-tree path is the honest form here.
const WIRE_FIXTURE_RELATIVE_PATH: String = "../tests/fixtures/wire"

var assertion_count: int = 0
var failures: Array[String] = []
var current_test: String = ""


static func wire_fixture_root() -> String:
	return ProjectSettings.globalize_path("res://").path_join(WIRE_FIXTURE_RELATIVE_PATH).simplify_path()


static func wire_fixture_path(decoder: String) -> String:
	return wire_fixture_root().path_join(decoder + ".json")


func run_suite(suite: RefCounted, suite_name: String, selected: PackedStringArray = PackedStringArray()) -> Array[String]:
	suite.set("_support", self)
	var names: Array[String] = []
	for method: Dictionary in suite.get_method_list():
		var method_name: String = method["name"] as String
		if method_name.begins_with("test_") and (selected.is_empty() or method_name in selected):
			names.append(method_name)
	names.sort()
	for method_name: String in names:
		current_test = suite_name + "/" + method_name
		await suite.call(method_name)
	current_test = ""
	var qualified: Array[String] = []
	for method_name: String in names: qualified.append(suite_name + "/" + method_name)
	return qualified


func expect(condition: bool, message: String) -> void:
	assertion_count += 1
	if not condition:
		failures.append(current_test + ": " + message)


func expect_equal(actual: Variant, expected: Variant, message: String) -> void:
	expect(actual == expected, message)


func expect_accept(result: Dictionary, message: String) -> void:
	expect(result.get("ok", false), message + " [" + str(result.get("error", "missing result")) + "]")


func expect_reject(result: Dictionary, message: String) -> void:
	expect(not result.get("ok", false), message)


func test_all_client_scripts_parse() -> void:
	var script_paths: Array[String] = []
	_collect_script_paths("res://", script_paths)
	script_paths.sort()
	expect(not script_paths.is_empty(), "client script inventory must not be empty")
	for script_path: String in script_paths:
		var resource: Resource = load(script_path)
		expect(resource is Script, script_path + " must load as a script resource")


func _collect_script_paths(directory_path: String, script_paths: Array[String]) -> void:
	var directory: DirAccess = DirAccess.open(directory_path)
	if directory == null:
		expect(false, directory_path + " must be readable")
		return
	var directories: PackedStringArray = directory.get_directories()
	directories.sort()
	for directory_name: String in directories:
		var child_path: String = directory_path.path_join(directory_name)
		if child_path in ["res://.godot", "res://build", "res://tests/fixtures"]:
			continue
		_collect_script_paths(child_path, script_paths)
	var files: PackedStringArray = directory.get_files()
	files.sort()
	for file_name: String in files:
		if file_name.ends_with(".gd"):
			script_paths.append(directory_path.path_join(file_name))
