# Rustle3D user guide

<!--
Source of the user documentation on rustle3d.com. Written for people who
use the app, not for developers: what it does and how to do it, in plain
words. Keep it in step with the app: every user-visible change (a tool, a
shortcut, a menu, a format, a limit) updates this file in the same
commit (CLAUDE.md §5). Shortcuts are written with the macOS keys; on
Windows and Linux ⌘ is Ctrl and ⌥ is Alt.
Last updated: 2026-11-19 (captured motions whose rest pose lies flat play upright; pieces can be renamed from the inspector; a moving avatar keeps its hands clear of its
hips, so trousers no longer slide with each step; a garment worn over
trousers starts outside them; captured motions: Motion › Import a captured
motion… plays a BVH motion capture on the avatar, Motion › Motion
library… keeps them for every project; M15, game-engine export: Export › Garment for a game
engine (glTF)…, levels of detail, normal maps, skinning to the body's
skeleton with its motions; M31, imported bodies: Body › Import body…,
the check, the arrangement points; M20, buttons and zippers: the Button and
Zipper tools, buttoned or open while the garment drapes, the shirt
buttoned and the trousers' fly zipper; M21, prints: stripes, gingham, dots, twill and
your own images on the fabric; M24, fit maps and the tape measure; M30,
the tech pack, the projector, a
garment's notes; fabric as bought, area and mass of the
pieces, the marker with its length and cost; M23, the avatar walks, turns and raises its arms;
handles dragged with Select; Dimension tool, formulas typed in a value;
Sew: part of an edge, several edges, internal lines; ⌘-click to pick a second corner or edge,
rail tips beside the rail; M14; long sleeves, mirrored seams, agent status;
rename, delete, export and import projects; smooth views on Windows;
cloth comes to rest; pieces arranged side by side by width; M17,
draping on the graphics card; M18, shirt and trousers templates, pieces
placed on a leg; M19, several garments in layers; M22, sizes and
grading; M25, maximized views; M26, history, copy and paste, progress of long steps;
GPL licence, donations; M27, constraints, dimensions, half pieces,
construction lines, snapping, ellipses, circle skirts and arcs; M28,
formulas and variables; M29, darts, pleats, holes, joining and cutting
pieces, seam ease, partial seams and patch pockets).
-->

Rustle3D is a free, open-source desktop app for making clothes: draw the
sewing pattern in 2D, sew the pieces together and watch the garment drape
on a body in 3D, with real-time cloth physics. It runs on macOS, Windows
and Linux, and an AI agent can drive it too.

## Getting started

### The start screen
- **What will you make?** Pick a template: T-shirt, shirt (buttoned down
  the front, long sleeves with buttoned cuffs and a stand collar),
  trousers (with a fly zipper), dress, straight skirt, A-line
  skirt or tote bag (the easiest start), or a blank project. Templates
  are drafted to the body's measurements. For a shirt, the sleeve length
  includes the cuff; trousers are measured from the waist to the hem.
- **Recent projects** show a thumbnail; **All projects** lists the rest.
  You can also open a project from anywhere on disk, or open the projects
  folder.
- The **⋯** button on a project's card renames it, exports it as a .zip
  file (to send it to someone or keep a copy), or deletes it. Deleting
  asks first: tick the box saying you understand, then **Delete**. The
  project folder is deleted for good, not moved to the trash.
- **Import .zip…** adds a project someone sent you as a .zip file and
  opens it.
- Links to rustle3d.com for news and updates, and **Support Rustle3D**
  to make a donation.

### Free software, and supporting it
Rustle3D is free and open source, under the GNU GPL (version 3 or later).
You can use it for anything, including paid work, and what you make with
it (patterns, garments, pictures) is yours. Donations help pay for its
costs (hosting, test hardware, development time); nothing is locked
behind them.

After you have used the app for a while, the start screen sometimes shows
a small card asking for support: at most once every three months, never
while you work on a project. **Not now** hides it until then; **Don't
show again** hides it for good. The **Support Rustle3D** link at the
bottom of the start screen stays either way.

### The guided flow
The bar at the top walks through five steps: **Measurements → Pattern →
Sew → Fabric → Drape**. Each step says what to do and points at the tool.
Steps can be taken in any order; hide the guide with ✕ and click a step
to bring it back.

### Learn
**Learn** (top right) opens short how-tos. **Show me** points at the part
of the screen where it happens; it never changes your project. A first
tour is shown on the first launch.

### Language
English or French, from the language button in the top bar.

