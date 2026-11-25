# 15 — Game-engine export

Provenance: maintainer's functional description (2026-11-14; ROADMAP M15, 2026-11-19: LODs, baked wrinkle normal maps, back faces or thickness, skinning to the avatar skeleton with the procedural motions, presets for Godot, Unity and Unreal); Khronos glTF 2.0 specification (skins, animations, `TANGENT`, `normalTexture`, `KHR_materials_sheen`); public papers: Garland & Heckbert, *Surface Simplification Using Quadric Error Metrics*, SIGGRAPH 1997 (LODs); Cohen, Olano & Manocha, *Appearance-Preserving Simplification*, SIGGRAPH 1998 (normal maps through a shared parameterization); Mikkelsen, *Simulation of Wrinkled Surfaces Revisited*, 2008 (tangent frames); Magnenat-Thalmann et al. 1988 (linear blend skinning, spec 22); Abdrashitov et al., *Robust Skin Weights Transfer via Weight Inpainting*, SIGGRAPH Asia 2023 Technical Communications (closest-point weight transfer); general real-time asset practice (LOD naming, texture sizes, atlas padding) and general knowledge of how Godot, Unity and Unreal import glTF; maintainer's brief 2026-11-23 (ROADMAP M37: the game export carries the motion, built-in and captured, skins an imported body, and is opened in game engines, every problem found fixed, the inside of a shell showing the outside's wrinkles reversed among them; Godot checked by the tests, the others by the maintainer with a written check list); Godot 4 (MIT), run headless by a script to open the exports.

## User story
As a game or real-time artist, I export the garment so it drops into Godot,
Unity, Unreal or Blender and looks like it did in Rustle3D: lighter
versions for far away, the wrinkles kept in a normal map, and the garment
moving with the character's skeleton.

## Status (2026-11-25, M37)

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

2026-11-25 (M37): an imported body is skinned (to our skeleton placed
on it, spec 27), its joints and the garment in the pose it stands in;
the project's captured motions are animations besides the built-in
ones; the inside of a shell shows the outside's relief the right way
round; every export kind is opened in Godot by `scripts/godot-check.sh`
(a CI job); a check list for Unity, Unreal and Blender, by hand.

Not yet: the `MSFT_lod` extension (LODs are separate nodes, named for
the engines' LOD conventions); dual-quaternion skinning; roughness or
sheen maps; OpenUSD.

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
  one-sided material (for engines or shaders that cull back faces). The
  inner surface shares the outer's atlas and normal map; its tangents
  point the other way, so a texel gives it the opposite of the outer
  normal: from inside, a fold shows its other side (tangents found on
  its own turned-over triangles lit a fold's slope as from outside,
  the relief reversed).
- **Skinning** (`skin`, default on): the avatar's 24-bone skeleton (spec
  22) as joint nodes in the pose the body stands in (the built-in body's
  rest pose, without rotations; an imported body's stance, spec 27, each
  joint turned as its bone, its inverse bind matrix the inverse of its
  place), the garment's skin weights
  transferred from the body: each garment vertex takes the weights at the
  closest point of the body's surface (interpolated from that triangle's
  corners), smoothed over the garment's mesh (a few Laplacian passes) and
  kept to the four heaviest bones, normalized. The garment is exported
  where the body stands: a drape that ended while the avatar moved is
  carried back by inverting each vertex's blended bone motion. With
  `animations` (default on), the built-in motions (`walk`, `turn`,
  `raise_arms`, `idle`) and the captured motions of the project's
  `motions/` folder (named after their files, a name taken by another
  numbered: `walk (2)`) are baked as glTF animations, one cycle each at
  30 frames a second, at full amplitude, looping. With `avatar`, the body
  is exported too, skinned with its own weights. A project without an
  avatar cannot be skinned (`export.no_skeleton`: export with `skin:
  false`).
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
  ranges), and an `AnimationPlayer` with the motions.
- **Unity**: with the glTFast package (open source, Apache-2.0), drop the
  `.glb` into Assets; put the `_LODn` meshes in a LOD Group.
- **Unreal Engine 5**: Import (Interchange glTF) as a skeletal mesh with
  its animations; units become centimetres and Z up on import.
- **Blender**: File › Import › glTF 2.0.

### Checked in the engines
- **Godot** (by the tests): `scripts/godot-check.sh` exports a T-shirt
  skinned to the built-in body with the avatar, its motions and a
  captured one; the same T-shirt as an Unreal shell without skin; a
  dress on an imported body (the built-in body as an OBJ, without a
  skeleton). It imports them in a headless Godot 4 (an import error
  fails it) and opens each as a game does (`scripts/godot-check.gd`): one
  skeleton of at least 24 bones, every mesh skinned to it, one mesh per
  LOD and the avatar, every animation reported present and moving bones.
  Run by a CI job (Godot downloaded once into `target/godot`), and by
  `check.sh` when Godot is installed (`godot` on the `PATH` or `GODOT`).
- **Unity, Unreal, Blender** (by hand, on the same three files, exported
  from the app or with the commands of the script): for each,
  1. the file imports without an error or a warning about the skin or
     the animations;
  2. the garment stands on the avatar as in Rustle3D (not lying down,
     not mirrored, not 100 times too big or small), the imported body as
     it stands in its file;
  3. each animation plays: the avatar walks, turns, raises its arms,
     stands at ease, plays the captured motion; the garment moves with
     it without coming apart at the seams or sinking into the body;
  4. the levels of detail switch (Unity: the `_LOD` meshes in a LOD
     Group; Unreal: the LODs of the skeletal mesh) and keep their
     wrinkles in the normal map;
  5. the shell, seen from inside (the camera in the sleeve), shows its
     folds lit the right way and no hole along its edges;
  6. the print sits on the pieces as in Rustle3D.

## UX
- Export menu › **Garment for a game engine (glTF)…** opens a dialog: preset (Generic,
  Godot, Unity, Unreal), LODs (a field such as `100, 50, 20` in %),
  normal map size (none, 512, 1024, 2048, 4096), back faces (double-sided
  or thickness, with the thickness in mm), skin to the avatar's skeleton
  (with its motions), include the avatar. Choosing a preset fills the
  fields. Export asks where to save the `.glb`.

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
- A project without an avatar and skinning give `export.no_skeleton`.
- An imported body (the built-in body as an OBJ) is skinned with its
  avatar and the four motions; each joint's place in the scene (its
  node's transform composed up the hierarchy) is the inverse of its bind
  matrix, some joints turned. A captured motion of the project named as
  a built-in one is exported as `walk (2)`.
- The inner surface of a shell decodes each texel of the normal map to
  the opposite of the outer surface's normal there.
- `scripts/godot-check.sh` passes (CI).
