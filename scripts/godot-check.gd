# Opens game exports (spec 15) in Godot 4 as a game does and checks what
# it finds against what the export reported: a skeleton of our bones,
# every mesh skinned to it, one mesh per level of detail (and the avatar),
# and the animations by name, each moving the bones.
#
# Run by scripts/godot-check.sh:
#   godot --headless --path <project> --script res://godot-check.gd -- a.glb ...
# Each `a.glb` has `a.glb.json` beside it: the export's result (`lods`,
# `skinned`, `animations`) plus `avatar` (the body was exported too).
extends SceneTree


func _init() -> void:
	var failed := false
	for file in OS.get_cmdline_user_args():
		var problems := check("res://" + file)
		if problems.is_empty():
			print("ok ", file)
		else:
			failed = true
			for p in problems:
				printerr(file, ": ", p)
	quit(1 if failed else 0)


func check(path: String) -> Array[String]:
	var problems: Array[String] = []
	var expected: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(path + ".json"))
	var scene: PackedScene = load(path)
	if scene == null:
		return ["Godot cannot load it"]
	var root := scene.instantiate()
	var meshes := root.find_children("*", "MeshInstance3D", true, false)
	var lods: int = expected["lods"].size()
	var wanted := lods + (1 if expected.get("avatar", false) else 0)
	if meshes.size() != wanted:
		problems.append("%d meshes, %d expected" % [meshes.size(), wanted])
	if expected["skinned"]:
		var skeletons := root.find_children("*", "Skeleton3D", true, false)
		if skeletons.size() != 1:
			problems.append("%d skeletons, one expected" % skeletons.size())
		else:
			var skeleton: Skeleton3D = skeletons[0]
			if skeleton.get_bone_count() < 24:
				problems.append("%d bones" % skeleton.get_bone_count())
			for m in meshes:
				var mesh: MeshInstance3D = m
				if mesh.skin == null or mesh.get_node_or_null(mesh.skeleton) != skeleton:
					problems.append("%s is not skinned to the skeleton" % mesh.name)
		var players := root.find_children("*", "AnimationPlayer", true, false)
		var names: Array = []
		for p in players:
			for n in (p as AnimationPlayer).get_animation_list():
				var animation := (p as AnimationPlayer).get_animation(n)
				if animation.get_track_count() == 0 or animation.length <= 0.0:
					problems.append("animation %s moves nothing" % n)
				names.append(n)
		for n in expected["animations"]:
			if not names.has(n):
				problems.append("animation %s missing (found %s)" % [n, names])
	root.free()
	return problems