## The body
**Body › Measurements…** opens the measurements: choose a size (women's
or men's presets) or enter height, bust, waist and hips. Save your own
presets. The garment drapes on the new body.

### Your own body: a scan or a character
**Body › Import body…** drapes on a body of your own: a 3D scan of a
client, a character made in another program. Pick an OBJ, glTF or GLB
file (a glTF must keep its data in one file: save it as GLB if it
comes with a .bin). You must have the right to use it; it is copied
into the project's `avatars` folder and never leaves it.

Rustle3D works out the units (metres, centimetres, millimetres or
inches, from a plausible height), which way is up and which way the body
faces (its toes point forward), stands it on the floor and shows the
**Check body** window. It lists what may go wrong when draping:
- holes in the surface (a scan open under the feet or at the neck),
  edges shared by more than two triangles, loose parts: the body still
  drapes, holes are closed for the cloth as well as they can be;
- a height that does not look human, a body lying down or facing
  backwards: correct the **Units**, the **Up** axis or the **Turn**
  there;
- triangles facing inward, which are turned for you.
The Body button shows a dot while something is worth a look.

**Arrangement points** tell the drape where the chest, waist, hips and
crotch are, and the shoulders, elbows, wrists, thighs and shins: sleeves
wrap around the arms through them, trouser legs around the legs.
They are placed for you on bodies standing with their arms down or in
an A pose. **Body › Show arrangement points** draws them in the 3D view;
drag one to move it (in the plane facing you), or type its position in
the Check body window. A point outside the body shows red. Moving the
chest, waist or hips point measures the body again there: these
measurements size the templates. **Guess again** puts every point back
where Rustle3D would place it.

**Body › Use the built-in body** goes back to the built-in body, with
the measurements taken on yours. An imported body stands still: the
Motion button is off (see Known limits).

## The pattern (2D)

The left view is the pattern board: drag to pan, scroll or pinch to zoom.
The tool rail sits on its left; a button with a small corner opens a menu
of related tools (the last one used stays on the button).

| Tool | Key | What it does |
|---|---|---|
| Select | V | Click a piece, a corner or an edge (⌘-click or ⇧-click a second corner or edge); drag a corner or an edge to reshape the piece; a picked curved edge, or a corner between curves, shows its handles: drag them to bend the curve (a smooth point keeps its two handles lined up) |
| Shapes → Rectangle | R | Adds a rectangular piece |
| Shapes → Ellipse | E | Adds a circle or an ellipse of the width and height you type |
| Shapes → Circle skirt | | Adds the pieces of a full, half or quarter circle skirt from the waist and the skirt's length |
| Polygon | P | Click the corners of a new piece; click the first corner or press Enter to close; Esc cancels |
| Curve | C | Draws a piece with curves: click for a corner, drag for a smooth point; click the first point or press Enter to close |
| Edit curves → Bend | B | Click an edge, then drag its two handles to bend it |
| Edit curves → Straighten | L | Click a curved edge to make it straight |
| Edit curves → Add point | A | Click an edge to add a point there (the curve keeps its shape) |
| Edit curves → Arc | | Click an edge, type how far it bulges: it becomes a circular arc |
| Construction line | G | Click two points: a line across the board to draw against (never cut or sewn); click it and press Delete to remove it |
| Dimension | T | Click an edge, two corners or two edges, then where the dimension goes; type its value (or a formula) right there |
| Details → Dart | | Click an edge where the dart goes, type its width and length: it is drawn, folded and sewn |
| Details → Pleat | | Click an edge where the pleat starts, type its depth (and how far it is stitched down), knife or box: it is folded and sewn |
| Details → Hole | | Select a piece, click the corners of a hole inside it, Enter: the hole is cut out of the cloth |
| Cut | K | Click (or drag, for curves) a line across a piece, from outside to outside, then Enter: the piece becomes two pieces sewn together along the line |
| Draw on garment | D | Drag across the 3D garment: the line lands on the pattern |
| Notch | N | Click an edge where the notch goes |
| Sew | S | Click an edge or a line, then the one to sew it to; drag along one to sew part of it; ⌘-click several edges in a row to sew them as one side |

**Make a view bigger**: the button at the top right of the board or of
the 3D view (or **M** with the pointer over it) lets that view take the
space of both; the same button, or M again, brings both views back. While
the 3D view is hidden, a running drape pauses and goes on when the view
comes back (a drape you paused yourself stays paused); starting the drape
(Space) brings the 3D view back. The board keeps its zoom.

Other keys: **⌘Z** undo, **⇧⌘Z** redo (also Ctrl+Y on Windows and Linux),
**Backspace/Delete** deletes the selection (a single corner picked: that
point; a construction line picked: that line), **Esc** cancels a drawing.

**Snapping**: while you draw (Polygon, Curve, Cut, Construction line) or
drag a corner, the point lands on what is near the pointer: a corner,
the middle of an edge, an edge, a construction line, a symmetry axis, or
level with the previous point (or the first one, or a dragged corner's
neighbours). A small label says what it snapped to.

**Naming a piece**: select it; in the inspector, click the pencil next
to its name (or double-click the name), type the new one and press
Enter. Esc keeps the old name. Renaming is one step you can undo.

**Half pieces**: with Polygon or Curve, turn on **Half piece** (top left
of the board) and draw half of a symmetric piece, ending where you
started on the centre line: the edge that closes it is the axis, and the
other half appears, mirrored. Edit the half you drew; the other one
follows (points, curves, notches) and the axis is drawn dash-dotted.
Seams and notches can go on either half. An existing piece can be
unfolded the same way on one of its straight edges (**Unfold here** in
the edge's card), and **Separate halves** in the inspector makes both
halves ordinary again.

**Long steps**: opening a project, adding a template, choosing sizes,
changing the body's measurements, importing, exporting and starting the
drape can take a few seconds (fitting the body to new measurements
alone takes about two). The app stays responsive: a card says what it is
doing, with a bar showing the sizes being drafted when it knows how
long it will be. **Stop** on the card cancels a template or a size
range; nothing is changed. When the 3D view needs a body with new
measurements, it keeps showing the old one, with "Fitting the body…"
at the bottom, until the new one is ready.

**Copy and paste**: **⌘C** copies the selected pieces, **⌘V** pastes
them, in the same project or in another one (open it from the start
screen and paste). Pieces come with what belongs to them: the seams
between them, their notches, internal lines, pins, size grading (when
the project has the same sizes) and mirror links among them. Pasted
pieces land to the right of the others and are selected. Pasted into
another project they keep their place around the body, ready to drape;
pasted beside their originals, they are left off the body until you
place them in the inspector, so they do not start inside them.

**Undo and the history**: hover the undo or redo arrow to see what it
will undo or redo ("Undo Move point"). A whole gesture is one step: a
drag of a corner, an edge or a curve handle, a piece moved around the
body, a template added. Undoing selects the pieces that changed, so you
see what came back. The clock button next to the arrows lists every step
since the project was opened, the latest first: click one to go back to
just after it (the steps after it stay listed, muted, until you change
something else); click **Opened** to go back to the start.

### Measuring and setting sizes
- Click an edge: the inspector shows its **length**. Type a new one (or
  drag the slider); a sewn edge's partner follows so the seam stays
  matched. **Match length** makes both sides of a seam equal.
- Click a corner, then **⌘-click** another: the inspector shows the
  distance between them. Type a new distance: the second corner moves
  along the line.
- A curved edge can be made straight again with **Straighten** in its
  card, or made a circular arc with **Arc**.

### Constraints and dimensions
Constraints keep a piece the way you want it while you change the rest.
Pick what to constrain on the board: a corner or an edge, and ⌘-click (or ⇧-click) a
second one. The inspector offers what fits:
- one edge: **Horizontal**, **Vertical**, **Length** (keeps the length it
  has now);
- two edges: **Parallel**, **Square**, **Same length**, **Angle**;
- one corner: **Fixed** (it stays put while the rest adjusts),
  **Smooth** (no corner there: the curves pass through it smoothly) or
  **Corner**, **Remove point**;
- two corners: **Distance**, **Width** (across), **Height** (up and
  down), **Joined** (at the same place, for corners of two pieces);
- a corner and an edge: **On the edge**.

With **Dimension** (T) you put a dimension straight on the pattern:
click an edge (its length), a corner then another corner (the distance
between them) or two edges (the angle between them), then click where
the dimension goes. Between two corners, pulling it above or below them
gives a **width**, beside them a **height**, in between the straight
distance; the value follows the pointer. A field opens where you
clicked: type the value (or a formula) and press Enter, or Esc to keep
the value it has.

The piece's constraints are listed under **Constraints** in the
inspector and drawn on the board when it is selected. Type a new value
for a distance, a length or an angle: the piece is redrawn so that this
value and every other constraint hold, moving as little as possible;
the other half of a half piece follows. Drag a corner: it stays where
you drop it and what is constrained to it follows. If constraints
cannot all hold, the change is refused and nothing moves; a constraint
that does not hold shows in red. The inspector also says how many ways
the piece can still move (moving or turning it as a whole counts,
unless a corner is fixed), and warns when some constraints repeat
others.

### Darts, pleats, holes, joining pieces
- **Dart** (Details group): click the edge where the dart goes and type
  its width at the edge and its length. It is drawn, folded and sewn in
  one step: when the garment drapes, the dart closes.
- **Pleat**: click the edge where it starts, type its depth and choose
  knife (one fold) or box (two folds facing away). A knife pleat takes up
  twice its depth along the edge. With "Stitched down", the folds are also
  sewn that far into the piece. Notches mark the fold lines.
- **Hole**: select a piece, then click the corners of the hole inside it
  and press Enter. There is no cloth in a hole; its edges can be sewn
  (a facing) like any other.
- **Join or cut**: with a piece selected, the inspector's **Join or cut**
  takes another piece: **Join** makes one piece of the two (they must
  overlap), **Cut out** cuts the other piece out of this one (a bite out
  of its edge, or a hole when it lies inside). The other piece goes. A
  piece that is sewn, mirrored or symmetric must be unpicked or unlinked
  first.
- **Ease**: a seam's card (click a sewn edge) has an **Ease** setting:
  how much longer one side is meant to be, eased in (a sleeve cap). Only
  a difference beyond the ease is flagged.
- Seams on part of an edge, on several edges in a row (a pocket's sides
  and bottom) or on an internal line (a patch pocket on its placement
  line, a tuck) are made with **Sew**: see Sewing below.

### Formulas: patterns made to measure
A distance, a length or an angle can follow the body: type a formula
in place of its value (or click **ƒ** next to it), like `waist / 4 + 10 mm` or
`hips / 4 + ease`. Formulas use the body's measurements (`height`,
`bust`, `waist`, `hips`), numbers with their unit (`mm`, `cm`, `in`),
`+ − * /`, parentheses, `min` and `max`, and your own **variables**
(nothing selected: the **Variables** section of the inspector; **+**
adds one, like `ease = 2 cm`, click one to change it). A length needs a
unit: `waist / 4 + 10` is refused, `waist / 4 + 10 mm` is fine; the
message says what is wrong.

When you change the body's measurements or a variable, every piece with
formulas is drawn again for it. With **sizes**, each size is drafted
with its own measurements: the corners the formulas move are graded by
them (and cannot be graded by hand); the rest keeps its grade rules.
Empty the formula to keep the value as it is.

### Cutting a piece
With **Cut** (K), draw a line across one piece, starting and ending
outside it, and press Enter. You get two pieces, sewn together along the
cut, the new one moved a little aside. Notches, lines and pins go with the
piece they are on; a seam that the cut crosses is split in two. Both
pieces start the drape where the whole piece was. A piece that has a
linked mirror copy must be unlinked first.

## Sewing
- With **Sew** (S), pick one side, then the side to sew it to. The edge
  or line under the pointer lights up. A side is:
  - an edge or an internal line: click it;
  - part of one: drag along it from where the seam starts to where it
    ends (the ends catch on corners, notches and the ends of other
    seams);
  - several edges in a row (a pocket's sides and bottom): ⌘-click each
    of them, in any order, then click the last one or press Enter.

  Esc starts again. Matching arrows on both sides show how they join; a
  mismatch in length is flagged in red. If the seam looks twisted, flip
  it in the edge's card. An edge sewn to the same edge on its mirror copy
  (a centre front or back) joins it end to matching end.
- **Gathered**: in an edge's card, a seam can be gathered: the longer side
  is gathered evenly onto the shorter one (a skirt into a waistband, a
  sleeve cap). The ratio is shown (for example 2 : 1).

## Buttons and zippers
- **Button** (in the rail's **Trims** menu): click where the button goes,
  then where it buttons, on the piece that laps over it: that point is
  the end of the buttonhole where the button sits. The board draws the
  button at its size (11 mm unless you change it) and the buttonhole as
  a slit running into the piece. A cuff buttons on itself: click both
  points on the cuff.
- **Zipper** (Trims menu): click one edge near the zipper's bottom, then
  the edge it closes onto. The zipper runs along both edges; an edge
  that is sewn cannot take a zipper (and a zipped edge cannot be sewn).
  On the board a zipper is a brass band, solid where it is closed,
  dashed where it is open.
- Select a piece: its card lists its buttons, buttonholes and zippers.
  Switch a button off to unbutton it (or **Unbutton all**), drag a
  zipper's **Open** slider or type how many centimetres are open from
  the top, and remove a trim with its ×. Click the garment in the 3D
  view to pick the piece there.
- In 3D, a buttoned button holds the two pieces together and a zipper
  closes like a seam up to its slider. Button, unbutton or open a zipper
  while the garment drapes: it opens or closes in a fraction of a
  second, without starting over. Buttons show as small dark discs on the
  cloth, a zipper as a brass band.
- The shirt template comes buttoned: buttons down one front, buttonholes
  on the other (left over right on a men's body, right over left
  otherwise), one on each cuff. The trousers have an 18 cm fly zipper,
  closed.

## Fabric and garments
With nothing selected, the panel on the right lists the project's
garments: pick each one's fabric (cotton, denim, jersey, silk, wool).
Fabric changes how a garment hangs and looks; the drape follows. (A
custom colour can be set by an agent or the command line,
`garment.set_color`; not yet in the app.)

Under the fabric, **As bought** describes the fabric you buy: its
**Weight** (grams per square metre; left empty, the usual weight of that
fabric, shown in grey), its **Width** (cm, from one selvedge to the
other) and its **Price / m** (a metre of length, in your currency).
The weight is also what the cloth weighs in the drape. Below, the
garment's totals: how many pieces, the fabric they take once cut (with
their seam allowances) and what they weigh. With a piece selected, its
area, perimeter, cut area and mass are in the inspector.

Under the fabric, **Print** puts a print on the garment: none, one of
four built-in prints drawn in the garment's colour (**Stripes**,
**Gingham**, **Dots**, **Twill**), or **your own image** (a PNG or JPEG
photo or scan of a fabric; it is copied into the project's `textures`
folder). The print lies along each piece's grainline at its real size,
like a fabric laid out for cutting: stripes run down the fronts, the
back and along the sleeves, wherever each piece's grainline points. Set
the **Repeat width** (cm, how wide one tile of the print is on the
fabric; the height follows the image) and its **Angle to the grain**
(turn it 45° for a bias effect). The print shows in the 3D view, on the
pattern pieces, in photos and in the 3D exports. With a piece selected,
**Print on this piece** shifts it across or up and turns it on that
piece only: to centre a motif on a front, or to match stripes across a
seam.

Under them, **Notes** takes free text for whoever sews or produces the
garment (interfacing, pressing, topstitching, labels). It is saved when
you leave the field and printed in the tech pack.

A project can hold several garments, for example trousers and a shirt.
Start a second template from the project (or **Add a garment**): it
comes as a garment of its own, worn over the others. The list goes from
the body out; the arrows wear a garment under or over its neighbour. To
move a piece to another garment, pick it in the piece's **Garment**
menu. Deleting a garment deletes its pieces too (undo brings them back).

## Sizes
A pattern can come in a range of sizes, for example 36 to 52. It is
drawn in one size, the **base size**; each point says how it moves from
size to size (its grade), and the other sizes follow.
- With nothing selected, **Sizes** › **Add a size range** in the panel on
  the right: pick a chart (women's 32 to 56 or men's 44 to 60, European
  sizes), the smallest and largest size, and the base size. Each size has
  a body with its measurements, built from your avatar.
- A garment started from a template, before or after setting the sizes,
  is drafted again for every size's body, so every size fits; the
  changes you made to its pieces are kept. (Not when you have added or
  deleted a piece, or points on a piece's outline: grade those points
  yourself.) Garments you drew yourself stay in one size until you grade
  their points.
- On the board, each piece shows the other sizes' outlines around it;
  **Show the sizes on the board** hides or shows them.
- **Shown in 3D** picks the size the garment is draped in, on a body
  with that size's measurements; the drape starts again. The pattern
  you edit stays in the base size.
- To grade a point yourself, click it on the board: **Grade per size**
  sets how far it moves, in mm, from each size to the next (x to the
  right, y up). Curves, notches and lines inside the piece follow the
  points; a mirror copy follows its source.
- **Change** picks another range or base size (the grades are kept for
  the sizes that stay); the delete button removes the sizes and every
  grade.

## Draping (3D)
- **Simulate** (Space) drapes the garment: the seams close and the cloth
  falls onto the body. Press again to pause; the arrow next to it starts
  again from the pattern.
- Look around: drag to turn, scroll or pinch to zoom; keys **1–4** for
  front, side, back and three-quarter views.
- **Drag** the cloth to pull it; **⌥-click** pins it where it is (the
  pins are listed in the inspector).
- **⇧-drag** a hem on the garment to lengthen or shorten it; **⇧-drag** a
  seam to take it in or let it out. The pattern follows.
- **⌘-drag** a piece to move it around the body, or set its place in the
  inspector (around the body, an arm, a leg, or flat; angle and height).
  A sleeve placed on an arm starts down the upper arm and, past the
  elbow, down the forearm, so long sleeves drape too. A trouser piece
  placed on a leg starts around that leg below the crotch and around
  that side of the hips above it; place its top at the waist.
- **Arrange around the body** places every piece side by side in seam
  order at the waist, each taking room in proportion to its width.
- **Motion** (the walking figure next to Simulate) makes the avatar move
  while the garment drapes: **Walk** on the spot, **Turn** half round and
  back, **Raise arms** above the shoulders and down; **Stand still**
  stops. The motion starts once the garment has closed on the body
  (after about 1.5 s of draping) and repeats; choosing another one first
  brings the body back to rest. The cloth follows the body: watch a skirt
  swing, or a shirt ride up when the arms go up. You can keep editing the
  pattern meanwhile, and a photo or a 3D export shows the body where it
  is.
- **Motion › Import a captured motion…** plays a motion recorded on a
  real person, from a BVH file (the usual motion capture format). The
  file is copied into the project's `motions` folder and listed in the
  menu under its file name. The motion is fitted to the avatar's
  skeleton and size, played on the spot (the avatar does not travel)
  and repeated: Rustle3D finds the longest stretch that ends as it
  starts, such as one or two steps of a walk, and blends the end into
  the start. Most capture skeletons work (names such as Hips, LeftUpLeg,
  LeftArm, LeftForeArm), whichever way up the file was written; the file
  needs at least about 1.5 s of motion.
  Where the person's arms hung closer to the body than the avatar's hips
  allow, the avatar turns its arms out just enough to keep its hands and
  forearms clear of the body (and of the cloth on it).
  Use files whose licence lets you use them.
- After an import, **Add to library** (in the message at the bottom)
  keeps the motion in your **motion library**, shared by all your
  projects. Its motions are listed in the Motion menu too; choosing one
  copies it into the project and plays it. **Motion › Motion library…**
  manages it: **Add files…** (several BVH files at once), play a motion
  on the avatar, rename or delete it. Deleting a motion from the
  library leaves the projects that use it unchanged: they keep their
  copy.
- **Fit** (the gauge next to Motion) colours the garment by how it
  fits, from dark violet (loose) to yellow (tight); a legend at the
  bottom right gives the scale and the tightest spot:
  - **Strain**: how much the cloth is stretched beyond its pattern, 0 %
    to 20 % and more. A knit T-shirt reads a few percent and more under
    the arms; a woven fabric barely stretches, so it stays dark except
    where it pulls.
  - **Gap to the body**: how far the cloth stands off the body, from
    5 cm and more to touching (yellow).
  - **Fabric colours** shows the garment as it is again. The colours
    are chosen to read for colour-blind eyes and in grey.
- **Tape measure** (the tool under Draw on garment): click on the draped
  garment. The tape goes around the garment at that height and is drawn
  on it; a card gives the height, the **garment's** circumference (along
  the cloth, folds included), the **body's** inside it (as a tape around
  the body reads it), the **ease** between them and the **gap** between
  the cloth and the body where you clicked. Click on a sleeve to measure
  around the arm. Where the garment does not go all round (an open
  front, a hem), there is no ease. The tape is taken when you click:
  click again after the drape has moved. Drag to turn the view; the ×
  or another tool puts the tape away.
