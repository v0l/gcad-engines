@tool
extends EditorPlugin

const Gcad = preload("res://addons/gcad/gcad.gd")

var importer: EditorImportPlugin


func _setting(name: String, value: Variant, info: Dictionary) -> void:
	if not ProjectSettings.has_setting(name):
		ProjectSettings.set_setting(name, value)
	ProjectSettings.set_initial_value(name, value)
	info.name = name
	ProjectSettings.add_property_info(info)


func _enter_tree() -> void:
	importer = preload("res://addons/gcad/gcad_import.gd").new()
	add_import_plugin(importer)
	_setting("gcad/gcad2glb_path", "gcad2glb", {"type": TYPE_STRING, "hint": PROPERTY_HINT_GLOBAL_FILE})
	_setting("gcad/cache_dir", "res://.godot/gcad-cache", {"type": TYPE_STRING, "hint": PROPERTY_HINT_DIR})
	EditorInterface.get_resource_filesystem().filesystem_changed.connect(reimport_stale)


func _exit_tree() -> void:
	EditorInterface.get_resource_filesystem().filesystem_changed.disconnect(reimport_stale)
	remove_import_plugin(importer)
	importer = null


func reimport_stale() -> void:
	var cache_dir := Gcad.cache_dir()
	if cache_dir.is_empty():
		return
	var tool := str(ProjectSettings.get_setting("gcad/gcad2glb_path", "gcad2glb"))
	var stale := Gcad.stale(tool, cache_dir)
	if not stale.is_empty():
		EditorInterface.get_resource_filesystem().reimport_files(stale)
