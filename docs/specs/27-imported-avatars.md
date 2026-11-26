# 27 — Imported avatars

Provenance: maintainer's functional brief (ROADMAP M31: import a body in OBJ, glTF or GLB, the user owning its licence; check the collision mesh before draping; arrangement points placed with help and checked; motions only with a usable skeleton); open standards (Wavefront OBJ, glTF 2.0; PLY and STL, for body scans: maintainer's brief 2026-11-26, ROADMAP M40); public papers: generalized winding numbers (Jacobson, Kavan & Sorkine-Hornung 2013) and their fast hierarchical evaluation (Barill, Dickson, Schmidt, Levin & Jacobson 2018) for the sign of the distance field of open meshes, the solid angle of a triangle (Van Oosterom & Strackee 1983), angle-weighted pseudo-normals (Bærentzen & Aanæs 2005, already used for the built-in body); general tailoring knowledge (girths taken as a tape around the body, body proportions as fractions of stature); motions (maintainer's brief 2026-11-23, ROADMAP M37: an imported body moves, its own skeleton mapped onto ours when it has one, otherwise our skeleton fitted to it and skin weights computed): the glTF 2.0 specification (skins, inverse bind matrices, `JOINTS_0`/`WEIGHTS_0`), closest-point skin weight transfer (Abdrashitov et al., SIGGRAPH Asia 2023 Technical Communications, as in spec 15), our own placement of the skeleton.

## Status (2026-11-26, M40)

Implemented: import of OBJ, glTF and GLB bodies into the project's
`avatars/` folder (schema v18); detection of the units, the up axis and
the facing, with corrections by a command; the check of the collision
mesh as a report (agents and the app); a distance field that tolerates
holes; arrangement points guessed from the body's sections, shown and
moved in the app, checked before draping; measurements estimated from
the body; `r3d avatar import` / `r3d avatar check`, MCP tools from the
registry; the app's Body menu (Import body…, Check body…, Points).

2026-11-25 (M37): imported bodies move. Our skeleton is placed on the
body, by the file's own skeleton when its joints are found by their
names, otherwise by the arrangement points; the skin follows the file's
weights, or weights carried from the built-in body; every motion
(built-in and captured) plays on it, on the CPU and the GPU, and the
game export skins it.

2026-11-26 (M40): bodies in PLY and STL, the formats of most body
scanners.

Not yet: an imported body in other sizes than its own (every size
drapes on the same body), automatic fit of the arrangement points as the
garment drapes, a disk cache of the distance field of large scans, the
file's own extra bones (fingers, twist bones) moving (they follow the
bone of ours they hang from), hands and feet turned as in the file (they
keep the forearm's and the shin's turn: a few centimetres at the
fingertips).

## User story
As a designer, I drape on my client's body scan, or on the character of
a game, rather than on a body built from four measurements: I import the
file, the app tells me whether it can be draped on and fixes what it
can, I check where the arms and legs are, and I drape a shirt on it.

## Behavior

### Import
- Formats: **OBJ** (vertices and faces; polygons are split into
  triangles; texture coordinates, normals, groups and materials are
  ignored) and **glTF 2.0**, as `.glb` or as `.gltf` with its buffers
  embedded (`data:` URIs). Every triangle primitive of the default scene
  is read, with its nodes' transforms. Files whose buffers are separate
  `.bin` files, or compressed with an extension (Draco, meshopt), are
  refused with a message saying so (save as GLB without compression).
  **PLY**, ASCII or binary (either byte order): the `x`, `y`, `z` of the
  `vertex` element and the `vertex_indices` (or `vertex_index`) lists of
  the `face` element, polygons split into triangles; other elements and
  properties (colours, normals) are ignored; a point cloud without faces
  is refused (`model.empty`). **STL**, binary or ASCII (a binary file is
  told by its size, even when its header starts with `solid`): corners
  with the same coordinates are welded into one vertex, so a closed scan
  checks as closed; facet normals are ignored. FBX is not read: it is a
  closed format (convert it to GLB, user guide).
- The file is copied into the project's `avatars/` folder and referenced
  by its relative path, with a fingerprint of its content (FNV-1a 64):
  the drape's cache follows the file, and a file changed by hand is
  reported (`file.unavailable`) rather than draped silently. The user owns
  the file's licence; nothing of it leaves the project folder.
- The imported body replaces the built-in one for draping, the 3D view,
  photos, exports with the avatar and the fit maps. The avatar keeps
  measurements (`body`): estimated from the imported body (below), they
  size templates and formulas. `avatar.set` changes them and keeps the
  imported body; `avatar.remove_mesh` goes back to the built-in body
  with the same measurements.
- Above 2 000 000 triangles a file is refused (`mesh.too_large`).