- The drape runs on the graphics card, about five times faster than on
  the processor; the status bar says **GPU** and the time of a step.
  Without a usable graphics card, or if it fails, the drape carries on
  from where it was on the processor (**CPU**). To use the processor
  anyway, start the app with the environment variable `R3D_SOLVER=cpu`.

## Photos
**Photo** (the camera button at the top right of the 3D view) makes a
photorealistic image of the garment on the body, as the 3D view shows it
now: the same angle and zoom, the cloth where it is. Choose:
- **Quality**: **Quick** (a first look in seconds), **Good** (clean
  enough to show, the usual choice) or **Best** (for print and close-ups;
  it takes several times longer).
- **Lighting**: **Studio** (a main light, a softer one on the other side
  and a light from behind, on a light grey backdrop), **Soft** (large
  soft lights on a white backdrop, few shadows, like a shop's product
  photos) or **Dramatic** (one hard light on a dark backdrop).
- **Size**: as the 3D view, 1920 × 1080, 1080 × 1350 (portrait, for
  social media), 2048 × 2048 or 3840 × 2160 (4K).
- **Advanced** (folded): samples per pixel (more is cleaner and slower),
  light bounces, and whether to remove the remaining noise.

The image appears in the 3D view and gets cleaner as it renders, with
its progress at the bottom. **Stop** keeps the image as it is; **Save
image…** saves it as a PNG file; **×** goes back to the live 3D view
(turning, simulating, editing). The photo takes a snapshot: the drape
can keep running underneath. The garment's colour in the photo is its
true colour under neutral light.

