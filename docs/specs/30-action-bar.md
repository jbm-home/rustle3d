# 30 — Sketching on the board: the action bar

Provenance: maintainer's functional brief (ROADMAP M35, 2026-11-23:
drafting in 2D is not practical; context buttons for the constraints and
actions that fit what is selected, as mechanical CAD tools do; our own
layout, icons and terms) and decisions on the plan (2026-11-24: no
separate sketch mode, the board already is one and every piece can be
sketched; the context buttons are a horizontal bar like the tool rail,
on the row above the 2D and 3D views where the guided flow's banner was;
dimensions can be named and used in formulas); a walk-through of the app
drafting a half bodice and its sleeve (what got in the way: constraints
only in the inspector, labels piled up and dragging one reshaped the
piece, no dimension on a notch or a construction line, picking stopped
at two); general CAD ideas (actions offered for what is picked, a
dimension typed where it is drawn); general garment-making knowledge
(drafting from a frame of construction lines: bust, waist, hip lines).

## Status (2026-11-24, M35)

Implemented: everything below (schema v21). Not yet: constraints on internal lines (their
points have no identity yet), a notch as the end of a straight distance,
a construction line moved by its constraints (lines are references the
points follow), single-letter keys for the bar's actions; a named
dimension without a formula gives formulas the drawn size's value in
every size of a range.

## User story
As a designer, I pick two corners of my bodice and the bar above the
board offers Width first: one click, the value field opens on the
dimension, I type 24 and Enter. I pick the armhole and click Length,
name it `armhole`; on the sleeve I give the cap the formula
`armhole + 1.5 cm`, and the cap follows when I reshape the armhole.

## Behavior
- **Picking** on the board: corners, edges, notches and construction
  lines, on any piece, as many as wanted: a click picks one (and the
  piece), ⌘-click (Ctrl-click on Windows and Linux) or ⇧-click adds or
  takes one away; a click on empty board clears.
- **The action bar** sits above the 2D and 3D views, from the tool
  rail's edge to the window's: what is picked on its left ("2 corners"),
  then buttons drawn like the rail's (icon, tip with the name and what it
  does), in groups set apart by a line. Nothing picked: a short hint.
  What it offers:

  | Picked | Dimensions | Constraints and actions |
  |---|---|---|
  | a corner | — | Fixed, Smooth / Corner, Remove point |
  | two corners | Width, Height, Distance | Joined |
  | an edge | Length | Horizontal, Vertical |
  | two edges | Angle | Same length, Parallel, Square |
  | three or more edges | — | Same length, Parallel (all to the first) |
  | a corner and an edge | — | On the edge |
  | a notch | From the start, From the end | Remove notch |
  | a corner and a construction line | Distance to the line | On the line |
  | a construction line | — | Level, Upright, Remove line |

  A dimension is added with the value the drawing has, and its value
  field opens at once on the board where its label is, the value
  selected: type a number (cm, or ° for an angle) or a formula, Enter;
  Esc keeps the measured value. Each button is one undo step.
- **Labels.** A dimension's label can be dragged: only the label moves
  (the piece does not), and it stays there (`constraint.place_label`,
  an offset from where it is drawn by default). A click on a label opens
  its value field. The dimensions of every piece are shown.
- **Named dimensions.** A dimension can be given a name
  (`constraint.rename`); formulas then use it like a variable: its
  formula when it has one (so it grades), else its value. A name follows
  the variables' rules and is unique among the variables and the
  dimensions. The inspector lists the selected piece's dimensions with
  their names.
- **Notches.** A notch's distance along its edge, from the edge's start
  or end, is a dimension (`notch_distance`): the notch keeps it when the
  edge changes (a value past the edge's end puts it at the end).
- **Construction lines in constraints.** A point on a construction line
  (`on_guide`) and a point's distance from one (`guide_distance`, on the
  side it is). Lines are references: moving a line moves the points
  constrained to it; the solver never moves a line. Level and Upright
  turn a picked line about its first point.
- **Crossing snap.** Drawing or dragging, the crossing of two
  construction lines, of a line and an edge, or of two edges is a snap
  target ("Crossing"). The point is placed there; it stays there only
  through constraints (On the line, On the edge) added after.

## Commands
- `constraint.add` takes the new kinds: `notch_distance {notch, from:
  start|end, mm}`, `on_guide {point, guide}`, `guide_distance {point,
  guide, mm}`.
- `constraint.place_label {constraint, offset: [x, y] | null}`: where a
  dimension's label sits, mm from its default place (null: back there).
- `constraint.rename {constraint, name | null}`: refused when not a
  valid name (`formula.bad_name`) or used (`invalid.duplicate`).
- `constraint.list` gives each dimension's `name` and `label`, and
  a notch's or a line's marks.

## UX
- The bar is one row; a picked edge's group (Length, Horizontal,
  Vertical) and the piece's own constraints chips leave the inspector.
- The inspector's list of constraints names each dimension by its name,
  or what it measures ("Length", "Width"), with its value, formula and
  a rename action.

## Acceptance criteria
- Two corners picked: the bar offers Width, Height, Distance, Joined;
  Width adds a width with the drawn value and opens its field focused;
  typing 24 and Enter makes it 24 cm (one undo step each).
- Three edges picked: Same length makes the three equal.
- Dragging a label moves only the label; the piece's points stay.
- `armhole` named on a length; a sleeve cap length with formula
  `armhole + 1.5 cm` follows when the armhole's value changes.
- A notch 8 cm from the end of an edge stays 8 cm from that end when the
  edge is lengthened.
- A corner 25 cm from a level construction line follows the line when
  it is moved.
- Drawing a polygon corner on the crossing of two construction lines
  snaps there ("Crossing").
- Files of schema v20 open (migration); v21 golden files load.