### Units, up axis and facing
Stored with the body (`scale`, `up`, `turn_deg`), detected on import
unless given, corrected with `avatar.orient`:
- **Up axis** (`y`, `z`, `x`, `-y`, `-z`, `-x`): glTF is Y-up by its
  specification; for OBJ, the longest side of the bounding box. Its end
  with the larger footprint (two feet against a head) is the floor.
- **Scale** (metres per file unit): the first of metres, centimetres,
  millimetres and inches that makes the height 1.0–2.3 m; otherwise the
  one closest to 1.7 m, with a warning.
- **Facing** (`turn_deg`, a turn about the vertical, right-handed: +90
  turns a body facing −X to face +Z): the body must face +Z. The line through the
  shoulders (the main direction of the section at 80 % of the height)
  is left–right; the feet, which point forward of the ankles, choose
  between the two directions across it. Within 10° of a quarter turn
  the turn is snapped to it.
- The body then stands on the floor (its lowest point at height 0), its
  vertical axis through the middle of its left–right extent and of the
  hips' front–back extent.

### The check (`avatar.check`)
A report of items, each with a stable `code`, a `severity` (`error`:
cannot drape; `warning`: drapes, but check it; `info`: fixed or noted)
and arguments. Errors block `simulate`; warnings are returned with its
result and shown when the drape starts in the app.
- `file.unavailable` (error): the file is not in the project folder, or
  differs from the one imported (import it again).
- `mesh.empty` (error), `mesh.too_large` (error), `mesh.heavy`
  (warning, more than 300 000 triangles: the field and views get slow),
  `mesh.coarse` (warning, fewer than 2 000).
- `mesh.open` (warning, `holes`, `edges`): boundary loops. The field
  is then signed by winding numbers near them (below).
- `mesh.non_manifold` (warning, `edges`): edges shared by more than two
  triangles.
- `mesh.pieces` (warning, `pieces`): separate parts (hair, a base, an
  accessory) besides the largest.
- `mesh.reoriented` (info, `triangles`): triangles turned so that all
  face the same way as their neighbours; `mesh.inside_out` (info): the
  whole surface faced inward and was turned; `mesh.degenerate` (info):
  triangles without area, ignored.
- `scale.height` (warning, `height_m`): the height is outside
  1.2–2.2 m.
- `orientation.up` (warning): the body is wider or deeper than tall —
  the up axis is likely wrong. `orientation.facing` (warning): the feet
  point backwards.
- `points.outside` (warning, `point`): an arrangement point lies more
  than 5 mm outside the body. `points.side` (warning, `point`): a left point is on the
  right of the body or the reverse.
- `measurements.clamped` (warning, `measurement`): an estimated
  measurement was outside the range `avatar.set` accepts and was brought
  into it.
- `motion.rig` (info, `source`): what places the skeleton the body
  moves with: `file` (the file's own skeleton) or `points` (the
  arrangement points).
The report also gives the triangle and vertex counts, the height, the
orientation and the arrangement points with their distance to the
surface (negative inside).

### Distance field
The drape collides with the imported body through the same narrow-band
distance field as the built-in one (docs/physics.md §5), on the CPU and
the GPU. Its sign comes from angle-weighted pseudo-normals, exact on a
closed manifold mesh. On an open or non-manifold mesh, nodes whose
closest point lies on a boundary or non-manifold edge (where the
pseudo-normal means nothing) take the sign of the generalized winding
number instead (inside where it exceeds ½), evaluated with Barill et
al.'s hierarchy: exact for near triangles, a dipole per cluster far
away. A small hole (a neck cut, the soles of a scan) then reads as
closed.

### Arrangement points
Fourteen points (mm, in the body's frame: Y up, facing +Z, the left of
the body at +x), guessed on import and when the orientation changes,
moved one by one (`avatar.set_point`), guessed again on demand
(`avatar.guess_points`):
- `chest`, `waist`, `hips`: the middle of the torso at those levels:
  the measurements are taken there (the largest girth between 68 and
  76 % of the height, the smallest between 57 and 66 %, the largest
  between 47 and 56 %);
- `crotch`: 1 cm above where the legs part (the section loop around the
  body's axis ends there), the start of trouser legs;
- `left_shoulder`, `left_elbow`, `left_wrist` and the right ones: the
  axis sleeves are wrapped around, found as for the built-in body (the
  upper arm's and the forearm's main directions, the wrist where the
  forearm is thinnest). The guess expects arms down or in an A pose; for
  a T pose, move them;
- `left_thigh`, `left_shin` and the right ones: the axis of each trouser
  leg (the middles of the thigh at 34 % and of the shin at 10 % of the
  height).
They lie inside the body; the check flags one outside it. Moving
`chest`, `waist` or `hips` measures the body again at its height.

### Measurements
Taken like the built-in body's (`r3d_avatar::measure`): height from the
floor, girths as a tape around the torso's section (its convex hull,
arms left out) at the chest, waist and hips points.
`masculinity` (which only templates read) is estimated from the
waist-to-hips ratio: 0 at 0.72 or less, 1 at 0.90 or more. Out of the
ranges `avatar.set` accepts, a value is brought into range and flagged.

