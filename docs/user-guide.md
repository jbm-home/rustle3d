# Rustle3D user guide

<!--
Source of the user documentation on rustle3d.com. Written for people who
use the app, not for developers: what it does and how to do it, in plain
words. Keep it in step with the app: every user-visible change (a tool, a
shortcut, a menu, a format, a limit) updates this file in the same
commit (CLAUDE.md §5). Shortcuts are written with the macOS keys; on
Windows and Linux ⌘ is Ctrl and ⌥ is Alt.
Last updated: 2026-11-18 (M14; long sleeves, mirrored seams, agent status;
rename, delete, export and import projects; smooth views on Windows;
cloth comes to rest; pieces arranged side by side by width; M17,
draping on the graphics card; M18, shirt and trousers templates, pieces
placed on a leg; M19, several garments in layers; M22, sizes and
grading).
-->

Rustle3D is a free, open-source desktop app for making clothes: draw the
sewing pattern in 2D, sew the pieces together and watch the garment drape
on a body in 3D, with real-time cloth physics. It runs on macOS, Windows
and Linux, and an AI agent can drive it too.

## Getting started

### The start screen
- **What will you make?** Pick a template: T-shirt, shirt (long sleeves,
  cuffs and a stand collar), trousers, dress, straight skirt, A-line
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
- Links to rustle3d.com for news and updates.

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
**Body** opens the measurements: choose a size (women's or men's presets)
or enter height, bust, waist and hips. Save your own presets. The garment
drapes on the new body.

## The pattern (2D)

The left view is the pattern board: drag to pan, scroll or pinch to zoom.
The tool rail sits on its left; a button with a small corner opens a menu
of related tools (the last one used stays on the button).

| Tool | Key | What it does |
|---|---|---|
| Select | V | Click a piece or an edge; drag a corner or an edge to reshape the piece |
| Rectangle | R | Adds a rectangular piece |
| Polygon | P | Click the corners of a new piece; click the first corner or press Enter to close; Esc cancels |
| Curve | C | Draws a piece with curves: click for a corner, drag for a smooth point; click the first point or press Enter to close |
| Edit curves → Bend | B | Click an edge, then drag its two handles to bend it |
| Edit curves → Straighten | L | Click a curved edge to make it straight |
| Edit curves → Add point | A | Click an edge to add a point there (the curve keeps its shape) |
| Cut | K | Click (or drag, for curves) a line across a piece, from outside to outside, then Enter: the piece becomes two pieces sewn together along the line |
| Draw on garment | D | Drag across the 3D garment: the line lands on the pattern |
| Notch | N | Click an edge where the notch goes |
| Sew | S | Click an edge, then the edge to sew it to |

Other keys: **⌘Z** undo, **⇧⌘Z** redo (also Ctrl+Y on Windows and Linux),
**Backspace/Delete** deletes the selection, **Esc** cancels a drawing.

### Measuring and setting sizes
- Click an edge: the inspector shows its **length**. Type a new one (or
  drag the slider); a sewn edge's partner follows so the seam stays
  matched. **Match length** makes both sides of a seam equal.
- Click a corner, then **⇧-click** another: the inspector shows the
  distance between them. Type a new distance: the second corner moves
  along the line.
- A curved edge can be made straight again with **Straighten** in its
  card.

### Cutting a piece
With **Cut** (K), draw a line across one piece, starting and ending
outside it, and press Enter. You get two pieces, sewn together along the
cut, the new one moved a little aside. Notches, lines and pins go with the
piece they are on; a seam that the cut crosses is split in two. Both
pieces start the drape where the whole piece was. A piece that has a
linked mirror copy must be unlinked first.

## Sewing
- With **Sew** (S), click two edges. Matching arrows on both edges show
  how they join; a mismatch in length is flagged in red. An edge sewn to
  the same edge on its mirror copy (a centre front or back) joins it end
  to matching end.
- **Gathered**: in an edge's card, a seam can be gathered: the longer side
  is gathered evenly onto the shorter one (a skirt into a waistband, a
  sleeve cap). The ratio is shown (for example 2 : 1).

## Fabric and garments
With nothing selected, the panel on the right lists the project's
garments: pick each one's fabric (cotton, denim, jersey, silk, wool).
Fabric changes how a garment hangs and looks; the drape follows. (A
custom colour can be set by an agent or the command line,
`garment.set_color`; not yet in the app.)

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
  view shows it, for Blender, game engines and other 3D tools.

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
  processor, so they give the same result every time.
- An agent can look at its work: a picture of the pattern, of the 3D
  view, or a photo like the app's (`render_photo`). From the command
  line: `r3d render My.r3d --view photo --out photo.png`.
Everything the app does is a command that an agent can run too.

## Files
Projects are saved as you work, in Documents › Rustle3D. They are
folders with plain JSON files, so they work with version control.
Renaming a project also renames its folder, when no other project has
that name. A project exported as .zip holds its folder (pattern, body,
exports), without the cache the app rebuilds by itself.
Patterns come in from DXF, SVG and PDF and go out as SVG, PDF and
DXF-AAMA; garments go out as glTF and OBJ: open formats only.

## Known limits
- On Linux, the pattern and the 3D views are less smooth than on macOS
  and Windows (their images are copied through the processor each
  frame). The same happens on a computer with several graphics cards
  when the app does not run on the one that shows the window.
- On the men's body presets, a heavy skirt (denim) can slide off the hips
  during the drape.
- Garments worn close together can show through each other where they
  touch, and a shirt cannot be tucked into fitted trousers yet: layers of
  cloth keep about 2 cm apart in the simulation.
- A point's handles cannot be edited after it is drawn yet; use Bend on
  the edge.
- Sizes: notches keep their distance along the edge in every size, and
  a grade is set per point, not per edge, in the app. The shirt template
  drapes with folds at the sides in the largest sizes (from about 48).
  A template's pieces graded after they were laid out keep their place
  on the board: the largest sizes of neighbouring pieces can touch.
- Photos: the fabric has no visible weave or print yet, and the cloth
  is a single thin layer (no thickness at the hem). Your own studio
  lighting (an HDRI image) cannot be loaded yet.
- Import: a PDF whose pages each hold a different part of the pattern
  (rather than the same drawing shifted) cannot be put back together;
  binary DXF files and protected PDFs cannot be read. Grading rules are
  not read: pieces come in at the size they are drawn.
