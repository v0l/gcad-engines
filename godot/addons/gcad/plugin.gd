@tool
extends EditorPlugin

var importer: EditorImportPlugin


func _enter_tree() -> void:
	importer = preload("res://addons/gcad/gcad_import.gd").new()
	add_import_plugin(importer)
	if not ProjectSettings.has_setting("gcad/gcad2glb_path"):
		ProjectSettings.set_setting("gcad/gcad2glb_path", "gcad2glb")
	ProjectSettings.set_initial_value("gcad/gcad2glb_path", "gcad2glb")
	ProjectSettings.add_property_info({"name": "gcad/gcad2glb_path", "type": TYPE_STRING, "hint": PROPERTY_HINT_GLOBAL_FILE})


func _exit_tree() -> void:
	remove_import_plugin(importer)
	importer = null
