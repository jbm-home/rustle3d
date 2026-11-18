# 18 — Grading and sizes

Provenance: maintainer's functional brief (ROADMAP M22, 2026-11-17; a shirt graded from 36 to 52, each size draped on a matching avatar); general garment-making knowledge (grade rules as offsets of key points per size, graded nests, European size charts); open standards: DXF-AAMA (ASTM D6673, `Size:` piece text).

## Status (2026-11-18)

Implemented: a size range on the project (`grading.set_sizes`, or a
built-in chart with `grading.use_chart`), grade rules on points
(`point.set_grade`: a step per size or offsets in some sizes), templates
graded to every size, the size draped and shown in 3D on its own body
(`grading.show_size`), the graded nest on the board and in the SVG, PDF
and DXF-AAMA exports, `grading.check` for seams that stop matching in a
size. The app: a Sizes section in the inspector (chart, range, base,
size shown in 3D), a grade step on a selected point. Project files:
schema v10 (`grading`, `grade` on points and curved edges, a garment's
`template`).

Not yet: grade rules read from imported files (they come in at the size
drawn), an ASTM rule table (`.rul`) next to the DXF, notches graded on
their own (they keep their distance from the edge's start), editing a
graded curve's handles per size in the app.

## User story
As a designer, I draft a shirt in size 40, grade it from 36 to 52, look
at each size draped on a body with that size's measurements, check that
the seams still match in every size, and print or send the graded nest.

## Behavior
- **Size range.** A project may have a size range: two or more sizes,
  smallest first, each with a name ("38", "M") and how its body differs
  from the base size's (height, bust, waist and hips, cm). The base size
  is the one the pattern is drawn in: the board, the pattern commands
  and the project's avatar are in the base size.
- **Charts.** Built-in European charts (general garment-making
  knowledge, approximate): women 32–56 (bust 76–134 cm, +4 cm per size
  up to 46, then +6; waist = bust − 18, hips = bust + 8), men 44–60
  (chest = 2 × size, waist = chest − 12, hips = chest + 4), every second
  size. A chart's sizes differ from the avatar as they differ from the
  chart's base size, so a custom avatar keeps its own proportions;
  without an avatar the chart's base body becomes it.
- **Grade rules.** Each contour point may have a grade: its offset from
  its base position in each size (mm), as in a grade rule table. A point
  without one is the same in every size. A rule is set as one step
  (the same move from each size to the next) or as offsets in some sizes,
  the others inter- or extrapolated linearly.
- **What follows the points.** A curve's control points keep their
  direction from their end point and scale with the edge's chord (so the
  angles at corners stay), unless the edge has a grade of its own for
  them (templates set one, so their curves are exact in every size).
  Internal lines and pins move like the piece's points around them
  (inverse squared distance). Notches keep their distance along their
  edge. Linked mirror copies take their source's grades, mirrored.
- **Templates.** A garment remembers the template it was drafted from
  and the measurements given (`Garment.template`). Added to a project
  with a size range, or when a range is set on a project whose template
  garments are not graded yet, the template is drafted for every size's
  body (each on a project of its own, in parallel) and the garment's
  points and curves graded by drafting order and contour position: each
  point moves as it does between the base size's draft and that size's,
  so edits to the pieces are kept. Body measurements given to the
  template (chest, waist, hips, as the start screen's form gives them)
  are the base size's and change with each size's body; lengths and
  ease stay as given and an unedited piece is exactly the
  template drafted for that body. A garment with a piece added or
  deleted is not graded; a piece whose outline gained or lost points is
  skipped. The
  pieces of every size share the draft's origin (a centre line on most
  pieces); added to a ranged project, each piece takes the room of its
  largest size on the board, so the sizes nest without touching the next
  piece (graded later, the pieces stay where they are).
- **Changing the range.** Points keep their grade in the sizes that stay
  (by name); new sizes are inter- or extrapolated. A new base size
  redraws the pattern in that size (points, curves, internal lines,
  pins, and the avatar takes that size's body).
- **New points.** A point added on an edge (split, cut) takes the grade
  interpolated between the edge's ends; a point inside a piece (a cut
  line) moves like the points around it. Changing an edge's curve
  removes its handles' own grade.
- **Shown size.** One size of the range is shown in 3D: the drape, the
  3D view, photos and 3D exports use the pattern and the avatar in that
  size. Clicks on the draped garment (drawing on it, pins, picking an
  edge) are brought back to the base size. Changing it starts the drape
  again.
- **Checks.** `grading.check` lists, for each size, the seams (not
  gathered) whose sides differ by more than a tolerance (1 mm by
  default) and the pieces whose outline crosses itself.
- **Exports.** SVG, PDF and DXF-AAMA take a list of sizes: one size
  alone, or several nested (a graded nest). The nest draws the base
  size in full (cutting and sewing lines, notches, internal lines,
  grainline, name) and the other sizes' cutting lines in colour, with a
  legend beside the test square; the DXF has every piece once per size,
  each a block with a `Size:` text. The app's pattern exports of a graded
  project are the nest of every size.

## UX
- With nothing selected, the inspector's **Sizes** section offers **Add a
  size range**: a dialog with the chart (women or men), the smallest and
  largest sizes and the base size. Once set: the sizes as chips (the base
  one marked), **Shown in 3D** (a menu of the sizes), **Change** (the
  same dialog) and a delete button.
- The board draws the other sizes' outlines around each piece, faint;
  the size shown in 3D a little stronger.
- With one point of a piece selected (not on a linked mirror copy):
  **Grade per size**, x and y in mm, the step from each size to the next.
  When the point's grade is not one step (set by a template or an
  agent), a note says so; a step typed replaces it.

## Acceptance criteria
- Every new command has an apply → undo → redo test; projects of schema
  v9 load (migration v9 → v10), and the v10 golden file exercises a
  graded point, a graded curve and a range.
- A shirt graded from 36 to 52 from the template: in every size, each
  piece is the template drafted for that size's body (points and curves
  within 1 µm, up to where the piece sits on the board), every seam
  matches (`grading.check` ok), no piece crosses itself.
- Each size drapes on its own body (`grading.show_size`, then simulate).
- The graded nest exports to SVG and PDF (one colour per size, a legend)
  and to DXF-AAMA (`Size:` per block).
