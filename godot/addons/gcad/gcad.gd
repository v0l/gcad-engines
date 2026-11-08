@tool
extends RefCounted


static func arguments(source: String, glb: String, options: Dictionary) -> PackedStringArray:
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
	args.append_array([source, glb])
	return args


static func build(tool: String, source: String, options: Dictionary) -> Dictionary:
	var dir := OS.get_cache_dir().path_join("gcad-godot")
	DirAccess.make_dir_recursive_absolute(dir)
	var glb := dir.path_join("%s-%d.glb" % [source.get_file().get_basename(), Time.get_ticks_usec()])
	var output: Array = []
	var code := OS.execute(tool, arguments(source, glb, options), output, true)
	if code != 0 or not FileAccess.file_exists(glb):
		return {"error": ERR_COMPILATION_FAILED, "message": "%s exited %d: %s" % [tool, code, "".join(output).strip_edges()]}
	var document := GLTFDocument.new()
	var state := GLTFState.new()
	var parsed := document.append_from_file(glb, state)
	DirAccess.remove_absolute(glb)
	if parsed != OK:
		return {"error": parsed, "message": "the GLB gcad2glb wrote did not parse"}
	var scene := document.generate_scene(state)
	if scene == null:
		return {"error": ERR_CANT_CREATE, "message": "no scene in the GLB"}
	scene.name = source.get_file().get_basename()
	_own(scene, scene)
	return {"error": OK, "scene": scene, "message": ""}


static func _own(node: Node, owner: Node) -> void:
	for child in node.get_children():
		child.owner = owner
		_own(child, owner)
