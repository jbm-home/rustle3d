# 31 — Construction details 2: elastic, topstitching, interfacing, quilting

Provenance: maintainer's functional brief (ROADMAP M34, 2026-11-23: elastic on an edge or a line with a relaxed length, shirring as several parallel elastic lines; topstitching in one or more rows at a distance from an edge or a seam, with thread colour and stitch length, seen on the board, in 3D, in photos and in the tech pack; interfacing on a piece or part of it, stiffer to bend and stretch, in the bill of materials; quilting and padding, thickness between stitched lines; a cuff gathered by elastic, a topstitched yoke, a collar that stands thanks to its interfacing, a quilted panel), and the maintainer's choices of 2026-11-24 (quilting as one layer with its puffs drawn; a finer mesh along elastic lines; topstitching slightly stiffens the cloth; the shirt template uses them); general garment-making knowledge (elastic cut shorter than the edge it gathers and stretched as it is sewn; shirring elastic in rows about 1–2 cm apart; topstitching 2–3 mm stitches, 1–10 mm from an edge or a seam, often in a heavier or contrasting thread; fusible interfacing in light, medium and heavy weights, woven, non-woven or knit, on collars, cuffs, bands, facings and waistbands; quilting in channels or diamonds a few centimetres apart over a batting); public papers: XPBD (Macklin, Müller, Chentanez, MIG 2016) for the elastic, Choi and Ko (SIGGRAPH 2002) for cloth that buckles rather than compresses, docs/physics.md §3.

## Status

Done (M34, 2026-11-24): built in four steps, each one commit, then the
shirt template.

- Elastic and shirring (2026-11-24): the document (schema v22, details
  `detail:N`), `elastic.add`, `elastic.set`, `elastic.remove`,
  `elastic.shirr`, `detail.list`; the drape on both solvers; the board,
  the tech pack, the action bar and the inspector.
- Topstitching (2026-11-24): the document (schema v23),
  `topstitch.add`, `topstitch.set`, `topstitch.remove`, `detail.list`;
  the stiffer cloth under it on both solvers; the board, the 3D view
  and its renders, photos (which show buttons and zippers too since),
  the tech pack's sewing section; Topstitch in the action bar.
- Interfacing (2026-11-24): the document (schema v24, `fabric` and
  `weight`; `kind` names the detail), `interfacing.add`,
  `interfacing.set`, `interfacing.remove`, `detail.list`; the fused
  cloth on both solvers, its boundary a mesh line; hatched on the board;
  the bill of materials; Interfacing in the action bar (the piece, or a
  band along a picked edge). Lines inside a piece may cross since (the
  mesher joins them at their crossings).
- Quilting (2026-11-24): the document (schema v25), `quilting.add`,
  `quilting.set`, `quilting.remove`, `detail.list`; the padded cloth and
  its creases on both solvers, the puffs in every view and export; its
  stitches on the board, in 3D and photos; padding in the bill of
  materials and a Quilting table in the sewing section; Quilting in the
  action bar.
- The shirt template (2026-11-24): a yoke across the back, drafted as a
  piece of its own (the armholes cut at its seam, each sleeve's back cap
  split there so both seams grade), its seam topstitched in two rows;
  the collar and the cuffs interfaced. An elastic-cuff variant was not
  added: the shirt keeps its buttoned cuffs (an elastic is one action
  away on any hem).

## User story
As a designer, I gather a sleeve's cuff with an elastic two thirds its
length and see it close on the wrist; I shirr the top of a dress front
in five rows; I topstitch a shirt's yoke twice, 1 mm and 6 mm from its
seam, in a white heavy thread, and see the stitches on the board, the
3D view, the photo and the tech pack; I interface the collar stand and
see it stand; I quilt a jacket's front in 5 cm diamonds with 8 mm
padding and see it puff between the stitches. The tech pack lists the
elastic per size, the interfacing and the padding with their areas,
and the topstitching.

## Behavior

### Elastic
- An **elastic** runs along a side, given as a seam's: a whole edge,
  consecutive outline edges, or an internal line, all of it or a
  stretch (`from_mm`, `to_mm`).
- Its **ratio** is its relaxed length over the side's length
  (0.2–1; default 0.75): a 30 cm edge with a ratio of 2/3 takes a
  20 cm elastic. The command also takes the relaxed length in mm and
  turns it into the ratio, so the elastic follows the edge when it
  changes and grades with it (each size's length is that size's side
  × the ratio).
