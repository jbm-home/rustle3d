# 19 — Sketching: constraints, dimensions and symmetry

Provenance: maintainer's functional brief (ROADMAP M27, 2026-11-18: drafting lacks ease; general CAD ideas in our own UI and terms); public papers: Ben-Israel 1966 (minimum-norm Newton steps for underdetermined systems), Levenberg 1944 and Marquardt 1963 (damped least squares), Light & Gossard 1982 (variational geometry), Golub & Van Loan (rank by orthogonalisation), Riškus 2006 (circular arcs as cubic Béziers); general garment-making knowledge (half pieces cut on the fold, circle skirts from the waist and the length).

## Status (2026-11-18, M27)

Implemented: constraints and dimensions on contour points and edges
(`constraint.add`, `constraint.remove`, `constraint.set_value`,
`constraint.list`), solved after every step in the same undo step;
smooth and corner points (`point.set_smooth`); symmetric pieces
(`panel.unfold`, the `symmetric` option of `panel.create_polygon` and
`panel.create_path`, `panel.unlink`); construction lines (`guide.add`,
`guide.move`, `guide.remove`, `guide.list`); ellipses
(`panel.create_ellipse`), arc sectors (`panel.create_sector`) and
circular arcs on an edge (`edge.set_arc`). Project files: schema v11.
The app: the constraints of the selected piece over the board and in the
inspector, constraint chips for what is picked (Ctrl-click, ⌘ on macOS, or ⇧-click a second corner
or edge), the Half piece option of Polygon and Curve, the Construction
line tool (G), the Shapes group (rectangle, ellipse, circle skirt), the
Arc tool, snapping with a hint while drawing and dragging corners.

Not yet: constraints on internal lines and notches, construction lines
in constraints (a point on a construction line), moving a dimension's
label by hand, snapping to the intersection of two lines, curved
symmetry axes.

Since M32 (2026-11-18): the Dimension tool (T) puts a dimension on the
board: an edge's length, the distance between two corners (direct,
width or height, from where it is pulled out: above or below both
corners, a width; beside both, a height) or the angle between two
edges, measured as they are (`measure`), then its value (or a formula,
spec 20) typed in a field where it was placed.

## User story
As a designer, I draft a half bodice against its centre front with the
measurements that matter fixed (bust width, waist, shoulder length);
when I change one, the piece and its other half are redrawn and every
other measurement stays. I draw against construction lines, and points
land where I aim: on a corner, the middle of an edge, level with the
previous point.

## Behavior
- **Constraints** relate points and edges: `fixed` (the solver never
  moves the point), `coincident` (two points at one place, usually on two
  pieces), `horizontal` and `vertical` (an edge's ends level or one above
  the other), `parallel` and `perpendicular` (two edges' chords),
  `tangent` (a smooth point: the two edges meeting there leave it in
  opposite directions), `equal_length`, `on_edge` (a point on an edge, on
  its line extended for a straight one).
- **Dimensions** carry a value: `distance` between two points (straight,
  or across or up and down with `along`: `horizontal`, `vertical`),
  `length` of an edge along its curve, `angle` between two edges (at
  their common point, else between their chords, 0–180°). Added with
  `measure: true`, a dimension keeps the value the drawing has.
- **Solving.** After every step the drawing is solved so that every
  constraint holds: Gauss–Newton with the smallest weighted step
  (Levenberg–Marquardt damping), so the geometry moves as little as
  possible. What the step moved itself (the point dragged, a handle) is
  held first, then released but stiff if the constraints cannot hold
  otherwise. Constraints sharing no point are solved apart: one piece
  never moves another unless a constraint links them. Variables are the
  points the constraints name and, at smooth points, the handles of the
  two curves (as offsets from their ends, so handles follow their points
  as with `point.move`).
- **Never a broken drawing.** A step whose constraints cannot all hold is
  refused with `constraints_unsolvable` and the constraints concerned;
  nothing changes. A constraint repeating another the same way is
  refused (`invalid.duplicate`). Constraints whose points or edges go
  away go with them; splitting an edge (`edge.split`, an arc over 90°) or
  joining two (`point.remove`) removes the constraints on those edges.
- **Status.** `constraint.list` gives each constraint with its piece, a
  dimension's value and what the drawing measures, whether it holds, and
  `marks` (where to draw it); and per group of constraints sharing points
  the degrees of freedom left (moving or turning a piece as a whole
  included, unless a point is fixed) and the `redundant` ones (they
  repeat what others say: over-constrained, though consistent).
- **Symmetric pieces.** `panel.unfold` unfolds a half piece across one of
  its straight edges (the axis, e.g. a centre front on the fold), which
  disappears; drawing with `symmetric` does it with the closing edge. The
  contour is the drawn half, from one axis point to the other, then its
  reflection. The reflection follows every change (points, curves,
  grades, notches) and keeps its IDs, so seams and notches on it stay;
  the axis points stay on the axis. It cannot be edited or constrained
  (`derived`); `panel.unlink` makes the halves independent. A symmetric
  piece cannot be cut until then. A linked mirror copy of a symmetric
  piece copies the whole piece.
- **Construction lines** are lines through two board points, drawn
  across the board, snapped to; never cut, sewn, simulated or exported.
- **Shapes.** An ellipse (four arcs), an arc sector given the inner arc's
  length (the waist the piece covers), the length to the outer arc and
  the angle (a half circle is 180°), hanging below its centre; an edge
  made a circular arc bulging a given distance (outward when positive),
  split into arcs of at most 90°.
- **Undo.** Every constraint edit and every solve is part of one undo
  step with the edit that caused it.

## UX
- The board shows the selected piece's constraints: a small sign at each
  point or edge they name (red when one does not hold), dimensions with
  their value, a dashed line for a distance. Symmetry axes are drawn
  dash-dotted, construction lines dashed across the board.
- Picking: click a corner or an edge; ⌘-click (Ctrl-click on Windows
  and Linux) or ⇧-click adds a second one (two at most). The inspector then offers the constraints that fit what is
  picked, and lists the piece's constraints with their values (typed, in
  cm or degrees) and a remove button, and the degrees of freedom left.
- Drawing tools snap within a few pixels to corners, middles of edges,
  edges, construction lines and axes, and level with the previous point
  (or the first one), with a hint naming what was snapped to; a dragged
  corner snaps to the same, level with its neighbours.
- Delete with a single corner picked removes the point; with a
  construction line picked, the line.

## Acceptance criteria
- A half bodice drawn against an axis, with a fixed centre front waist,
  a horizontal waist and a bust width: changing the bust width redraws
  both halves, the centre front and the waist stay (test
  `a_half_bodice_with_dimensions_redraws_both_halves`).
- A dragged corner stays where it is dropped; the other end of a
  horizontal, fixed-length edge follows.
- Contradicting constraints are refused and the document is unchanged.
- A circle skirt sector for a 360 mm waist over 180° has a 360 mm waist
  arc (±0.2 mm) and a hem of π(r + length).
- Every new command has an apply → undo → redo test; files of schema v10
  open (migration) and v11 golden files load.
