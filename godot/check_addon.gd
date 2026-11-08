extends SceneTree

const Gcad = preload("res://addons/gcad/gcad.gd")

var errors: Array[String] = []
var report := {}


func count(node: Node, into: Dictionary) -> void:
	if node is MeshInstance3D:
		into.meshes += 1
		if node.skin != null:
			into.skinned += 1
	if node is AnimationPlayer:
		into.clips.append_array(node.get_animation_list())
	for child in node.get_children():
		count(child, into)


func bounds(node: Node, at: Transform3D, box: Array) -> void:
	var here: Transform3D = at * node.transform if node is Node3D else at
	if node is MeshInstance3D and node.mesh != null:
		var aabb: AABB = here * node.mesh.get_aabb()
		box[0] = aabb if box[0] == null else box[0].merge(aabb)
	for child in node.get_children():
		bounds(child, here, box)


func check(file: String, options: Dictionary, expect: Dictionary) -> void:
	var tool := OS.get_environment("GCAD2GLB")
	var built := Gcad.build(tool, ProjectSettings.globalize_path("res://" + file), options)
	if built.error != OK:
		errors.append("%s: %s" % [file, built.message])
		return
	var scene: Node3D = built.scene
	root.add_child(scene)
	var got := {"meshes": 0, "skinned": 0, "clips": []}
	count(scene, got)
	var box := [null]
	bounds(scene, Transform3D.IDENTITY, box)
	got.height = box[0].size.y if box[0] != null else 0.0
	got.clips.sort()
	report[file] = got
	for key in expect:
		var want = expect[key]
		var have = got[key]
		var ok: bool = abs(have - want) < 1e-3 if key == "height" else have == want
		if not ok:
			errors.append("%s: %s is %s, expected %s" % [file, key, have, want])
	scene.queue_free()


func cached_twice() -> void:
	var tool := OS.get_environment("GCAD2GLB")
	var cache := ProjectSettings.globalize_path("res://cache-check")
	var source := ProjectSettings.globalize_path("res://included.gcad")
	var first := Gcad.build(tool, source, {"height": 1.0}, cache)
	var second := Gcad.build(tool, source, {"height": 1.0}, cache)
	var other := Gcad.build(tool, source, {"height": 2.0}, cache)
	var part := FileAccess.open("res://included_body.gcad", FileAccess.WRITE)
	part.store_string("blob 0,0,0 r=2,2,4\nremesh 0.6\n")
	part.close()
	var edited := Gcad.build(tool, source, {"height": 1.0}, cache)
	for built in [first, second, other, edited]:
		if built.error != OK:
			errors.append("included.gcad: %s" % built.message)
			return
		built.scene.free()
	report.cache = {"first": first.cached, "second": second.cached, "other_height": other.cached, "include_edited": edited.cached}
	if first.cached or not second.cached:
		errors.append("the second build was not a cache hit: %s" % report.cache)
	if other.cached or edited.cached or edited.key == first.key:
		errors.append("a settings change or an include edit was served from the cache: %s" % report.cache)
	if Gcad.key(tool, source, {"height": 1.0}) != edited.key:
		errors.append("gcad2glb key does not match the cached file")


func _initialize() -> void:
	check("rigged_creature.gcad", {"height": 1.0}, {"skinned": 1, "clips": ["idle", "walk"], "height": 1.0})
	check("creature.gcad", {}, {"skinned": 0, "clips": []})
	check("bracket.gcad", {"height": 2.0, "centre": true}, {"height": 2.0})
	cached_twice()
	var bad := Gcad.build(OS.get_environment("GCAD2GLB"), ProjectSettings.globalize_path("res://broken.gcad"), {})
	if bad.error == OK or not "frobnicate" in bad.message:
		errors.append("broken.gcad was not refused with its line: %s" % bad.message)
	report.errors = errors
	report.passed = errors.is_empty()
	var out := FileAccess.open("res://addon-report.json", FileAccess.WRITE)
	out.store_string(JSON.stringify(report, "  "))
	out.close()
	quit(0 if errors.is_empty() else 1)