## Export
**Export** in the top bar, then choose where to save:
- **Pattern, SVG at 1:1**: one drawing at real size, for vector editors
  and cutting machines.
- **Pattern, PDF on A4 pages**: to print at home. Each page is labelled
  (A1, A2… row by row) and overlaps its neighbours by 1 cm, with dashed
  guides to line the pages up before taping them together.
- **Pattern, PDF on one page**: one large page, for a plotter or a print
  shop.
- **Pattern, DXF-AAMA (pattern software)**: the industry exchange format
  read by pattern-making, grading and cutting programs, in millimetres:
  each piece with its cutting and sewing lines, notches, grainline,
  internal lines and name.
- **Garment, GLB** (glTF) or **OBJ**, with or without the body, as the 3D
  view shows it, for Blender, game engines and other 3D tools. A print
  goes with it: inside the GLB file, or for OBJ as a material file
  (`.mtl`) and the print's image saved next to the OBJ.
- **Garment for a game engine (glTF)…**: the garment ready for Godot, Unity, Unreal or
  Blender, in one GLB file (see below).

### Game engines
**Export › Garment for a game engine (glTF)…** opens a window:
- **Engine**: Generic, Godot, Unity or Unreal. It fills the other
  settings with what suits that engine, and names things its way (for
  Unreal `SK_`, `M_` and `T_` names).
