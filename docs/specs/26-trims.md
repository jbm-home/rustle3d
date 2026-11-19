# 26 — Trims: buttons and zippers

Provenance: maintainer's functional brief (ROADMAP M20: buttons and buttonholes, zippers, placed on the pattern and simulated as closures, opened and closed in 3D; a shirt that buttons and trousers with a fly zipper, draped closed; trims in the tech pack's bill of materials); general garment-making knowledge (a buttonhole is the button's diameter plus about its thickness long, cut across the band or the cuff from just past the centre line inward; buttons sit on the centre line of the under side, the buttonholes on the overlapping side; a fly zipper runs from the waist down the centre front, the crotch seam closed below it); XPBD (Macklin, Müller, Chentanez, MIG 2016) for the closures' constraints, as for seams (docs/physics.md §3).

## Status (2026-11-19, M20)

Implemented in step 1: buttons and zippers in the document (schema v17),
their commands, `trim.list`, the board (buttons, buttonholes, zippers),
the tech pack's bill of materials.

## User story
As a designer, I put five buttons down the front of a shirt and two on
its cuffs, a zipper in the fly of trousers, drape them closed, then open
the top button or the zipper halfway and see the garment open, without
starting the drape again. The tech pack lists 7 buttons of 11 mm and one
18 cm zipper.

## Behavior

### Buttons
- A **button** is sewn on a piece at a point (mm) and has a diameter
  (default 11 mm, up to 80 mm). It always comes with its **buttonhole**:
  a point on a piece (often the overlapping piece; the same piece for a
  cuff that wraps) where the button sits once buttoned, and the slit's
  direction from there. The slit is the diameter plus 3 mm long.
  Omitted, the slit runs across the piece towards its middle (from the
  edge inward, as on a front band).
- Both points must lie inside their pieces (not in a hole).
- A button is **buttoned** (closed, the default) or not. Buttoned, the
  drape holds the button point and the buttonhole point together (a
  stiff zero-length tie). Unbuttoned, nothing holds them.
- A button follows its pieces: moved with them, cut with the half it is
  on, removed with them; in other sizes it moves with the points around
  it (as pins do, spec 18).

### Zippers
- A **zipper** joins two sides, given as a seam's (a whole edge,
  consecutive outline edges, or an internal line; maybe a stretch of
  it). Both sides start at the zipper's **bottom stop**; its length is
  the shorter side's.
- **Open length** (`open_mm`): how much is open from the top, the slider
  that far below the top end. 0 is closed (the default), the length or
  more is open all the way. Measured from the top so a closed zipper
  stays closed when its sides change length.
- Closed, a stretch drapes as a seam (point for point, like a seam's
  sides). Above the slider, nothing joins the sides.
- A stretch sewn by a seam cannot be zipped and the other way round
  (`sewn`, naming the seam or zipper).
- A zipper whose sides no longer exist is removed with them.

### Drape
- See step 2 (docs/physics.md, "Closures").

### Board
- Buttons drawn at their real size (a ring and a dot), buttonholes as
  slits with a bar tack at each end, fainter when unbuttoned. Zippers as
  a brass band along each side, solid where closed, dashed where open,
  with a bar at the bottom stop.

### Tech pack
- The bill of materials lists the trims per garment: buttons grouped by
  diameter, zippers by length (rounded up to the centimetre), with how
  many and the pieces they go on. A project without trims says so.

## Commands and queries
- `button.add` (panel, at, diameter_mm?, hole_panel, hole_at,
  hole_angle_deg?, closed?), `button.set` (move or resize; merges while
  dragged), `button.set_closed` (one or several buttons),
  `button.remove`.
- `zipper.add` (a, b as seam sides, open_mm?), `zipper.set_open` (merges
  while dragged), `zipper.remove`.
- `trim.list`: every button (with its slit's ends) and zipper (length,
  open length, share closed).

## Acceptance criteria
- Every command round-trips apply → undo → redo; the golden file v17
  has a button and a zipper; v16 files load unchanged.
- A button outside its piece is refused (`invalid.outside_piece`); a
  zipper on a sewn stretch is refused (`sewn`).
- Deleting a piece removes its buttons in the same undo step.
