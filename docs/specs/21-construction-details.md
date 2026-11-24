# 21 — Construction details

Provenance: maintainer's functional brief (ROADMAP M29, 2026-11-18: seams on part of an edge and on chains of edges with ease, seams on internal lines for patch pockets and tucks, darts, pleats, holes, booleans between pieces); general garment-making knowledge (darts sewn leg to leg, knife and box pleats folded and stitched down, patch pockets on placement lines, ease in a sleeve cap); public papers: Chew 1989 / Ruppert 1995 refinement as implemented by `spade` for the meshes, constrained Delaunay triangulation for region booleans.

## Status (2026-11-18, M29)

Implemented: seam sides that are a whole edge, a run of consecutive
outline edges, or an internal line, all of it or a stretch (`from_mm`,
`to_mm`); seam ease (`seam.set_ease`); darts (`dart.add`) and knife and
box pleats, optionally stitched down (`pleat.add`); holes in pieces
(`panel.add_hole`, `panel.remove_hole`); union and difference of pieces
(`panel.boolean`). The cloth follows: internal lines that are sewn and
holes are part of each piece's mesh. Project files: schema v13. The
app: the Dart, Pleat and Hole tools, Join or cut in the
inspector, the Ease of a seam in the edge card; since M32 the Sew tool
picks every kind of side (spec 02, The Sew tool).

Elastic and shirring, topstitching, interfacing, quilting and padding:
spec 31 (M34). Holes are
cut on their line (no seam allowance inward). Mirror copies and
symmetric pieces cannot have holes yet.

## User story
As a designer, I shape a skirt with darts and a pleat at the waist and
sew a patch pocket on its placement line, then see it drape; I sew a
sleeve cap with ease, sew a facing to part of an edge, and cut a piece
from another.

## Behavior
- **Seam sides.** A side is `edge` (one whole edge), `edges` (consecutive
  outline edges of one piece, in their order around it: a pocket's sides
  and bottom), or `line` (an internal line). `from_mm` and `to_mm` keep
  only a stretch of it (between two notches, say). Sides are matched
  point for point along their lengths, start to start unless reversed.
  A stretch already sewn cannot be sewn again; two stretches of one edge
  can be sewn to each other (a pleat). Shortened edges clamp a stretch.
- **Ease.** `ease_mm` is how much longer side `a` is meant to be than
  `b`; the extra is eased in along the seam, and only the difference
  beyond it is a mismatch (board, `seam.list`, `grading.check`).
- **Tools on whole-edge seams** (taking in, matching lengths, cutting
  across a seam, lengthening along a seam in 3D) work on seams between
  two whole edges and say so otherwise (`invalid.not_whole_edges`).
- **Darts.** `dart.add` opens the edge `width_mm` wide around `at_mm`,
  puts the dart's point `length_mm` into the piece, and sews the two
  legs together: the dart closes when the cloth drapes.
- **Pleats.** `pleat.add`: a knife pleat folds `depth_mm` under from
  `at_mm` (the edge's stretch of twice that is sewn to itself, folded in
  the middle); a box pleat is two such folds facing away from each
  other. With `stitched_mm`, the outer folds are stitched together that
  far down the piece, along internal lines. Notches mark the folds.
- **Holes.** Closed paths inside a piece, not crossing its outline or
  each other, left out of the cloth; their edges can be sewn and
  notched. They count in the piece's area, show on the board and in the
  exports (a cut line; a cut-out in DXF-AAMA).
- **Booleans.** `panel.boolean` joins two overlapping pieces (`union`) or
  cuts one with the other (`difference`: a bite out of the outline, or a
  hole when the other lies inside). The result is fitted with straight
  and curved edges (Schneider's curve fitting, as for imports). Sewn,
  symmetric or mirrored pieces must be unpicked or unlinked first; the
  result must be one piece.
- **Simulation.** Each piece is meshed with its holes left out and its
  sewn internal lines as mesh edges; every seam side is cut into the same
  number of segments as its partner, spread over its stretches by
  length.

## UX
- Rail: the Darts and pleats group (Dart, Pleat) in the sewing section,
  Hole in the drawing section (since 2026-11-19). Dart and Pleat: click the
  edge where it goes, type the sizes. Hole: select a piece, click the
  hole's corners inside it, Enter.
- Inspector: Join or cut (another piece, then Join or Cut out); in an
  edge's card, the seam's Ease.

## Acceptance criteria
- A pocket's three edges sewn to a U-shaped placement line: both sides
  470 mm, no mismatch; their points pair up in the mesh.
- A dart closes; undo removes it in one step.
- A skirt with two darts, a stitched knife pleat and a patch pocket
  drapes on the body with its seams closed (< 1 cm) and stays up.
- A hole leaves no cloth inside; a circle cut out of a piece becomes a
  hole of the circle's area (± 20 mm²).
- Files of schema v12 open (migration), v13 golden files load.
