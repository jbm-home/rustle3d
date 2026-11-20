# 28 — Seamly2D and Valentina patterns

Provenance: maintainer's functional brief (2026-11-20: import Seamly2D/Valentina patterns and measurement files, the pattern run for the body and graded in every size, drafted again for another body; a measurement file gives the body, a multisize one the sizes); the public descriptions of these open XML formats (the XSD schemas and documentation of Valentina 1.2.1 and Seamly2D 0.7.6, two GPL-3.0-or-later programs, read for interoperability only: no code taken, everything implemented from a description in our own words); general garment-making knowledge (drafting from body measurements, notches, grainlines, cutting on the fold); elementary geometry (lines, circles, cubic Béziers, de Casteljau splitting; a circular arc as cubics with handles `4/3 · tan(θ/4)`).

## Status (2026-11-20)

Implemented: patterns (`.val`, `.sm2d`, both dialects, files from 2016
on and most older ones), individual (`.vit`, `.smis`) and multisize
(`.vst`, `.smms`, the old size/height layout and the newer dimensions)
measurement files; the drafting steps listed below; pieces with curves,
seam allowance, notches, grainline, internal paths and fold lines; the
garment kept with its pattern, graded in every size and drafted again
for another body (`garment.redraft`); a measurement file giving the
avatar's body, variables and sizes (`avatar.use_measurements`).

