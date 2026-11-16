# Rustle3D user guide

<!--
Source of the user documentation on rustle3d.com. Written for people who
use the app, not for developers: what it does and how to do it, in plain
words. Keep it in step with the app: every user-visible change (a tool, a
shortcut, a menu, a format, a limit) updates this file in the same
commit (CLAUDE.md §5). Shortcuts are written with the macOS keys; on
Windows and Linux ⌘ is Ctrl and ⌥ is Alt.
Last updated: 2026-11-16 (M12).
-->

Rustle3D is a free, open-source desktop app for making clothes: draw the
sewing pattern in 2D, sew the pieces together and watch the garment drape
on a body in 3D, with real-time cloth physics. It runs on macOS, Windows
and Linux, and an AI agent can drive it too.

## Getting started

### The start screen
- **What will you make?** Pick a template: T-shirt, dress, straight
  skirt, A-line skirt or tote bag (the easiest start), or a blank
  project. Templates are drafted to the body's measurements.
- **Recent projects** show a thumbnail; **All projects** lists the rest.
  You can also open a project from anywhere on disk, or open the projects
  folder.
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
  how they join; a mismatch in length is flagged in red.
- **Gathered**: in an edge's card, a seam can be gathered: the longer side
  is gathered evenly onto the shorter one (a skirt into a waistband, a
  sleeve cap). The ratio is shown (for example 2 : 1).

## Fabric
With nothing selected, the panel on the right shows the garment: pick a
fabric (cotton, denim, jersey, silk, wool). Fabric changes how the
garment hangs and looks; the drape follows. (A custom colour can be set
by an agent or the command line, `garment.set_color`; not yet in the
app.)

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
  inspector (around the body, an arm, or flat; angle and height).
- **Arrange around the body** places every piece in seam order at the
  waist.

## Export
**Export** in the top bar:
- the pattern as **SVG** or **PDF** (1:1 scale, tiled on pages to print),
- the draped garment as **GLB** (glTF) or **OBJ**, with or without the
  body, for Blender, game engines and other 3D tools.

## AI agents
Rustle3D can be driven by an AI agent through MCP (Model Context
Protocol):
- **In the app**: the **Agents** button lets one agent edit the open
  project; you see every change live and can undo it, or disconnect the
  agent. Configure the agent to run `r3d mcp --attach`.
- **Without the app**: `r3d mcp --project My.r3d` serves a project
  headless.
Everything the app does is a command that an agent can run too.

## Files
Projects are saved as you work, in Documents › Rustle3D. They are folders with plain JSON files, so they work with version
control. Patterns go out as SVG and PDF, garments as glTF and OBJ: open
formats only.

## Known limits
- On the men's body presets, a heavy skirt (denim) can slide off the hips
  during the drape.
- A point's handles cannot be edited after it is drawn yet; use Bend on
  the edge.