- Its **width** (2–50 mm; default 10) sets how strongly it pulls: about
  0.8 N per mm of width when stretched to twice its length (general
  knowledge of knitted and braided elastics; an estimate to calibrate).
  On the body, an elastic shorter than what it goes round (an arm, a
  waist) stays stretched on it.
- A side can carry one elastic (a stretch of it, another); an elastic
  and a seam or a zipper may share a side (an elastic sewn into a
  waistband seam).
- **Shirring** (`elastic.shirr`): several parallel rows across a piece,
  each a new internal line with its elastic, made in one step. The rows
  run parallel to an edge's chord, the first at a distance from the
  edge, the next ones a gap apart (default 15 mm), across the whole
  piece, stopping 1 mm short of its outline. Undo removes the lines
  and the elastics together.
- Each row is a **shirring row** (schema v29, 2026-11-27): when the
  piece's outline changes (a point moved, an edge reshaped), the row
  keeps its straight line and its
  ends follow the outline, 1 mm short of it, on the stretch of the
  piece nearest the row's middle; a row the piece no longer runs
  across (under 10 mm) goes, with its elastic. Removing a row's elastic
  (`elastic.remove`) removes its line too, unless something else is on
  it (a seam, a topstitch…). A row whose line was redrawn as a curve is
  left as it is.
- Removing the side (the edge or the line) removes its elastic.
- An internal line left outside its piece by an outline edit (a line
  drawn by hand with an elastic, a seam) is named by the drape:
  `simulate` fails with `line_outside` and the line, to move back
  inside or remove.