### Motions
An imported body plays every motion of the built-in one (spec 22), in
the app, `simulate` and the game export. The motions are written for our
24-bone skeleton standing in its rest (A) pose; the body stands as its
file has it (an A or a T pose, its own proportions). So:
- **The reference.** The built-in body fitted to the measurements taken
  on the imported one, with its skeleton.
- **Where the joints are** (the anchors: the pelvis, the bottom of the
  neck and of the head, per side the shoulder, elbow and wrist joints,
  the hip, knee and ankle joints, and the clavicle when known):
  - **The file's skeleton**, when it has one (a glTF skin) in which our
    joints are found by their names, as for a captured motion (spec 22:
    hips, head, neck, spine, and per side the arm, forearm, hand, up-leg,
    leg, foot and optionally the clavicle and toe, whatever the namespace,
    case and side spelling). The joints stand where the inverse bind
    matrices put them, placed as the body (units, up axis, facing).
  - **Otherwise, the arrangement points.** The same points are found on
    the reference by the guess; each reference joint keeps its place
    relative to them: the pelvis, neck and head their offset from the
    torso's middle at their height, an arm's joints the turn and scale
    that take the reference's shoulder–elbow (and elbow–wrist) points
    onto the body's, a leg's joints those that take its axis points onto
    the body's. Moving the points moves the skeleton: on a T pose, the
    shoulder, elbow and wrist points moved onto the raised arms raise the
    skeleton's arms.
  The other joints follow the reference's proportions between the
  anchors.
- **The rest skeleton and the stance.** Each bone, parents first, keeps
  its length on the body and takes the reference's direction: that rest
  skeleton is ours in its A pose with the body's proportions, so every
  motion plays on it as on the built-in body. The stance is the pose
  that turns it into the body as it stands (each bone turned by the
  shortest arc from the reference's direction to the body's; a bone of
  no length takes its first child's direction, or its parent's turn).
- **Skin weights.** From the file when every vertex has them: each of
  its joints' weights go to its bone of ours, or its nearest ancestor's
  that has one (fingers to the hand, twist bones to their limb), the
  four heaviest kept. Otherwise carried from the reference posed in the
  stance: each vertex takes the weights at the closest point of its
  surface, smoothed over the body's mesh (spec 15's transfer).
- **Moving.** A motion's pose moves the body by each bone's motion from
  the stance to that pose (the stance's inverse, then the pose), so the
  body stands as imported until a motion starts, and comes from its
  stance into the motion over the same first second as the built-in body
  from its rest; stopped, it goes back to its stance. The soles (foot
  vertices within 2 cm of the floor) keep the body on the floor, the
  arms are kept clear of the body as on the built-in one.

## UX
- The top bar's **Body** button opens a menu: Measurements…, Import
  body…, Check body…, Show arrangement points, Use the built-in body.
- **Import body…** asks for an `.obj`, `.gltf`, `.glb`, `.ply` or `.stl`
  file, then
  shows the check.
- **Check body** is a dialog: the counts and the height, the units, up
  axis and turn with fields to correct them, and each item of the report
  with an icon by severity. A badge on the Body button shows when the
  imported body has warnings or errors.
- **Arrangement points** are drawn in the 3D view as dots (red when
  flagged); the dialog lists them with their coordinates in cm, to edit,
  and a button to guess them again.
- Starting the drape on a body with warnings says so (a snack bar with
  "Check body"); with errors, it does not start.

## Acceptance criteria
- An OBJ written in centimetres, Z-up, and a GLB in metres, Y-up, of the
  same body import to the same body within a millimetre: units, up axis
  and facing detected.
- The built-in body imported back gives measurements within 2 cm of
  its own, and points inside it.
- A body with holes cut in it has a field whose sign is right in the
  holes (winding number), on the CPU; the GPU reads the same field.
- A T-shirt drapes on an imported body (CPU and GPU) with its seams
  closed and no particle more than 3 mm inside.
- Every command (`avatar.import_mesh`, `avatar.orient`,
  `avatar.set_point`, `avatar.guess_points`, `avatar.remove_mesh`) has
  an apply → undo → redo test; the golden file v18 holds an imported
  body.
- Motions (M37): a game character (the built-in body under another
  rig's joint names, its twist bones unknown to us) imported as GLB
  moves with its own skeleton (`motion.rig` `file`), each point of its
  skin within 4 mm on average of the built-in body's walking; the
  built-in body raised to a T pose and imported as OBJ, its arm points
  moved onto the raised arms, stands in its T and walks arms down
  within 2 cm on average of the built-in body; on an imported body, a
  dress walks (CPU and GPU) and a T-shirt raises its arms, its hem
  riding up (seams closed, nothing more than 3 mm inside the moving
  body).
