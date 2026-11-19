# 15 — Game-engine export

Provenance: maintainer's functional description (2026-11-14; ROADMAP M15, 2026-11-19: LODs, baked wrinkle normal maps, back faces or thickness, skinning to the avatar skeleton with the procedural motions, presets for Godot, Unity and Unreal); Khronos glTF 2.0 specification (skins, animations, `TANGENT`, `normalTexture`, `KHR_materials_sheen`); public papers: Garland & Heckbert, *Surface Simplification Using Quadric Error Metrics*, SIGGRAPH 1997 (LODs); Cohen, Olano & Manocha, *Appearance-Preserving Simplification*, SIGGRAPH 1998 (normal maps through a shared parameterization); Mikkelsen, *Simulation of Wrinkled Surfaces Revisited*, 2008 (tangent frames); Magnenat-Thalmann et al. 1988 (linear blend skinning, spec 22); Abdrashitov et al., *Robust Skin Weights Transfer via Weight Inpainting*, SIGGRAPH Asia 2023 Technical Communications (closest-point weight transfer); general real-time asset practice (LOD naming, texture sizes, atlas padding) and general knowledge of how Godot, Unity and Unreal import glTF (no engine was run).

## User story
As a game or real-time artist, I export the garment so it drops into Godot,
Unity, Unreal or Blender and looks like it did in Rustle3D: lighter
versions for far away, the wrinkles kept in a normal map, and the garment
moving with the character's skeleton.

## Status (2026-11-19, M15)

Implemented (M9): `export.gltf` (GLB) and `export.obj`, also `r3d export
--format glb|obj [--avatar]` and the MCP tools. Every template exports
without error or warning in the Khronos glTF Validator
(`scripts/validate-exports.sh`, run by `check.sh` when Node.js is there and
by a CI job).

Implemented (M15): `export.game`, the game-ready GLB: LODs, a baked
tangent-space normal map per lower LOD, a base-colour atlas for prints,
double-sided cloth or a thin shell, skinning to the avatar's skeleton
with the three motions as animations, and presets for Godot, Unity and
Unreal; `r3d export --format game`, the MCP tool `export_game`, and the
app's Export menu (Garment for a game engine…). Checked by the glTF validator in
`check.sh`.

Not yet: imports checked by hand in the engines (the presets follow
their documented importers); the `MSFT_lod` extension (LODs are separate
nodes, named for the engines' LOD conventions); skinning of imported
bodies (they have no skeleton, spec 27); dual-quaternion skinning;
roughness or sheen maps; OpenUSD.

## Behavior

### The plain 3D export (M9)
glTF 2.0 / GLB with pattern UVs (1 UV unit = 1 m of fabric, `v` down the
pattern, or repeats of the print), a PBR material with
`KHR_materials_sheen` from the fabric's colour (linear) and sheen, one
named node and mesh per panel under a garment node, optional avatar node,
metres, Y-up. The draped positions are the cached `simulate` result if up
to date, otherwise the starting ones; the app exports what it shows. OBJ:
one object per panel, UVs and normals, geometry only.

### The game-ready export (M15, `export.game`)
- **One mesh per garment and LOD.** The garment's pieces are merged into
  one mesh with one material, as engines draw it in one call. Its texture
  coordinates are an **atlas**: every piece laid flat as on the pattern
  (same scale for all, along its grainline, never turned), packed into
  the unit square with a gap between pieces, `v` down the image.
- **LODs.** A list of ratios of the full triangle count, the first 1
  (LOD0 is the draped mesh itself), each smaller than the one before,
  at most five, each above 0 (default 1, 0.5, 0.2). Lower LODs are
  simplified by quadric error metrics (Garland & Heckbert 1997) with
  half-edge collapses onto existing vertices, so every LOD keeps the
  vertices' texture coordinates and skin weights. The pieces' edges are
  kept exactly (seams stay closed, the hem and the neckline keep their
  shape); a collapse that would flip a triangle in 3D or in the atlas, or
  pinch the surface, is refused, so a LOD can stay above its ratio (the
  result says how many triangles each has).
- **Normal map.** For each LOD below LOD0, the full drape's shading is
  baked into a tangent-space normal map of the chosen size (0 = none;
  256 to 4096, a power of two; default 1024): both meshes share the
  atlas, so a texel's full-resolution normal is read at the same atlas
  point (Cohen, Olano & Manocha 1998), and written in the low mesh's
  tangent frame. Tangents are exported (`TANGENT`, `w` the handedness)
  and the map follows glTF's convention (green towards `−v`, the image's
  up). Texels outside the pieces are filled from their neighbours
  (padding) so mipmaps do not bleed. Deterministic.
