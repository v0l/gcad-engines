@tool
extends EditorImportPlugin

const Gcad = preload("res://addons/gcad/gcad.gd")


func _get_importer_name() -> String:
	return "gcad.scene"


func _get_visible_name() -> String:
	return "gcad scene"


func _get_recognized_extensions() -> PackedStringArray:
	return PackedStringArray(["gcad", "gasm"])


func _get_save_extension() -> String:
	return "scn"


func _get_resource_type() -> String:
	return "PackedScene"


func _get_priority() -> float:
	return 1.0


func _get_import_order() -> int:
	return 0


func _get_preset_count() -> int:
	return 1


func _get_preset_name(_preset: int) -> String:
	return "Default"


func _get_import_options(_path: String, _preset: int) -> Array[Dictionary]:
	return [
		{"name": "height", "default_value": 0.0, "property_hint": PROPERTY_HINT_RANGE, "hint_string": "0,1000,0.01,or_greater"},
		{"name": "scale", "default_value": 0.0, "property_hint": PROPERTY_HINT_RANGE, "hint_string": "0,1000,0.0001,or_greater"},
		{"name": "centre", "default_value": false},
		{"name": "detail", "default_value": 1.0, "property_hint": PROPERTY_HINT_RANGE, "hint_string": "0.1,8,0.1"},
		{"name": "triangles", "default_value": 0, "property_hint": PROPERTY_HINT_RANGE, "hint_string": "0,1000000,1,or_greater"},
	]


func _get_option_visibility(_path: String, _option: StringName, _options: Dictionary) -> bool:
	return true


func _import(source_file: String, save_path: String, options: Dictionary, _platform_variants: Array[String], _gen_files: Array[String]) -> Error:
	var tool := str(ProjectSettings.get_setting("gcad/gcad2glb_path", "gcad2glb"))
	var cache_dir := Gcad.cache_dir()
	var built := Gcad.build(tool, ProjectSettings.globalize_path(source_file), options, cache_dir)
	if built.error != OK:
		push_error("gcad: %s: %s" % [source_file, built.message])
		return built.error
	if not cache_dir.is_empty():
		Gcad.remember_key(cache_dir, source_file, built.key, options)
	print("gcad: %s %s %s" % [source_file, "cached" if built.cached else "built", built.key])
	var packed := PackedScene.new()
	var result := packed.pack(built.scene)
	built.scene.free()
	if result != OK:
		return result
	return ResourceSaver.save(packed, "%s.%s" % [save_path, _get_save_extension()])
