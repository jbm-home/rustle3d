# 15 — Game-engine export

Provenance: maintainer's functional description (2026-11-14); Khronos glTF 2.0 specification and extensions (`KHR_materials_sheen`, `KHR_texture_transform`); general real-time asset practice (LODs, normal-map baking).

## User story
As a game or real-time artist, I export the garment so it drops into Godot,
Unity, Unreal or Blender and looks like it did in Rustle3D.

## Behavior
- **M9**: glTF 2.0 / GLB with pattern UVs (1 UV unit = 1 m of fabric), PBR
  materials with sheen, one primitive per panel, optional avatar, metres,
  Y-up.
- **M14, game-ready preset**: decimated LODs, wrinkles baked from the full
  simulation into normal maps, back faces or thickness, and skinning
  weights transferred from the avatar skeleton when avatars are animated.
- Engine presets set scale, axes and texture sizes for each target engine.
- Proprietary formats (FBX) are not supported. OpenUSD may come later.

## Acceptance criteria
- Exports pass the Khronos glTF validator with no errors.
- The game-ready preset of the T-shirt imports into Godot and Blender with
  correct scale, materials and LODs (checked in CI where possible).
