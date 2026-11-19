# 07 — Import / export

Provenance: maintainer's functional brief (2026-11-14; M13 roadmap 2026-11-16); open specifications: Wavefront OBJ, Khronos glTF 2.0, W3C SVG 1.1, ISO 32000 (PDF), AAMA DXF / ASTM D6673 (layer numbers and piece texts), AutoCAD DXF R12 ASCII group codes; curve fitting from P. J. Schneider, "An Algorithm for Automatically Fitting Digitized Curves", Graphics Gems (1990); general pattern-making knowledge (cutting vs sewing line, notches, grainline arrows, test squares).

## User story
As a user, I bring in an avatar and patterns drawn in other tools, and I
take out my draped garment and my patterns to use in other tools or to
print and cut.

## Behavior
- **Import avatar** (M31, spec 27): OBJ, glTF with embedded buffers, or
  GLB. Units, up axis and facing are detected and can be corrected; the
  mesh stands on the ground, centred, and is checked (holes and
  non-manifold edges produce a warning, not a refusal).
- **Export garment**: OBJ or glTF/GLB of the draped mesh (or the arranged
  state if not simulated), with normals and UVs from the 2D pattern along
  each piece's grainline (in repeats of the garment's print, or 1 UV unit
  = 1 m of fabric without one; spec 25), one mesh or node per panel, and
  optionally the avatar. The print travels with it: embedded in the GLB
  as the base colour texture (repeating sampler), or for OBJ an MTL file
  beside it (`Kd`, `map_Kd`) with the image files.
- **Export patterns**:
  - SVG at 1:1 scale (mm user units), with panels, internal lines, notches,
    grainlines and labels.
  - PDF at 1:1 scale, either as one large page (plotter) or **tiled** to
    A4/Letter with overlap margins, alignment marks, page grid labels and a
    100 mm test square on the first page.
  - Optional seam allowance (constant width per panel): the cutting line is
    the sewing line offset outward, and the sewing line is drawn dashed.
  - Notches are 6 mm ticks cut inward from the cutting line.
  - Panel names are drawn beside the grainline. PDF text uses the built-in
    Helvetica (Latin-1); other characters print as `?` until we embed a
    font.
  - **DXF-AAMA** (ASTM D6673), ASCII DXF R12 in millimetres, for pattern
    and cutting software. One block per piece, inserted at its board
    position. Layers: 1 boundary (the cutting line) and piece texts
    (`Piece Name: …`, `Quantity: 1`), 2 turn points and 3 curve points
    (only when the boundary is the sewing line, i.e. no seam allowance),
    4 notches (points on the cutting line), 7 grainline, 8 internal lines,
    14 sewing line (when there is a seam allowance). Curves become
    polylines within 0.05 mm. Units: `$INSUNITS` = 4 and `Units: METRIC`.
  - The grainline is drawn through the middle of the piece and shortened
    until it lies inside the outline.
  - **Sizes** (M22, spec 18): SVG, PDF and DXF take `sizes`, names from
    the project's range: one size alone, or a graded nest (the base size
    in full, the others' cutting lines each in a colour, a legend beside
    the test square; in DXF every piece once per size, block `P<n>-<size>`
    with a `Size: …` text). The app exports a graded project's nest of
    every size.
- **Import patterns** (`import.pattern`, one undo step through the
  `pattern.import` command): DXF-AAMA or plain DXF, SVG, PDF.
  - The file must be inside the project folder; the app and `r3d import`
    copy the chosen file into `imports/` first (same name reused for the
    same content, numbered otherwise), so the project keeps its sources.
  - **Units**: DXF from `$INSUNITS`, else a `Units: METRIC|ENGLISH` text;
    SVG from the root `width` in a physical unit with its `viewBox`; PDF
    points (1/72 in, times `UserUnit`). When the file says nothing, DXF is
    read as mm and SVG as CSS pixels (1/96 in), and the result says
    `"scale": "assumed"` so the user can check. `units` overrides what the
    file says; `scale` is a size factor on top (a pattern drawn at half
    size: 2).
  - **DXF-AAMA**: one piece per boundary (layer 1) in each inserted block;
    a closed sewing line (14) at an even distance makes it the outline and
    sets the seam allowance; turn points (2) split the outline; notches
    (4, 80–83: points or ticks) go to the nearest outline point; grainline
    (7); internal lines (8) and cutouts (11); `Piece Name:` text. Pieces
    that overlap (blocks all drawn at the origin) are laid out side by
    side. Bulges, arcs and circles are sampled. Binary DXF is refused.
  - **SVG, PDF, and DXF without AAMA layers** are plain drawings, read with
    pattern-drawing rules: an outermost closed shape is a piece (its
    cutting line); a closed shape inside it at an even distance (80 % of
    samples within 0.5 mm or 5 %, at most 50 mm) is its sewing line and
    gives the seam allowance; a straight tick of at most 15 mm touching
    the outline is a notch; the longest straight line with arrowheads is
    the grainline; other lines and closed shapes entirely inside are
    internal lines; the largest text inside is the name. A 25.4, 50, 50.8
    or 100 mm square holding only its caption (a test square) and closed
    shapes covering most of a PDF page (frames) are skipped. Shapes outside
    every piece are counted in `ignored`.
  - **Tiled PDF**: when every page repeats one drawing, shifted (as
    Rustle3D and most pattern programs tile), the pages are put back
    together by finding the shift that lays each page onto the first;
    page frames, guides and labels, which do not move with the pattern,
    drop out. Other multi-page files are refused with
    `import_pdf_pages_differ`; `page` imports one page.
  - **Curves**: outlines drawn as polylines are fitted back to straight
    edges and cubic curves within 0.1 mm (corners: a turn above 40°, or
    above 8° and 2.5 times its neighbours'), so pieces are as easy to edit
    as drawn ones. A fitted outline that would cross itself falls back to
    the polyline; one that crosses itself anyway is skipped.
  - Imported pieces are placed to the right of the pieces already on the
    board (`pattern.import` takes `keep_position` for exact coordinates).
- All export paths stay inside the project folder (`exports/` by default)
  unless the user chooses a location in a save dialog.

## UX
- One "Export" menu with clear presets ("Print at home (A4 tiles)", "Plotter",
  "3D (glTF)"), and a preview of the tiling before saving.
- "Import pattern…" in the same bar opens a file dialog (DXF, SVG, PDF);
  the result is announced (pieces added, shapes left out), and a scale
  that was assumed is flagged so the user can undo and import again with
  the right units.

## Acceptance criteria
- Round-trip: export glTF → re-import as a mesh → same vertex count and
  positions within 1e-5 m.
- The test square measures 100 mm in the exported PDF and SVG (checked by
  parsing the output).
- Exported glTF passes the Khronos glTF validator (CI step).
- Every template, exported as SVG, PDF (one page and A4 tiles) and DXF,
  imports back with the same pieces: names, areas and perimeters within
  0.2 %, seam allowances within 0.15 mm, notch and internal-line counts,
  grainline angles (`crates/r3d-commands/tests/interchange.rs`).
- Reference files drawn by hand (`crates/r3d-commands/tests/data/`: an
  AAMA DXF in inches with blocks at the origin, an Inkscape-style SVG with
  transforms and relative paths) import with the expected pieces,
  allowances, notches, darts and grainlines.
