# Roadmap

Every milestone ends with a green CI and updated docs; the maintainer
tests it against its spec. The order can change as we learn (see "Firm
vs. flexible" in `CLAUDE.md`). What each closed milestone built is in
its spec (`docs/specs/`), `docs/architecture.md`'s change log and the
git history; this file keeps the plan and what is still open. The
maintainer's briefs that specs cite as "ROADMAP Mxx" in their
`Provenance:` line are in this file's history before 2026-11-20.

## Milestones

| # | Milestone | Goal |
|---|---|---|
| M0 | Bootstrap ✅ | `cargo build && cargo test` green, planning docs |
| M1 | Document + commands + CLI ✅ | CLI script builds panels, undo/redo, save/reopen |
| M2 | Geometry, 2D export, headless MCP ✅ | An agent drafts a skirt pattern via MCP and exports a tiled PDF |
| M3 | Flutter + Rust spike, design system, 2D editor ✅ | Draw and edit panels in a polished UI |
| M4–M7 | Simulation, avatar, sewing, materials ✅ (2026-11-15) | A skirt made from scratch, sewn and draped; the same T-shirt in jersey vs denim |
| M8 | 3D → 2D editing tools ✅ (2026-11-15) | Lengthen a hem and take in a seam directly in 3D; the pattern follows |
| M10 | Templates, guided flow, polish → **v0.1** ✅ (2026-11-16) | A first-time user makes a T-shirt in < 10 minutes |
| M9 | glTF export, attached MCP ✅ | An agent edits the garment live in the open app; the result opens in a game engine |
| M11 | Pattern drafting tools ✅ (2026-11-16) | Draw a bodice with the curve tool, set the distance between two points, gather a skirt into a waistband |
| M12 | Drafting tools 2: curve family, cut ✅ (2026-11-16) | Draw a curved piece in one go, cut a bodice along a princess line into two sewn pieces |
| M13 | Pattern interchange: DXF, SVG and PDF ✅ (2026-11-16) | A pattern drawn elsewhere is imported, edited, sewn and exported back |
| M14 | Photorealistic rendering ✅ (2026-11-17) | A path-traced studio image of the draped garment |
| M16 | Zero-copy views on Windows ✅ and Linux | The garment drapes as smoothly on Windows and Linux as on macOS |
| M17 | GPU cloth solver ✅ (2026-11-17) | A T-shirt drapes at 60 fps; a dress stays interactive |
| M18 | Trousers and shirt templates ✅ (2026-11-17) | A first-time user makes trousers and a shirt from templates |
| M19 | Layers and multiple garments ✅ (2026-11-19) | A shirt tucked into trousers, each in its own fabric and colour |
| M22 | Grading and sizes ✅ (2026-11-18) | A shirt graded from 36 to 52, each size draped on a matching avatar |
| M25 | Maximized views ✅ (2026-11-18) | The board fills the window, the drape pauses; one click brings both views back |
| M26 | History, editing comfort, feedback on long steps ✅ (2026-11-18) | Jump back ten steps from the history panel; paste two sewn pieces with their seam |
| M27 | Sketching 2: constraints and symmetry ✅ (2026-11-18) | Draw a half bodice against a symmetry axis with fixed dimensions; change one, the shape follows |
| M28 | Parametric patterns ✅ (2026-11-18) | A skirt whose waist is `waist / 4 + 10 mm` redrafts itself on another body |
| M29 | Construction details ✅ (2026-11-18) | A skirt with darts, a pleat and a patch pocket sewn on an internal line |
| M32 | Editor finishing ✅ (2026-11-18) | A patch pocket sewn on its line and a dimension drawn on the board, all with the mouse |
| M23 | Animated avatars ✅ (2026-11-19) | The avatar walks and the garment follows |
| M30 | Production: marker, yardage, tech pack, projector ✅ (2026-11-19) | A graded shirt nested on a 150 cm fabric, its yardage and cost, its tech pack PDF |
| M24 | Fit maps and virtual tape measure ✅ (2026-11-19) | Tight spots show in colour; the ease at the chest is read on the drape |
| M20 | Trims: buttons and zippers ✅ (2026-11-19) | A shirt that buttons and trousers with a fly zipper, draped closed |
| M21 | Textures and prints ✅ (2026-11-19) | A striped shirt whose stripes follow the grainline on every piece |
| M31 | Imported avatars ✅ (2026-11-19) | A body scan in OBJ/glTF is checked, the pieces placed around it and draped |
| M15 | Game-ready export ✅ (2026-11-19) | LODs + baked wrinkle normal maps + skinning, imported in Godot/Unity/Unreal |
| M33 | Body measurements ✅ (2026-11-23) | A project made with four measurements; later its arm length and neck are set, the avatar and a sleeve drafted from `arm_length` follow |
| M34 | Construction details 2 ✅ (2026-11-24) | A cuff gathered by elastic, a topstitched yoke, a collar that stands thanks to its interfacing, a quilted panel |
| M35 | Sketching on the board: the action bar ✅ (2026-11-24) | A half bodice drawn and constrained without the inspector: what fits the selection is offered above the board, one click applies it |
| M36 | Natural motions ✅ (2026-11-25) | The walk looks human: no hop, a supple trunk, relaxed arms |
| M37 | Imported avatars in motion, game export checked ✅ (2026-11-25) | A scanned body walks in its garment; the garment and its motion open in a game engine as exported |
| M38 | Pattern library ✅ (2026-11-25) | In an open project, a jean and a short are added from the library; a skirt saved there from one project is added to another, drafted for its body |
| M39 | Children ✅ (2026-11-26) | A 4-year-old girl's pyjama and a baby's bodysuit, drafted on children's bodies and graded by age |
| M40 | Body scans in PLY and STL ✅ (2026-11-26) | A body scan saved as PLY or STL is imported, checked and draped like an OBJ |
| M41 | Diagnostic report ✅ (2026-11-26) | An agent marks the faces where a drape goes wrong and reads why: stretch, contacts, seam gaps |
| M42 | The user's own HDRI ✅ (2026-11-26) | A photo render lit by an environment the user brings (`.hdr`) |
| M42b | Photo scene ✅ (2026-11-26) | The light's turn and brightness and the lens are set while the 3D view shows the photo as it will be; the scene is saved with the project |
| M43 | Fabric relief: normal and roughness maps | A denim's weave and a satin's sheen show in the 3D view and the photo |
| M44 | Fabric presets from open measurements | Presets whose weights and thicknesses come from a published dataset, each value with its source |
| M45 | Self-collision by contacts | A folded skirt's layers lie on each other, without the puffy ~8 mm gap |
| M46 | Objects: projects without a body ✅ (2026-11-28) | A cushion drapes round and smooth, stuffed; a tote bag as an object hangs flat, without folds |
| M47 | The body at creation: size, silhouette, height ✅ (2026-11-28) | A project made for a women's 38 pear shape, 1.60 m; its size range starts around its own size |
| M48 | Sizes and units of the user's region ✅ 2026-11-28 (step 1: sizes, body measurements; step 2: every length) | A user in the US sees a 6 for the 38 and measurements in inches; every length of the app follows (step 2) |

Next: M43; M16's Linux part when the work
moves to Linux.

## Planned milestones (proposed 2026-11-26)

- **M40 — Body scans in PLY and STL**: PLY (ASCII and binary) and STL
  (ASCII and binary) read like OBJ (spec 27): the up axis detected,
  colours and normals ignored. No FBX: it is a closed format whose
  readers come from reverse engineering (CLAUDE.md §1); the user guide
  explains how to convert one to GLB with Blender.
- **M41 — Diagnostic report**: a command (CLI and MCP) that reports on
  the marked faces of a drape, or the worst ones: stretch and area per
  face, nearby contacts, seam gaps, the most strained faces, with the
  project and its drape state for replay.
- **M42 — The user's own HDRI**: an environment map in Radiance `.hdr`
  lights the photo render (spec 14), next to the three built-in
  studios; a default set under CC0 (e.g. Poly Haven). OpenEXR later.
- **M42b — Photo scene** (from the maintainer's use of M42): the photo
  set up with a live path-traced preview in the 3D view, a lens (focal
  length), the light's turn and brightness for the studios too, saved
  in the project (schema v28) and used by `render_photo` (spec 35).
- **M43 — Fabric relief**: normal and roughness maps for fabrics,
  alongside the colour texture (spec 25), in the 3D view, the photo
  render and the glTF export; a default set under CC0 (e.g. ambientCG,
  Poly Haven).
- **M44 — Fabric presets from open measurements**: weights and
  thicknesses from published fabric datasets (e.g. the MIT Fabric
  Properties Dataset, Bouman et al., ICCV 2013), each value with its
  source. First check that the data's licence allows commercial use;
  without it, the milestone is dropped.
- **M45 — Self-collision by contacts** (Offset Geometric Contact, Chen
  et al., SIGGRAPH 2025): point–triangle and edge–edge contacts with a
  no-crossing guarantee, instead of the particle spheres whose layers
  stay up to ~8 mm apart (physics.md §5), on the CPU and the GPU.
  Reimplemented from the paper.

## Test report 2026-11-24 (open points: to take up next)

The maintainer had an agent build a hoodie and joggers over MCP, then
used the app.

- **Bugs**:
  - Done: the 3D view became unusable after pulling the garment in
    the 3D view (project "ProjetTest").
  - Done: the trousers fell because no elastic held anything up
    (friction did not count an elastic's tension, physics.md, Elastic).
    `seam.add` had chosen the right direction for the crotch; the
    flipped one twisted it, which held the trousers by the twist (118–131
    %). "Not settled" came from the last substep's speed (now the
    frame's).
  - Done: the seams beside an elastic waist stayed open (36 mm on a
    jersey tube): the elastic pulled their ends apart and its pressure
    made friction hold each end where it was. The elastics are solved
    before the seams, and friction holds a seam's two ends by their
    common motion where an elastic pulls on it (physics.md §5,
    "Friction on a seam").
- **Annoyances** (all done): templates no longer rename the project; a
  range's end within 0.05 mm of the path's length is its end; a new
  piece joins the newest garment with a 10 mm allowance; an allowance set
  on a linked copy goes to its piece.
- **Missing**:
  - Done: hoodie and joggers templates (spec 11).
  - Done: a garment taken out of the drape and put back
    (`garment.set_draped`, spec 17).

## M16 — Zero-copy views on Linux (open)

macOS shares the views' texture with Flutter (IOSurface), Windows through
a shared D3D11 texture (done 2026-11-17: UI at 59.9 fps against 36 fps
read back). Linux still renders into a texture of its own and reads it
back through the CPU every frame.

- Linux: a Vulkan texture exported as a dma-buf, imported by the GTK
  runner as an EGL image behind an `FlTextureGL`.
- `unsafe` kept to the sharing code in `r3d-ffi`, as for the IOSurface;
  the readback stays as the fallback (WSL, software GPUs).
- Pass: in release builds, 60 fps with the 3D view draping and UI
  animations running, input latency under one frame, on native Linux.

## Open points of closed milestones

As noted when each milestone closed; recheck before taking one up.

- **Environment lighting (M42, M42b)**: OpenEXR; light bounced off the
  garment onto the floor; the photo scene's panel and its live preview
  not yet seen on screen by the maintainer; the preview shows the drape
  as it was when the scene opened (no refresh while the drape runs).
- **Diagnostic report (M41)**: marking the faces with the mouse in the
  3D view and copying the report for an agent; a signed distance to a
  moving body.
- **Body scans (M40)**: PLY and STL tested on files we wrote, not yet
  on a real scanner's file.
- **Children (M39)**: the app's new controls (Adult | Child and the age
  in the Body panel, the five charts in the sizes dialog, the children's
  cards on the start screen and in the library) were not seen on screen
  (captures came back white): to review. The bodysuit's envelope
  neckline (lapped shoulders), snaps as a trim (its crotch is sewn),
  binding on its leg openings; bodies from 14 to 18 are mixed with the adult's
  (MakeHuman has no teenager); a baby's hip height wanders between belly
  and hips (its estimate is not checked); a child's own gait (an adult's
  walk scaled to its legs).

- **Imported bodies in motion, game export (M37)**: the Godot check
  (`scripts/godot-check.sh`, a CI job) has not run yet: its first CI
  run, or locally with Godot installed; Unity, Unreal and Blender by
  hand with spec 15's check list. An imported body's hands and feet
  keep the turn of its forearms and shins (a few centimetres at the
  fingertips), and its file's extra bones (fingers, twist bones) follow
  ours.
- **Natural motions (M36)**: arms straight overhead (the body takes it
  with dual quaternion skinning, but the shoulders' skin carries a
  T-shirt's neckline in and up and it rolls over at the chest; the
  motion stops at a V); walking forward along a path (on the spot, as
  on a treadmill); the knee bends about 7° as the foot takes the weight
  where published curves give 15–20°; standing keeps the rest pose's
  wide stance; the stop back to rest blends poses, so the feet may
  slide during that second; the game export skins by glTF's linear
  blend (an engine's elbows and shoulders differ a little from ours).

- **Construction details 2 (M34)**: elastics, topstitches and the like
  copied with a pasted or mirrored piece; bending across seams (a collar
  turns freely at its neckline seam, the shirt's stands by its ring
  shape); quilting with two simulated layers and a padding volume; the
  game export's baked textures do not draw stitches (the meshes carry
  the puffs); an elastic-cuff variant of the shirt; gathers finer than
  about 8 mm (the mesh along an elastic).

- **Sketching (M35)**: constraints on internal lines (their points
  need an identity first: a schema change); a notch as the end of a
  straight distance; construction lines moved by their constraints
  (today they are references the corners follow); a point snapped on a
  crossing stays there only through constraints added after;
  single-letter keys for the action bar's buttons; a named dimension
  without a formula gives the drawn size's value in every size of a
  range.
- **Drafting (M11, M12, M32)**: a gathering
  curve; cutting a piece and its linked mirror copy together (refused
  today: unlink first); holes in mirror copies and symmetric pieces;
  seam allowance inside holes.
- **Interchange (M13)**: sizes and grading (AAMA RUL files), seams in
  the exchange (DXF-AAMA has none), PDFs whose pages hold different
  parts of the pattern, binary DXF, pieces with holes, the tiling
  preview.
- **Photo render (M14)**: hardware ray queries (an unsafe, experimental
  wgpu feature), the user's own HDRI, weave normal maps, cloth thickness
  at the edges.
- **Templates (M12, M18, M22)**: a waistband and darts on the trousers
  and shirt; trousers hold at the waist by friction only (a 52 settles
  under the belly, its hem on the floor); a women's 52 shirt parts a
  little below its last button over the belly (no darts); on the Men L
  body an A-line skirt in denim slides off the hips (weak waist grip;
  solver convergence, physics.md §7). On macOS two tests fail just
  under their threshold, the same weak hold: `trousers_fit_a_mens_body`
  (top at 1.029 m for 1.03) and, since the avatar is fitted to every
  chart size (52a8621), `edits_carry_the_drape_over_without_a_jump` (a
  hem 5 cm longer hangs 1.9 cm lower, 2 asked).
- **Grading (M22)**: notches graded on their own; an ASTM rule table
  (`.rul`) and grading read from imported files; a graded template
  takes about 6 s to add on 9 sizes.
- **Parametric (M28)**: Seamly2D/Valentina steps `parallelCurve`, `graduatedCurve`,
  `ellipticalArcWithLength` and the union of pieces; seam allowances
  that change along the outline.
- **Body measurements (M33)**: the other measurements in the user's
  own presets; Seamly2D/Valentina measurement files giving them
  (`neck_base_circ`… still become variables); wrist, knee, ankle, front
  waist length, waist height and crotch depth on the 3D body (no
  MakeHuman targets in the pack: `measure-wrist-circ` and the like could
  be added); the body model's own proportions, which the estimates copy,
  are short in the arm (51 cm for a 168 cm woman, about 58 usual) and
  thin at the wrist, knee and ankle; the shirt's sleeve is `arm_length -
  2 cm` because the whole length let a tucked shirt show through the
  trousers.
- **Several garments (M19)**: edge–edge contacts between garments, and
  point–triangle contacts within a garment (its own folds stay a
  particle diameter apart); on the GPU a shirt and trousers step in
  13.9 ms (6.2 ms without contacts between garments), a hash of its own
  for self-collision is the lever.
- **Trims (M20)**: trims in glTF/OBJ exports (photos show them since
  M34); buttons kept when pieces are copied and pasted, or mirrored;
  the Zipper tool on part of an edge (agents can); a button's own colour
  and shape; vertical buttonholes drawn along the band; a waistband with
  a button on the trousers; snaps, hooks and elastic; the overlapping
  bands of a shirt may pass through each other near a button
  (self-collision is off there, physics.md).
- **Prints (M21)**: placing pieces on the print by dragging them over
  it, stripe and check matching in the marker, weave relief (normal and
  roughness maps), transparency, a different print on the inside of the
  cloth, a user library of prints shared across projects.
- **Motion (M23)** (arms overhead: see M36): walking along a path, a
  timeline and a video of a motion.
- **Production (M30)**: trims in the bill of materials; the projector in
  the operating system's own full screen and keystone correction; points
  of measure named by the user; true-shape nesting (no-fit polygons) and
  a gap between pieces, one-way fabrics (nap, prints), folded fabric and
  several plies, a marker of several garments in one fabric, PDF pages
  longer than 5 m for Acrobat.
- **Fit (M24)**: a stress map (force per width: strain times the
  fabric's stiffness per direction) and a pressure map; the tape
  following the cloth as it moves; the length of an opening's edge (a
  neckline, an armhole).
- **Imported avatars (M31)**: an imported body per
  size of a range,
  automatic fit of the arrangement points as the garment drapes, points
  guessed on a T pose, templates placing their pieces from the points
  rather than from fractions of the height (on the built-in body raised
  to a T pose, its arm points moved, a T-shirt's seams stay 8–9 mm open
  instead of 1), a disk cache of the distance
  field of large scans, compressed glTF.
- **Game export (M15)**: the `MSFT_lod` extension; roughness or sheen
  maps.
- **Objects (M46)**: an object resting on a table (gravity and a floor:
  a bag standing on its base); stuffing per garment; a weightless tote
  bag's handles wave a little (two strips joined only at their ends).
- **Body at creation (M47)**: the fifteen other measurements at
  creation; a silhouette for an imported body.

## Later (architecture kept ready, not scheduled)

Material calibration from measurements (a home protocol: a strip
hanging over an edge for bending, a weight and a ruler for stretch; a
community fabric library under CC0; only data whose licence allows
commercial use) · turntable video of the photo render (a runway walk) ·
step-by-step sewing instructions generated from the seams (needs a
sewing order) · OpenUSD export, and bodies imported from OpenUSD
(`.usda`; `.usdc`/`.usdz` need a binary reader) · finer cloth mesh with a strain limit
(smaller folds; physics.md §7), finer where the cloth folds (near
seams, gathers, pleats), sized per the physics-inspired estimate of
Zhang et al., SIGGRAPH 2025 · picking points and notches in 3D ·
signed packages.
