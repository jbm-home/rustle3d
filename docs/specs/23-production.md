# 23 — Production: indicators, marker, yardage, cost, tech pack and projector

Provenance: maintainer's functional brief (ROADMAP M30, 2026-11-18: indicators per piece, a marker of one or several sizes on a given fabric width with the grainline respected, yardage and cost from a fabric price, an algorithm from the open literature); public papers: S. Jakobs, "On genetic algorithms for the packing of polygons", European Journal of Operational Research 88 (1996) 165–181 (the bottom-left placement rule); E. K. Burke, R. S. R. Hellier, G. Kendall, G. Whitwell, "A new bottom-left-fill heuristic algorithm for the two-dimensional irregular packing problem", Operations Research 54(3) (2006) 587–601 (bottom-left-fill: holes left behind are filled); J. A. Bennell, J. F. Oliveira, "The geometry of nesting problems: a tutorial", European Journal of Operational Research 184 (2008) 397–415 (raster representations of pieces and their overlap test); J. F. Oliveira, J. S. Ferreira, "Algorithms for nesting problems", Applied Simulated Annealing, Springer LNEMS 396 (1993) 255–273 (raster nesting); general garment-making knowledge (markers laid along the selvedge with the grainline parallel to it, fabric sold by the running metre at a given width, weights in g/m², marker efficiency as the share of the fabric used; what a tech pack holds: views of the garment, a bill of materials, measurements per size, colourways and construction notes; projecting a pattern at full size onto fabric on the cutting table, checked with a ruler on a projected square). Second half: maintainer's brief of 2026-11-18 (tech pack: views, measurement table, bill of materials, colourways, sewing notes; projector: full-screen calibrated view, high contrast, mirror, pan, calibration as an app preference); the PDF format (ISO 32000, an open standard).

## Status (2026-11-19, M30 done)

Implemented: fabric details on each garment (weight, width, price per
metre; the weight also drives the drape), indicators per piece and per
garment (`production.indicators`), the marker (`marker.compute`),
its SVG/PDF export (`export.marker`, `r3d marker`), the app's
indicators in the inspector and its Marker dialog; the garment's notes
(`garment.set_notes`, schema v15), the tech pack (`export.techpack`,
`r3d techpack`, the MCP tool `export_techpack`, Export → Tech pack… in
the app), the projector view in the app.

Later: trims in the bill of materials (M20), the operating system's own
full screen for the projector view (the app fills its window; the user
maximises it on the projector), keystone correction (a projector not
square to the table), points of measure named by the user in the
measurement table. Also later:
true-shape nesting with no-fit polygons (the raster is accurate to its
cell, 5 mm by default), a gap between pieces, fabric with a nap or a
print to match (one direction only), folded fabric (pieces cut on the
fold), several plies, a marker of several garments in the same fabric,
PDF pages longer than 5 m for readers that limit page size (Acrobat).

## User story
As a small brand, I want to know how much fabric my shirt takes and what
it costs before I order: I pick the sizes I will cut, the width of the
fabric, and Rustle3D lays the pieces out on it, tells me the length to
buy, how much of it is used, and the price; I print the marker or send
it to the cutting room.

## Behavior
- **Fabric details** (per garment, `garment.set_fabric_details`): the
  fabric's weight (g/m²; default: the fabric's usual weight, cotton 120,
  denim 400, jersey 180, silk 80, wool 260), the width it is sold in
  (mm; none: 1500 mm in a marker) and its price per metre (any
  currency, none: no cost). The command sets all three as given; an
  omitted one goes back to its default. The weight is also the cloth's
  mass in the drape.
- **Indicators** (`production.indicators`, in one size of the range or
  the pattern as drawn): for each piece its area (sewing line, holes
  left out), its cut area (with its seam allowance), its perimeter and
  the mass of the cut piece (cut area × weight); for each garment the
  totals. Areas in mm², masses in grams.
- **Marker** (`marker.compute`): the pieces of one garment in one or
  several sizes laid out on a strip of fabric of a given width:
  - Every piece of the garment, once per size, by its cutting line (the
    seam allowance included). A linked mirror copy is a piece of its
    own. `quantities` changes how many times a piece is cut (0 leaves
    it out), makes the copies a mirrored pair (every second copy
    turned over), or frees its grain.
  - **Grain**: the fabric's length (x in the marker, along the
    selvedge) is the warp. Each piece is turned so its grainline lies
    along it, then may be laid either way (0° or 180°). A piece with a
    free grain may also lie at every 45°.
  - **Algorithm**: raster bottom-left-fill. The fabric and each piece in
    each direction are rasterised on a grid (cells of `step_mm`, 5 mm
    by default; a cell is taken when the piece touches it, so pieces
    never overlap). The pieces go down largest first; each goes to the
    position, in any of its allowed directions, whose right end is the
    leftmost, then the lowest: the first free place scanning the length
    from the start, so holes left by earlier pieces get filled. The
    result is deterministic.
  - **Results**: the marker's length (the rightmost point of any
    piece), the efficiency (the cloth of the pieces over length ×
    width), the yardage (the length in metres, rounded up to the
    centimetre), the cost (yardage × price per metre) when a price is
    set, and every placed piece (panel, size, copy, rotation, whether
    turned over, its offset; its outlines with `outlines: true`).
  - A piece wider than the fabric in every allowed direction is refused
    (`marker_too_narrow`).
- **Export** (`export.marker`, `r3d marker`): the marker as SVG or PDF
  at 1:1 (one page, for a plotter, or A4/Letter tiles): the fabric's
  edges, each piece's cutting and sewing lines, its grainline and its
  name and size, a line with the width, length and efficiency, and the
  100 mm test square.