- **Levels of detail**: lighter copies of the garment for when it is far
  away, as percentages of its triangles, for example `100, 50, 20`. The
  first is always 100. The edges of the pieces never move, so seams stay
  closed and hems keep their shape; a level can keep a few more
  triangles than asked.
- **Normal map**: the size of the image (512 to 4096 pixels) into which
  the full garment's wrinkles are baked for each lighter level, so they
  still look wrinkled. **None** leaves them out.
- **Back faces**: **Double-sided** (the cloth drawn from both sides) or
  **Thickness** (a real inside surface a little inside the outside one,
  joined along the edges; give the thickness in mm), for engines or
  shaders that hide back faces.
- **Skin to the body's skeleton**: the garment follows the body's 24
  bones, with the walk, turn and raise-arms motions as animations, so it
  moves with a character. It is off for an imported body (it has no
  skeleton). **Include the body** adds the body too.

The garment is exported as the 3D view shows it, put back in the body's
standing pose when skinned. Each garment is one mesh per level of
detail, named `Shirt_LOD0`, `Shirt_LOD1`…, and its pieces are laid out
in one image (with the print baked into it). The file is in metres with
Y up, as glTF wants: the engines convert it themselves.
- **Godot 4**: drop the `.glb` into the project and open it: the
  skeleton, one mesh per level (show one, or set visibility ranges) and
  an AnimationPlayer with `walk`, `turn` and `raise_arms`.
