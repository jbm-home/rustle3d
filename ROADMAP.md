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

- **Drafting (M11, M12, M32)**: points on edges and notches as distance
  targets; a gathering curve; cutting a piece and its linked mirror copy
  together (refused today: unlink first); constraints on internal lines,
  notches and construction lines; holes in mirror copies and symmetric
  pieces; seam allowance inside holes; moving a dimension's label by
  hand.
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
  solver convergence, physics.md §7).
- **Grading (M22)**: notches graded on their own; an ASTM rule table
  (`.rul`) and grading read from imported files; a graded template
  takes about 6 s to add on 9 sizes.
- **Parametric (M28)**: more body measurements (neck, shoulder, arm…);
  Seamly2D/Valentina steps `parallelCurve`, `graduatedCurve`,
  `ellipticalArcWithLength` and the union of pieces; seam allowances
  that change along the outline.
- **Construction details (M29)**: elastic and shirring, topstitching,
  interfacing, quilting and padding (need solver work).
- **Several garments (M19)**: edge–edge contacts between garments, and
  point–triangle contacts within a garment (its own folds stay a
  particle diameter apart); on the GPU a shirt and trousers step in
  13.9 ms (6.2 ms without contacts between garments), a hash of its own
  for self-collision is the lever.
- **Trims (M20)**: trims in photos (the path tracer) and in glTF/OBJ
  exports; buttons kept when pieces are copied and pasted, or mirrored;
  the Zipper tool on part of an edge (agents can); a button's own colour
  and shape; vertical buttonholes drawn along the band; a waistband with
  a button on the trousers; snaps, hooks and elastic; the overlapping
  bands of a shirt may pass through each other near a button
  (self-collision is off there, physics.md).
- **Prints (M21)**: placing pieces on the print by dragging them over
  it, stripe and check matching in the marker, weave relief (normal and
  roughness maps), transparency, a different print on the inside of the
  cloth, a user library of prints shared across projects.
- **Motion (M23)**: walking along a path, arms straight overhead (needs
  a skinning that does not fold the armpit, e.g. dual quaternions), a
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
- **Imported avatars (M31)**: motions of an imported body (mapping its
  skeleton, if any, onto ours), an imported body per size of a range,
  automatic fit of the arrangement points as the garment drapes, points
  guessed on a T pose, templates placing their pieces from the points
  rather than from fractions of the height, a disk cache of the distance
  field of large scans, compressed glTF.
- **Game export (M15)**: imports checked by hand in Godot, Unity and
  Unreal (no engine was run); the `MSFT_lod` extension; skinning an
  imported body; dual-quaternion skinning; the inside of a shell shows
  the outside's wrinkles reversed; roughness or sheen maps.

## Later (architecture kept ready, not scheduled)

Material calibration from measurements (a home protocol: a strip
hanging over an edge for bending, a weight and a ruler for stretch; a
community fabric library under CC0; only data whose licence allows
commercial use) · turntable video of the photo render (a runway walk) ·
step-by-step sewing instructions generated from the seams (needs a
sewing order) · OpenUSD export · finer cloth mesh with a strain limit
(smaller folds; physics.md §7) · picking points and notches in 3D ·
signed packages.
