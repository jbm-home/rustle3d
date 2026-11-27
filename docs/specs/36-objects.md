# 36 — Objects: projects without a body

Provenance: maintainer's functional brief (2026-11-27: pattern without a
mannequin; choose the type of project when it is created; some patterns
cannot be worn; the 3D view must then show something that is not
crumpled; modern UX rules); pressure on a closed cloth surface as in
inflatable and plush-toy design (Mori and Igarashi 2007, "Plushie: an
interactive design system for plush toys", SIGGRAPH; Müller et al. 2007,
"Position Based Dynamics", §4.4 cloth balloons); our own XPBD solver
(docs/physics.md, "Objects").

## Status (2026-11-28, M46)

Implemented: everything below.

Not yet: an object resting on a table (gravity with a floor, for a bag
standing on its base); stuffing set per garment (one setting per
project); bags made of several parts stuffed apart.

## User story
As a designer of bags, cushions and toys, I start a project as an
object: no mannequin appears, and once sewn my cushion shows in 3D as a
smooth, round cushion, my tote bag as a flat, smooth bag, instead of
cloth fallen in folds.

## Behavior
- **Two kinds of project.** A garment is worn: it has a body and drapes
  on it under gravity, as before. An object has no body (`object` in the
  document, schema v30): its avatar and measurements are removed, a body
  cannot be set (`invalid.object`), size charts cannot be used (they
  bring a body); custom size ranges still can.
- **Weightless drape.** An object's cloth has no weight and no floor: it
  keeps the shape its seams give it, where its pieces were laid. It moves
  as if 30 times heavier and is damped five times more (shown at rest,
  not hanging; its shape at rest does not depend on its mass), and it
  keeps no drift nor spin of its own (docs/physics.md, "Objects").
- **Stuffing** (0 none – 1 firm, 0.5 for a new object): the parts
  closed all around (every edge sewn or fastened) are pushed out from
  inside by a pressure, up to 400 Pa, as stuffing does; open parts (a
  bag's mouth) never are, they would drift off. A half-stuffed 40 cm
  cotton cushion swells about 15 cm thick.
- **Templates**: a cushion (two rectangles sewn all around, width and
  length) makes an object project; a tote bag can be either.
- **Camera**: an object is framed on its cloth (app, CLI renders, MCP),
  not on a standing body.

## Commands
- `object.set {stuffing?}`: make the project an object (the avatar goes,
  one undo step) or change its stuffing; out of 0–1 is `invalid.range`.
- `object.clear {}`: a garment again (then `avatar.set`).
- `project.summary` gives `object` (`{stuffing}`, or null).

## UX
- The new-project window (a blank project or a template) shows the
  choice as two cards side by side, **Garment** (worn: draped on a body)
  and **Object** (no body: a bag, a cushion, an accessory), when both
  make sense; the window then shows the body to choose, or the stuffing
  slider (None – Firm).
- In an object project the top bar has an **Object** button where the
  Body menu was: the stuffing slider (applied when let go) and **Wear it
  on a body**, which brings back the standard body (one undo step).
- The Body menu of a garment project has **Make it an object (no
  body)**; a message says what changed, and undo brings the body back.

## Acceptance criteria
- A blank project created as an object opens without a body; the Object
  button stands where the Body menu was.
- A cushion stuffed at 0.5 drapes round: its middle at least 5 cm
  thicker than unstuffed, no crease over 20° between two triangles away
  from its seams, seams closed, its centre within 1 cm of where it
  started; unstuffed, as smooth.
- A tote bag as an object hangs flat and smooth from its pins, and does
  not turn about them.
- `avatar.set` on an object is refused; `object.clear` then `avatar.set`
  works; undo brings the object back.
- Files of schema v29 open; v30 golden files (with and without an
  object) load.