- **Unity**: install the glTFast package, drop the `.glb` into Assets,
  and put the `_LOD` meshes in a LOD Group.
- **Unreal Engine 5**: Import, choose the `.glb`: it comes in as a
  skeletal mesh with its animations (in centimetres, Z up).
- **Blender**: File › Import › glTF 2.0.

From the command line: `r3d export My.r3d --format game --preset godot`
(also `--lods 1,0.5,0.2`, `--normal-map 2048`, `--shell`,
`--thickness-mm 1`, `--no-skin`, `--avatar`).

Every pattern export shows the cutting line, the sewing line (dashed,
when the piece has a seam allowance), notches, internal lines, the
grainline and the piece name, plus a **100 mm test square**. Print at
100 % ("actual size", never "fit to page") and measure the square before
cutting.

A pattern with sizes exports every size nested: the base size in full,
the other sizes' cutting lines in colour, named in a legend next to the
test square; in DXF-AAMA, every piece once per size, with its size. To
export one size or a few, use the command line
(`r3d export My.r3d --format pdf --sizes 44`) or an agent.

## Marker: fabric needed and cost
**Export** → **Marker: the pieces laid out on the fabric…** lays the
pieces of a garment out on its fabric the way a cutting room does, to
know how much fabric to buy:
- Pick the **sizes to cut** (with a size range; the size shown in 3D is
  picked first) and the **fabric width** (the garment's, else 150 cm).
  Each piece is laid once per size, with its seam allowance, its
  grainline along the selvedge (it may be turned end for end). A left
  and a right piece (a mirror copy) are two pieces.
- The layout is drawn to scale, each size in its colour, with the
  **length** of fabric it takes, the share of the fabric **used** (the
  rest is waste), the length **to buy** (rounded up to the centimetre)
  and its **cost**, when the garment's fabric has a price per metre.
- **Save SVG** or **Save PDF** writes the marker at 1:1 (one long page,
  for a plotter): the fabric's edges, each piece's lines, grainline,
  name and size, the figures and the 100 mm test square.

The layout is automatic and the same every time. Pieces are placed on
a 5 mm grid, so they may sit a few millimetres apart; a careful layout
by hand may still save a little. Fabric with a direction
(a nap, a one-way print), folded fabric and several layers are not
handled yet. Agents and scripts can cut a piece more times, in pairs or
with a free grain (`marker.compute`, `r3d marker My.r3d --sizes 38,40,42
--width-cm 150 --format pdf`).

## Tech pack
**Export** → **Tech pack (PDF)…** writes the garment's production file,
in landscape A4 pages, in the app's language:
- **Views**: the garment from the front and the back, as the 3D view
  shows it now (drape first for a garment that hangs right).
- **Colourways**: each garment's colour, fabric and colour code.
- **Bill of materials**: each garment's fabric with its weight, width
  and price per metre, the fabric one garment takes (from its marker in
  the size the pattern is drawn in) and its cost; with sizes, the fabric
  per garment in every size; the trims: buttons by size and zippers by
  length, how many, and the pieces they go on.
- **Measurements**: with sizes, the size chart (height, bust, waist and
  hips of each size's body); every piece's length along its grainline,
  width, sewing line, areas, seam allowance and mass; with sizes, every
  piece's length and width in each size.
- **Pattern**: every piece (every size nested, with their colours)
  scaled to fit the page; the scale is written under it (1:15). Print
  the pattern itself from **Export** at 1:1.
- **Sewing notes**: every seam (its two pieces and their lengths, ease,
  gathering) and the garments' notes.

Agents and scripts write it too (`r3d techpack My.r3d`; with
`--views photo`, quick photos instead of the 3D view).

## Projector
**Export** → **Projector: the pattern at 1:1…** shows the pattern at
real size for a projector hung over the cutting table, to trace or cut
on the fabric without printing. Select the pieces to project first (on
the board); with nothing selected, every piece comes. They come in the
size shown in 3D, as white lines on black.
- Move the window to the projector's screen and maximise it.
- **Calibrate** (C) once for the room: a square that should be 20 cm a
  side is projected; measure its width and height on the table with a
  ruler and type them in. The pattern is then at 1:1. A 10 cm scale
  bar in the bottom right corner lets you check it at any time.
- Drag, or use the arrow keys (1 cm; with Shift, 10 cm), to bring the
  part of the pattern you are cutting under the lamp; Home brings the
  pieces back to the middle.
- **Mirror** (M) turns the pattern over, for a projector shining
  through a mirror or a piece cut from the wrong side; **Turn** (R)
  turns it by a quarter; **Contrast** (I) swaps to black lines on
  white.
- Escape closes it. The calibration, mirror, turn and contrast are
  remembered by the app (they belong to the room, not to the project).

## Import
**Import** in the top bar brings in a pattern drawn in another program:
a **DXF-AAMA** or plain **DXF** file, an **SVG**, or a **PDF** (a pattern
printed on several pages is put back together, as long as each page shows
part of the same drawing).
- Each closed outline becomes a piece. A second outline just inside it,
  at the same distance all around (usually dashed), is read as the
  sewing line, and that distance as the seam allowance. Short marks across the edge become notches, a line with
  arrowheads the grainline, other lines inside the piece internal lines,
  and the largest text inside the piece its name. A test square is left
  out.
- Curves drawn as many small straight segments are turned back into
  smooth curves you can edit with the usual tools.
- The new pieces appear to the right of the ones already on the board.
  They are not sewn yet: sew them with **Sew** (S). One **⌘Z** removes the
  whole import.
- A message says how many pieces came in, and how many shapes were left
  out (outside every piece).
- If the file does not give its scale (some DXF and SVG files), Rustle3D
  asks which unit it was drawn in: millimetres, centimetres, inches,
  points or pixels. **Keep as is** keeps the first reading.
- A copy of the file is kept in the project, in its `imports` folder.

## AI agents
Rustle3D can be driven by an AI agent through MCP (Model Context
Protocol):
- **In the app**: the **Agents** button lets one agent edit the open
  project; it reads "Agent connected" while one is attached, even during
  a long action such as a drape. You see every change live and can undo
  it, or disconnect the agent. Configure the agent to run
  `r3d mcp --attach`.
- **Without the app**: `r3d mcp --project My.r3d` serves a project
  headless. Drapes made there (and with `r3d simulate`) run on the
  processor, so they give the same result every time. A drape can make
  the avatar move: `r3d simulate My.r3d --frames 300 --motion walk`
  (`walk`, `turn`, `raise_arms`, or a `.bvh` file, copied into the
  project's `motions` folder); pictures and exports then show the
  body and the garment at the last frame.
- An agent can look at its work: a picture of the pattern, of the 3D
  view, or a photo like the app's (`render_photo`). From the command
  line: `r3d render My.r3d --view photo --out photo.png`.
- An agent can check the fit: `r3d render My.r3d --view 3d --fit-map
  strain --out fit.png` colours the drape (or `gap`), and `r3d fit
  My.r3d` lists each piece's strain, the tight spots and the ease at
  the chest, waist and hips (`fit_report`, `fit_tape` for an agent).
Everything the app does is a command that an agent can run too.

## Files
Projects are saved as you work, in Documents › Rustle3D. They are
folders with plain JSON files, so they work with version control.
Renaming a project also renames its folder, when no other project has
that name. A project exported as .zip holds its folder (pattern, body,
exports), without the cache the app rebuilds by itself.
Print images you add are kept in the project's `textures` folder;
replacing a file there (same name) changes the print. Bodies you import
are kept in its `avatars` folder; if that file changes, import it again
(the check says so).
Patterns come in from DXF, SVG and PDF and go out as SVG, PDF and
DXF-AAMA; garments go out as glTF and OBJ; the marker as SVG or PDF and
the tech pack as PDF: open formats only.

## Known limits
- The projector view fills the app's window, not the whole screen:
  maximise the window on the projector. It cannot correct a projector
  that is not square to the table (keystone): the scale bar then reads
  differently at the far end. The tech pack's PDF font writes Western
  European letters only; others show as "?".
- On Linux, the pattern and the 3D views are less smooth than on macOS
  and Windows (their images are copied through the processor each
  frame). The same happens on a computer with several graphics cards
  when the app does not run on the one that shows the window.
- On the men's body presets, a heavy skirt (denim) can slide off the hips
  during the drape.
- Garments worn close together can show through each other where they
  touch (a T-shirt about as wide as the trousers under it, at the hips),
  and a shirt cannot be tucked into fitted trousers yet: layers of cloth
  keep about 2 cm apart in the simulation.
- Game export: the levels of detail are separate meshes (no automatic
  switching inside the file); the inside of a garment exported with
  thickness shows the wrinkles of the outside reversed.
- The avatar walks on the spot (not along a path), raises its arms to
  above the shoulders but not straight up, and only the built-in body
  moves (not an imported one, even if its file has a skeleton). A
  captured motion that travels or turns is played on the spot, facing
  forward; its fingers are not used; the game export includes only the
  built-in motions.
- An imported body is the same in every size of a size range. Its
  arrangement points are guessed for arms down or in an A pose: on a
  T pose, move the shoulder, elbow and wrist points. Compressed glTF
  (Draco, meshopt) cannot be read; files over 2 million triangles are
  refused, and above 300 000 the views and the drape slow down.
- Not yet in the simulation: elastic and shirring, visible topstitching,
  interfacing, quilting and padding. Holes are cut on their line (no seam
  allowance inside them), and a mirrored or symmetric piece cannot have
  holes.
- Fit maps show the stretch of the cloth, not the force in it (a stiff
  woven pulls harder for the same stretch). The tape measures
  horizontally only, and the body's circumference is read inside the
  garment's loop: at a sleeve's hem cut on a slant there may be none.
- Constraints apply to corners and edges of the outline, not to internal
  lines, notches or construction lines. A dimension stays where the
  board draws it (it cannot be moved by hand). A symmetric piece's axis
  is straight.
- Sizes: notches keep their distance along the edge in every size, and
  a grade is set per point, not per edge, in the app. The shirt template
  drapes with folds at the sides in the largest sizes (from about 48).
  A template's pieces graded after they were laid out keep their place
  on the board: the largest sizes of neighbouring pieces can touch.
- Buttons and zippers: photos do not show them yet. A copied piece is
  pasted without its buttons. The two pieces a button holds may pass
  through each other near the button (they are allowed to lie on each
  other there). Agents can place a zipper on part of an edge; in the app
  the Zipper tool takes whole edges.
- Prints colour the fabric only: no relief of the weave yet, and the
  same print shows on both sides of the cloth. Transparent parts of an
  image are shown as they are, not see-through. The marker does not
  match stripes or checks between pieces yet.
- Photos: the cloth is a single thin layer (no thickness at the hem). Your own studio
  lighting (an HDRI image) cannot be loaded yet.
- Import: a PDF whose pages each hold a different part of the pattern
  (rather than the same drawing shifted) cannot be put back together;
  binary DXF files and protected PDFs cannot be read. Grading rules are
  not read: pieces come in at the size they are drawn.