### Topstitching
- A **topstitch** runs along a side (as an elastic's) in 1 to 3
  **rows**: the first `offset_mm` from the side (default 6), the next
  ones `gap_mm` further (default 6). On outline edges the rows are
  inside the piece (≥ 1 mm). On an internal line they are on its left
  (walking from its start) when the offset is positive, on its right
  when negative, on it when 0.
- A **stitch length** (1–8 mm; default 3), a **thread colour**
  (`#RRGGBB`; default a darker shade of the garment's colour) and a
  **thread**: normal or heavy (topstitching thread, about twice as
  thick).
- To topstitch beside a seam (a yoke), topstitch the edge of the piece
  the stitches show on.
- The cloth under the rows (each triangle with a corner within the
  band from the side to the last row, plus 2 mm) bends 1.5 times
  harder: a topstitched edge stays flatter.
- Rows are clipped to the piece; removing the side removes the
  topstitch.

### Interfacing
- An **interfacing** is fused on part of a piece: the **whole piece**,
  a **band** along consecutive outline edges (`width_mm`, a facing, a
  waistband, a front band), or the **inside of a closed internal line**.
- Its **fabric** (woven, non-woven, knit) and **weight** (light, medium,
  heavy; default woven, medium) give its physical values (docs/physics.md §3, Interfacing): it
  adds its weight and thickness, makes the cloth much stiffer to bend
  (fused layers bend as one) and stiffer to stretch and shear (a knit
  one stretches with the cloth).
- Interfacings on one piece do not overlap (`invalid.overlap`).
- Removing the piece, the edges or the line removes it.

### Quilting
- A **quilting** covers the whole piece or the inside of a closed
  internal line, in **channels** (parallel lines) or **diamonds** (two
  families of lines), at an angle from the grainline (default 45° for
  diamonds, 0° for channels) and a **spacing** (15–150 mm; default
  50), over a **padding** of a loft (thickness, 1–30 mm; default 8) and
  a weight (g/m²; default 100). Its stitches are drawn as topstitching
  (stitch length, thread colour).
- One layer of cloth stands for the shell, the padding and the lining:
  it is heavier, thicker against the body and other cloth, and stiffer
  to bend; it folds more easily along the stitched lines. Between the
  lines the surface puffs out by up to the loft, in the 3D view, photos
  and 3D exports.
- One quilting per area; areas on a piece do not overlap. The lines run
  through the area's middle, a spacing apart; none is stitched on the
  area's edge.

### Drape
- **Elastic**: a tie along the side that only pulls, its rest length
  the side's times the ratio, reached over the sewing ramp. The cloth
  along the elastic gathers: it may shorten there without resisting
  (it buckles, finer than the mesh can show), and the cloth beyond it
  folds. Along elastic lines the cloth is meshed finer (about 8 mm, in
  a band of three triangles on each side), so the gathers are small.
  Lines inside a piece (shirring rows, an interfacing's edge, sewn
  lines) may cross: the mesh joins them where they cross.
- **Topstitching, interfacing, quilting** change the cloth where they
  are: the triangles there take the garment's fabric combined with
  them. Their boundaries (an interfacing's edge, the stitched lines of a
  quilting) are mesh lines, so the change is crisp.
- A quilted piece is meshed at a quarter of its quilting's spacing at
  most, so the puffs have the points to show.
- Changing a detail during a drape rebuilds the scene; the cloth carries
  on.
- `simulate` reports the elastics' lengths on the cloth and how far each
  is from its relaxed length.

### Board
- Elastic: a green zigzag along its side (its ratio is in the
  inspector). Shirring: each row likewise.
- Topstitching: its rows dashed at the stitch length, in the thread's
  colour over a faint dark edge (a white thread on a white piece).
- Interfacing: its area hatched at 45°.
- Quilting: its stitched lines dashed, its area outlined.

### 3D view and photos
- Topstitches and quilting stitches as short strips of thread on the
  cloth, 1 mm above it, at their length and the thread's width (0.4 mm,
  0.8 mm heavy), in the thread's colour, a 0.6 mm gap where the thread
  goes in. In the app's 3D view, the headless renders, the tech pack's
  pictures and the photos (with buttons and zippers).
- Quilting puffs, as above.

### Tech pack
- Bill of materials, per garment: elastic by width, with its length per
  size (relaxed, rounded up to the centimetre) and the total; interfacing
  by kind and weight, with its area (m²); padding by loft and weight,
  with its area.
- Sewing section: a Topstitching table (piece, length, rows, distance,
  gap, stitch length and stitches per inch, thread and colour), an
  Elastics table (piece, length, relaxed length in % and cm, width),
  and the quiltings (piece, pattern, spacing, loft).

## Commands and queries
- `elastic.add` (side, ratio? | relaxed_mm?, width_mm?), `elastic.set`
  (ratio?, relaxed_mm?, width_mm?), `elastic.remove`, `elastic.shirr`
  (panel, edge, rows, first_mm?, gap_mm?, ratio?, width_mm?).
- `topstitch.add` (side, rows?, offset_mm?, gap_mm?, stitch_mm?,
  color?, thread?), `topstitch.set`, `topstitch.remove`.
- `interfacing.add` (panel, area?: {kind: whole} | {kind: band, edges,
  width_mm} | {kind: inside, line}, fabric?, weight?), `interfacing.set`
  (fabric?, weight?, width_mm?), `interfacing.remove`.
- `quilting.add` (panel, line?, pattern, angle_deg?, spacing_mm?,
  loft_mm?, padding_gsm?, stitch_mm?, color?), `quilting.set`,
  `quilting.remove`.
- `detail.list`: every elastic (its side, ratio, relaxed length in each
  size), topstitch (its rows' lengths), interfacing (its area in mm²)
  and quilting (its area, its stitched lines' total length).

## UX
- The action bar (spec 30): an edge picked offers **Elastic**,
  **Shirring** (rows parallel to that edge) and **Topstitch**; a piece
  alone offers **Interfacing** and **Quilting**. Each asks for its
  values in a small dialog.
- The inspector's **Construction details** card lists the selected
  piece's elastics (relaxed length and width; a Relaxed length slider),
  topstitches, interfacings and quiltings, each with a remove action.

## Acceptance criteria
- Every command round-trips apply → undo → redo; a golden file per new
  schema version holds the new details; older files load unchanged.
- **A cuff gathered by elastic**: a T-shirt's sleeve hem (≈ 330 mm)
  with an elastic of ratio 0.6 closes to the elastic's length ± 5 % on
  the cloth where nothing holds it open; on the forearm it stays
  stretched on it. On a hanging sheet the cloth beside the elastic
  gathers (its rows are at least 20 % longer than they span). No NaN.
- **A topstitched yoke**: two rows on the yoke's seam edge show on the
  board, in the 3D view and in a photo (their pixels in the thread's
  colour), and in the tech pack.
- **A collar that stands**: a 30 × 8 cm band of cotton clamped at its
  foot, with heavy interfacing, stays up (its top ≥ 80 % of its height
  above its foot) where the bare band folds over (< 70 %), on both
  solvers. (The shirt's collar, a closed ring around the neck, already
  stands without: 0.87 of its width, 0.89 with heavy interfacing.)
- **A quilted panel**: a quilted panel weighs its cloth plus padding,
  and its points between stitched lines stand out from the lines by
  about the loft.
- Removing a piece removes its details in the same undo step.
