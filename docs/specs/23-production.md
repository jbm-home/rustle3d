# 23 — Production: indicators, marker, yardage and cost

Provenance: maintainer's functional brief (ROADMAP M30, 2026-11-18: indicators per piece, a marker of one or several sizes on a given fabric width with the grainline respected, yardage and cost from a fabric price, an algorithm from the open literature); public papers: S. Jakobs, "On genetic algorithms for the packing of polygons", European Journal of Operational Research 88 (1996) 165–181 (the bottom-left placement rule); E. K. Burke, R. S. R. Hellier, G. Kendall, G. Whitwell, "A new bottom-left-fill heuristic algorithm for the two-dimensional irregular packing problem", Operations Research 54(3) (2006) 587–601 (bottom-left-fill: holes left behind are filled); J. A. Bennell, J. F. Oliveira, "The geometry of nesting problems: a tutorial", European Journal of Operational Research 184 (2008) 397–415 (raster representations of pieces and their overlap test); J. F. Oliveira, J. S. Ferreira, "Algorithms for nesting problems", Applied Simulated Annealing, Springer LNEMS 396 (1993) 255–273 (raster nesting); general garment-making knowledge (markers laid along the selvedge with the grainline parallel to it, fabric sold by the running metre at a given width, weights in g/m², marker efficiency as the share of the fabric used).

## Status (2026-11-18, M30 first half)

Implemented: fabric details on each garment (weight, width, price per
metre; the weight also drives the drape), indicators per piece and per
garment (`production.indicators`), the marker (`marker.compute`),
its SVG/PDF export (`export.marker`, `r3d marker`), the app's
indicators in the inspector and its Marker dialog.

Not yet (M30, next): the tech pack PDF, the projector export. Later:
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