Not yet: the drafting steps `parallelCurve`, `graduatedCurve`,
`ellipticalArcWithLength` and the union of pieces (the pieces that use
them are left out and the import says which steps); seam allowances that
change along the outline (the piece's main width is used), custom seam
allowance paths, labels, pins and place labels; Seamly2D's newest
smoothed splines are read with the handle lengths written in the file.

## User story
As a pattern maker who drafts in Seamly2D or Valentina, I bring my
pattern and its measurement file into Rustle3D: the pieces come in as I
drafted them, for my client's measurements; in a size range each size is
drafted with its own body; when the avatar changes I draft the pattern
again for it, and what I changed in Rustle3D since stays.

## Behavior
- **The files.** A pattern there is a list of drafting steps (a point at
  a length and an angle from another, where two lines meet, a curve
  between two points, a curve cut at a length…) whose lengths and angles
  are formulas of body measurements (`bust_circ`, `waist_circ`…),
  variables of the pattern (`#ease`) and lengths of what was drafted
  before (`Line_A_A1`, `Spl_A1_A5`); then pieces, each a closed list of
  points and curves. Its measurement file is named in the pattern, by a
  path from the pattern's folder.
- **Import** (`import.pattern` on a `.val` or `.sm2d` file inside the
  project folder): its measurement file is the one it names (at that
  path from the pattern, else by its name next to it), or `measurements`;
  a missing one is refused (`import_val_missing_measurements`). Without a
  measurement file the pattern is drafted for the avatar's height, bust,
  waist and hips (`height`, `bust_circ`, `waist_circ`, `hip_circ`). The
  app and `r3d import` copy the pattern and its measurement file into
  `imports/`. The texts go to `pattern.import_parametric`, one undo step.
- **Drafting.** The steps are run in file order with the measurement
  values, in millimetres with y up (the files' screen y points down;
  their angles are counter-clockwise from the right, so they stay
  ordinary angles). Supported: points `single`, `endLine`, `alongLine`,
  `normal`, `bisector`, `shoulder`, `lineIntersect`, `pointOfContact`,
  `height`, `triangle`, `pointOfIntersection`/`intersectXY`,
  `lineIntersectAxis`, `curveIntersectAxis`, `cutArc`, `cutSpline`,
  `cutSplinePath`, `pointOfIntersectionArcs`,
  `pointOfIntersectionCircles`, `pointFromCircleAndTangent`,
  `pointFromArcAndTangent`, `pointOfIntersectionCurves`, `trueDarts`;
  curves `simpleInteractive`, `simple`, `pathInteractive`, `path`,
  `cubicBezier`, `cubicBezierPath`; arcs `simple`, `arcWithLength`;
  elliptical arcs `simple`; operations `rotation`, `flippingByLine`,
  `flippingByAxis`, `moving`. Formulas: `+ - * / ^`, comparisons,
  `c ? a : b`, trigonometry in radians and in degrees (`sinD`),
  `sqrt`, `abs`, `min`, `max`, `sum`, `avg`, `fmod`, `rint`, `r2cm`,
  `_pi`.
- **Errors** carry a code and change nothing: `import_val_malformed`,
  `import_val_unit`, `import_val_formula` (with the formula),
  `import_val_unknown_name` (a measurement missing from the file),
  `import_val_missing_object`, `import_val_construction` (two parallel
  lines that should meet, two circles apart; with the step's id),
  `import_val_no_pieces`. A step of a kind not supported is skipped, and
  so are the steps and pieces that use it: the import lists those kinds
  (`unsupported`) and counts the pieces left out (`ignored`).
- **Pieces.** A curve between two points lying on it runs from one to
  the other; between two curves, from where they cross. Curves stay
  exact (cubics; arcs in pieces of at most 90°). The seam allowance is
  the piece's width (none when the file includes it in the outline);
  notches stand on the points that have one; the grainline's angle comes
  from its two pins or its rotation; internal paths become internal
  lines; a piece with a fold line is unfolded. Pieces drawn over each
  other are laid out side by side, to the right of the board's pieces.
- **The garment.** The pieces make a garment of their own named after
  the file (on an empty board, the project's garment), which keeps the
  pattern's text, its measurement file's, the body it was drafted for
  (none: the measurement file's own values) and which panel is which
  piece (schema v19).
- **Sizes.** In a size range (set before or after), the pattern is run
  for each size's body: the avatar's (or, without one, the measurement
  file's base) changed by the size's differences; each contour point and
  curve handle is graded by how the pattern moves it between the base
  size and that size, matched by its place in the outline. A piece whose
  outline gained or lost points is not graded. Grades set by hand on
  these pieces are replaced when the range changes.
- **Measurements for a body.** An individual file takes the body's
  height, bust, waist and hips in place of its own, and the values
  computed from them follow; its other measurements stay. A multisize
  file is read at the size that fits the body best (least squares over
  those four measurements, the file being linear in each dimension).
- **Drafting again** (`garment.redraft`, a body or the avatar's): the
  pattern is run for the body it was drafted for and for the new one;
  each point and curve handle moves by the difference, so changes made
  to the pieces since stay; internal lines and pins follow the points
  around them (inverse squared distance), notches keep their place along
  their edge. A piece whose outline gained or lost points is left as it
  is. The garment remembers the new body and is graded again.
- **Measurement files** (`import.measurements`, then
  `avatar.use_measurements`, one undo step): the avatar takes the file's
  height, bust, waist and hips (refused when one is missing,
  `invalid.missing_measurement`) and its gender as masculinity; each
  other measurement becomes a variable for formulas (lengths in mm,
  `@custom` without the `@`); a multisize file's sizes along its chest
  (else first) dimension become the size range, its base size the base
  (an old-layout file, which does not say which sizes it covers: two
  sizes each side of the base).

## UX
- **Import** in the top bar also offers `.sm2d`, `.val`, `.vit`, `.vst`,
  `.smis` and `.smms`. A pattern says how many pieces came in and, if
  some were left out, that some drafting steps are not supported yet; a
  measurement file says the avatar took its measurements.
- In the inspector's garment list, a garment imported this way reads
  "Drafted from skirt.val" and has **Draft again for the avatar**.

## Acceptance criteria
- A quarter skirt written by hand (`tests/data/parametric/`): imported
  with its measurement file, it is the skirt drafted for those
  measurements (waist edge 185 mm, 250 × 580 mm), with its seam
  allowance, notch, grainline and hip line; the piece using an
  unsupported step is reported.
- Graded 36–44 on the women's chart, each size is 250 mm plus a quarter
  of its hips' difference from the base size's.
- Drafted again for waist 80 and hips 104 cm: waist edge 210 mm, width
  270 mm, the hip line following; one undo step back.
- A multisize file gives the avatar its base body, sizes 40–48 based on
  44, and its other measurements as variables.
- Every new command has an apply → undo → redo test; files of schema v18
  open (migration), the v19 golden file has a garment with its source.
