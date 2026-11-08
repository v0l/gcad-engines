@tool
extends RefCounted


static func flags(options: Dictionary) -> PackedStringArray:
	var args := PackedStringArray()
	if float(options.get("height", 0.0)) > 0.0:
		args.append_array(["--height", str(options.height)])
	elif float(options.get("scale", 0.0)) > 0.0:
		args.append_array(["--scale", str(options.scale)])
	if bool(options.get("centre", false)):
		args.append("--centre")
	if float(options.get("detail", 1.0)) != 1.0:
		args.append_array(["--detail", str(options.detail)])
	if int(options.get("triangles", 0)) >= 4:
		args.append_array(["--tris", str(options.triangles)])
	return args


static func arguments(source: String, glb: String, options: Dictionary) -> PackedStringArray:
	var args := flags(options)
	args.append_array([source, glb])
	return args


static func cache_dir() -> String:
	var setting := str(ProjectSettings.get_setting("gcad/cache_dir", "res://.godot/gcad-cache"))
	return "" if setting.is_empty() else ProjectSettings.globalize_path(setting)


static func key(tool: String, source: String, options: Dictionary) -> String:
	var args := PackedStringArray(["key"])
	args.append_array(flags(options))
	args.append(source)
	var output: Array = []
	if OS.execute(tool, args, output, false) != 0:
		return ""
	return "".join(output).strip_edges()


static func _cached(tool: String, source: String, options: Dictionary, cache_dir: String) -> Dictionary:
	DirAccess.make_dir_recursive_absolute(cache_dir)
	var args := flags(options)
	args.append_array(["--cache", cache_dir, source])
	var output: Array = []
	var code := OS.execute(tool, args, output, true)
	var text := "".join(output)
	var glb := ""
	var cached := false
	for line in text.split("\n"):
		if line.begins_with("cached "):
			cached = true
			glb = line.substr(7).strip_edges()
		elif line.begins_with("built "):
			glb = line.substr(6).strip_edges()
	if code != 0 or glb.is_empty() or not FileAccess.file_exists(glb):
		return {"error": ERR_COMPILATION_FAILED, "message": "%s exited %d: %s" % [tool, code, text.strip_edges()]}
	return {"error": OK, "glb": glb, "cached": cached, "key": glb.get_file().get_basename()}


static func _uncached(tool: String, source: String, options: Dictionary) -> Dictionary:
	var dir := OS.get_cache_dir().path_join("gcad-godot")
	DirAccess.make_dir_recursive_absolute(dir)
	var glb := dir.path_join("%s-%d.glb" % [source.get_file().get_basename(), Time.get_ticks_usec()])
	var output: Array = []
	var code := OS.execute(tool, arguments(source, glb, options), output, true)
	if code != 0 or not FileAccess.file_exists(glb):
		return {"error": ERR_COMPILATION_FAILED, "message": "%s exited %d: %s" % [tool, code, "".join(output).strip_edges()]}
	return {"error": OK, "glb": glb, "cached": false, "key": "", "temporary": true}


static func build(tool: String, source: String, options: Dictionary, cache_dir: String = "") -> Dictionary:
	var made := _uncached(tool, source, options) if cache_dir.is_empty() else _cached(tool, source, options, cache_dir)
	if made.error != OK:
		return made
	var document := GLTFDocument.new()
	var state := GLTFState.new()
	var parsed := document.append_from_file(made.glb, state)
	if made.get("temporary", false):
		DirAccess.remove_absolute(made.glb)
	if parsed != OK:
		return {"error": parsed, "message": "the GLB gcad2glb wrote did not parse"}
	var scene := document.generate_scene(state)
	if scene == null:
		return {"error": ERR_CANT_CREATE, "message": "no scene in the GLB"}
	scene.name = source.get_file().get_basename()
	scene.set_meta("gcad_key", made.key)
	_own(scene, scene)
	return {"error": OK, "scene": scene, "message": "", "cached": made.cached, "key": made.key}


static func keys_file(cache_dir: String) -> String:
	return cache_dir.path_join("imported.json")


static func imported_keys(cache_dir: String) -> Dictionary:
	var text := FileAccess.get_file_as_string(keys_file(cache_dir))
	var parsed = JSON.parse_string(text) if not text.is_empty() else {}
	return parsed if parsed is Dictionary else {}


static func remember_key(cache_dir: String, source: String, value: String, options: Dictionary) -> void:
	var keys := imported_keys(cache_dir)
	keys[source] = {"key": value, "options": options}
	DirAccess.make_dir_recursive_absolute(cache_dir)
	var out := FileAccess.open(keys_file(cache_dir), FileAccess.WRITE)
	if out != null:
		out.store_string(JSON.stringify(keys, "  "))
		out.close()


static func stale(tool: String, cache_dir: String) -> PackedStringArray:
	var out := PackedStringArray()
	var keys := imported_keys(cache_dir)
	for source in keys:
		var entry: Dictionary = keys[source]
		var path := ProjectSettings.globalize_path(source)
		if not FileAccess.file_exists(path):
			continue
		var now := key(tool, path, entry.get("options", {}))
		if not now.is_empty() and now != str(entry.get("key", "")):
			out.append(source)
	return out


static func _own(node: Node, owner: Node) -> void:
	for child in node.get_children():
		child.owner = owner
		_own(child, owner)
