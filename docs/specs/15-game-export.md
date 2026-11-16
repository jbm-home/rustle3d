# 15 — Game-engine export

Provenance: maintainer's functional description (2026-11-14); Khronos glTF 2.0 specification and extensions (`KHR_materials_sheen`, `KHR_texture_transform`); general real-time asset practice (LODs, normal-map baking).

## User story
As a game or real-time artist, I export the garment so it drops into Godot,
Unity, Unreal or Blender and looks like it did in Rustle3D.

## Status (2026-11-16)

Implemented (M9): `export.gltf` (GLB) and `export.obj`, also `r3d export
--format glb|obj [--avatar]` and the MCP tools. Every template exports
without error or warning in the Khronos glTF Validator
(`scripts/validate-exports.sh`, run by `check.sh` when Node.js is there and
by a CI job). Not yet checked by hand: imports in Blender and Godot.

## Behavior
- **M9**: glTF 2.0 / GLB with pattern UVs (1 UV unit = 1 m of fabric, `v`
  down the pattern), a PBR material with `KHR_materials_sheen` from the
  fabric's colour (linear) and sheen, one named node and mesh per panel
  under a garment node, optional avatar node, metres, Y-up. The draped
  positions are the cached `simulate` result if up to date, otherwise the
  starting ones; the app exports what it shows. OBJ: one object per
  panel, UVs and normals, geometry only.
- **M15, game-ready preset**: decimated LODs, wrinkles baked from the full
  simulation into normal maps, back faces or thickness, and skinning
  weights transferred from the avatar skeleton when avatars are animated.
- Engine presets set scale, axes and texture sizes for each target engine.
- Proprietary formats (FBX) are not supported. OpenUSD may come later.

## Acceptance criteria
- Exports pass the Khronos glTF validator with no errors.
- The game-ready preset of the T-shirt imports into Godot and Blender with
  correct scale, materials and LODs (checked in CI where possible).