- **Prints.** A garment with a print gets its print baked into a
  base-colour image in the atlas (same size as the normal map, 1024 when
  there is none); without a print the fabric's colour is the base colour
  factor. The material keeps `KHR_materials_sheen`.
- **Back faces or thickness** (`sides`): `double_sided` (default), the
  material drawn from both sides, or `shell`: an inner surface offset
  inwards by `thickness_mm` (default 1 mm, 0.2–20) facing the other way
  and a rim joining it to the outer one along the pieces' edges, with a
  one-sided material (for engines or shaders that cull back faces).
- **Skinning** (`skin`, default on): the avatar's 24-bone skeleton (spec
  22) as joint nodes in its rest pose, the garment's skin weights
  transferred from the body: each garment vertex takes the weights at the
  closest point of the body's surface (interpolated from that triangle's
  corners), smoothed over the garment's mesh (a few Laplacian passes) and
  kept to the four heaviest bones, normalized. The garment is exported in
  the body's rest pose: a drape that ended while the avatar moved is
  carried back to the rest pose by inverting each vertex's blended bone
  motion. With `animations` (default on), the three motions (`walk`,
  `turn`, `raise_arms`, `idle`) are baked as glTF animations, one cycle each at
  30 frames a second, at full amplitude, looping. With `avatar`, the body
  is exported too, skinned with its own weights. An imported body has no
  skeleton: skinning is refused with `export.no_skeleton` (export with
  `skin: false`); a project without an avatar too.
- **Presets** (`preset`): `generic`, `godot`, `unity`, `unreal`. Every
  file stays a valid glTF (metres, Y up, +Z forward): all three engines'
  importers convert units and axes themselves, so a preset never scales
  or turns the scene. A preset sets the defaults of the other options and
  the names:

  | Preset  | LODs                    | Maps | Names |
  |---------|-------------------------|------|-------|
  | generic | 1, 0.5, 0.2             | 1024 | `Shirt_LOD1`, material `Shirt_LOD1`, image `Shirt_normal_LOD1` |
  | godot   | 1, 0.5, 0.2             | 2048 | as generic |
  | unity   | 1, 0.5, 0.2             | 2048 | as generic (Unity's `_LODn` suffix convention) |
  | unreal  | 1, 0.5, 0.25, 0.125     | 2048 | `SK_Shirt_LOD1`, `M_Shirt_LOD1`, `T_Shirt_N_LOD1`, `T_Shirt_D` |

  Options given explicitly override the preset's.
- The drape exported is the cached `simulate` result if up to date
  (otherwise the starting positions), as for `export.gltf`; the app
  exports the drape it shows.
- Proprietary formats (FBX) are not supported. OpenUSD may come later.

### Importing (documented in the user guide)
- **Godot 4**: drop the `.glb` into the project; the scene has the
  skeleton, one `MeshInstance3D` per LOD (show one, or use visibility
  ranges), and an `AnimationPlayer` with the three motions.
- **Unity**: with the glTFast package (open source, Apache-2.0), drop the
  `.glb` into Assets; put the `_LODn` meshes in a LOD Group.
- **Unreal Engine 5**: Import (Interchange glTF) as a skeletal mesh with
  its animations; units become centimetres and Z up on import.
- **Blender**: File › Import › glTF 2.0.

## UX
- Export menu › **Garment for a game engine (glTF)…** opens a dialog: preset (Generic,
  Godot, Unity, Unreal), LODs (a field such as `100, 50, 20` in %),
  normal map size (none, 512, 1024, 2048, 4096), back faces (double-sided
  or thickness, with the thickness in mm), skin to the avatar's skeleton
  (with its motions), include the avatar. Choosing a preset fills the
  fields. Export asks where to save the `.glb`. The skin switch is off
  and explained for an imported body.

## Acceptance criteria
- Exports pass the Khronos glTF validator with no errors
  (`scripts/validate-exports.sh` exports the T-shirt game-ready, skinned
  with its animations, with a print and as a shell).
- LODs: the T-shirt's LOD1 and LOD2 have at most about 55 % and 30 % of
  its triangles, keep every piece's edge vertices, and no triangle turns
  over in 3D or in the atlas.
- The normal map of a flat sheet is flat (128, 128, 255); a wrinkled
  sheet's LOD bakes a map that differs from flat where the wrinkles are;
  two exports of the same project are byte-identical.
- Skin weights sum to 1 on every vertex; a sleeve vertex follows the arm
  bones; skinning the exported garment with a motion's pose moves it with
  the body.
- An imported body and skinning give `export.no_skeleton`.