## UX
- **Inspector, nothing selected**: each garment card shows its fabric
  details (weight g/m², width cm, price per metre) as fields, and its
  totals: pieces, area of fabric cut (m²), mass (g).
- **Inspector, one piece**: its area, cut area and mass in the
  measurements.
- **Marker…** in the Export menu opens a dialog: the garment (when
  there are several), the sizes (chips, the size shown in 3D picked
  first), the fabric width (cm, from the garment), **Lay out**; the
  layout is drawn to scale (each size in its colour) with the length,
  efficiency, yardage and cost; **Save SVG** and **Save PDF** write it
  where the user chooses.

## Tech pack
- **Content** (`export.techpack`), on landscape A4 or Letter pages,
  each with the project's name, the garments and the size range (with
  the sample size, the one drawn) at the top and the page number at the
  bottom:
  1. **Views**: pictures of the garment side by side, each captioned.
     The CLI and the MCP tool render the front and the back of the
     garment first (`views`: `3d`, the 3D view, default; `photo`, quick
     studio photos; `none`), from the cached drape when it is up to
     date, otherwise where the simulation starts; the app renders them
     from its 3D view as it is (the running drape). `images` adds PNG
     files from the project folder.
  2. **Colourways**: each garment's colour as a swatch, with its name,
     fabric and `#RRGGBB`.
  3. **Bill of materials**: each garment's fabric (name, colour, weight
     g/m², width cm, price per metre), the fabric one garment takes (the
     yardage of its marker in the sample size, on its fabric width or
     150 cm) and its cost; with a size range, the fabric per garment in
     every size; trims (none until M20).
  4. **Measurements**: with a size range and a body, the size chart
     (height, bust, waist, hips of each size's body, the sample size
     marked `*`); every piece in the sample size: length along its
     grainline and width across it (sewing line, cm), sewing line
     length, area and cut area (cm²), seam allowance (mm), mass (g); with
     a range, every piece's length × width in every size.
  5. **Pattern**: every piece (every size nested, with a legend of the
     sizes' colours) scaled to fit the page at an exact 1:N, written
     under it.
  6. **Sewing notes**: every seam (its two pieces, the lengths of its
     sides in mm, its ease, whether it is gathered) and each garment's
     notes.
- **Words**: every heading and column title is a label with a stable
  key and an English default (`LABELS` in `r3d_commands::techpack`;
  `decimal` is the decimal separator). `labels` replaces any of them:
  the app passes its own, in the user's language. The PDF's built-in
  font writes Latin-1; other characters become `?`.
- **Notes** (`garment.set_notes`): free text per garment, up to 20 000
  characters, trailing blank lines dropped; one undo step per field
  edit. In the app, a Notes field in the garment's card.
- Returns the path, the number of pictures and the fabric per garment
  (yardage, cost). A project with no piece is refused; a picture that
  is not a PNG fails with `image_unreadable`.

## Projector
- **What**: the pattern at 1:1 on a projector hung over the cutting
  table, so pieces are traced or cut on the fabric without printing.
- **Where**: Export → Projector… in the app opens a view that fills the
  window (move the window to the projector's screen and maximise it).
  It shows the pieces selected on the board (all when none is), in the
  size shown in 3D, with their cutting and sewing lines, notches,
  internal lines, grainlines and names.
- **Calibration**: the app cannot know the projected size of a pixel.
  **Calibrate** shows a square of 20 cm at the current scale; the user
  measures its two sides on the table with a ruler and types them in
  (cm); the scale becomes `scale × 20 / measured` on each axis (a
  projector's pixels are not always square). The calibration (pixels per
  mm across and down), the mirror, the turn and the contrast are an app
  preference (`projector.json` in the user's library folder), not in
  the project: they belong to the room and the projector. Default: 96
  pixels per inch.
- **Display**: high contrast: white lines on black (the fabric stays
  dark where nothing is drawn), or black on white. Mirror (left–right),
  for a projector shining onto a mirror or fabric seen from the other
  side; turn by 90° steps. A 10 cm scale bar stays in a corner, to
  check the calibration at a glance.
- **Pan**: drag with the mouse, or the arrow keys (1 cm; Shift: 10 cm),
  to bring the part of the pattern under the projector; Home brings the
  pieces back to the middle. Escape closes the view. M mirrors, R
  turns, C calibrates, I inverts the contrast.
- Nothing is changed in the document; agents do not drive it (it is a
  display of the app; they print the same lines with `export.pdf`).

## Acceptance criteria
- `garment.set_fabric_details` has an apply → undo → redo test;
  projects of schema v13 load (migration v13 → v14), and the v14 golden
  file has fabric details.
- The nest never overlaps two pieces, keeps every piece inside the
  fabric's width (property test on random polygons), keeps the
  grainline along the length (0° or 180°) and is identical from run to
  run.
- A shirt graded 36–52, sizes 38, 40 and 42 on a 150 cm fabric: every
  piece placed, efficiency above 60 %, the cost equal to the yardage
  times the price; the SVG and PDF exports are written.
- Indicators: a 100 × 200 mm rectangle with a 10 mm allowance has a cut
  area of 120 × 220 mm² and, in a 200 g/m² fabric, a mass of 5.28 g.
- `garment.set_notes` has an apply → undo → redo test; projects of
  schema v14 load (migration v14 → v15), and the v15 golden file has
  notes.
- The graded shirt's tech pack is written with its pictures, its
  labels replaced where given, the bill of materials, the size chart,
  the notes, on four pages or more; the fabric per garment is priced
  (yardage × price); the CLI writes it without a GPU (`--views none`),
  the MCP tool with rendered views.
- Projector: the view opens from the Export menu, shows the selected
  piece, mirrors, pans with the keys and closes with Escape
  (integration test); calibrating with sides measured 10 % too large
  shrinks the scale by 10 % and is remembered by the next view.
