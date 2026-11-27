# Rustle3D user guide

<!--
Source of the user documentation on app.rustle3d.com. Written for people who
use the app, not for developers: what it does and how to do it, in plain
words. Keep it in step with the app: every user-visible change (a tool, a
shortcut, a menu, a format, a limit) updates this file in the same
commit (CLAUDE.md §5). Shortcuts are written with the macOS keys; on
Windows and Linux ⌘ is Ctrl and ⌥ is Alt.
Last updated: 2026-11-27 (a dress down to the ankles hangs in front of and behind the legs instead of catching between them; on macOS, double-clicking the title bar or going full screen no longer closes the app; on a loose garment the tape measure clicked on the chest stays level instead of slanting with the folds; the website is now app.rustle3d.com: the app's links and its update check go there; the pattern and the 3D view are no longer redrawn while nothing changes: an idle window leaves the processor and the graphics card alone; dragging on the pattern is smoother; a place in a photo is shown as a room of the size you choose (**Place size**, from 1 m), and **Default settings** puts the photo scene's settings back, instead of its lower half stretched over the floor; the photo scene: **Photo** shows the photo in the 3D view as it will be while you turn around the garment, zoom and set the light, its turn and brightness (for the studios too) and a lens from 18 to 200 mm; the scene is saved with the project; photos lit by a real place: **Place** in the Photo window, five built-in photo studios or your own HDR image (.hdr), seen behind the garment with its shadow on the floor, turned and brighter or darker as you like; `r3d diagnose`, a report of where a drape goes wrong, for agents; bodies in PLY and STL, as body scanners save them; how to bring in a body in FBX or another format through Blender; a child's arms go into its sleeves: on a baby the arm came out through the sleeve's cloth; a Library button on the start screen and in a project, to manage your own patterns; on the start screen, Blank project comes first under New; a garment's name no longer squeezes beside its buttons in the inspector; children's tops (a baby's bodysuit above all) are cut with a child's eases: closer armholes, narrower sleeves; on the start screen, choosing another body for a new garment shows a card while the body is fitted, instead of the window freezing; children: in the Body panel, Adult or Child with an age, girl or boy, and presets for babies from 3 to 24 months and girls and boys from 2 to 14 years; size ranges by age (girls, boys, babies); new templates for children, in the library under Babies and children and on the start screen: a baby bodysuit, a pyjama and a jogging suit, each set added as two garments, the top over the bottoms, and a sweatshirt; garments are drafted and placed on a child's proportions; the 3D view frames a child as it frames an adult; the pattern library: **From the library…** under the garments adds a pattern to the open project, drafted for its body, and the bookmark button on a garment's card saves it to your own patterns, to add it to other projects, renamed, exported and imported as a file; two new templates: jeans, with five pockets, a back yoke, a fly and a waistband with belt loops, and shorts; copy and paste now bring a piece's elastics, topstitching, interfacing, quilting, buttons, zippers and dimensions; an imported body moves: walk, turn, raise arms, stand at ease and captured motions play on it, with its file's own skeleton when it has one, otherwise with a skeleton placed by its arrangement points; the game export skins an imported body and includes the project's captured motions; with Thickness, the inside of an exported garment shows its folds the right way round; a garment can be taken out of the drape and put back, from its card; two new templates: a hoodie, with a hood and gathered cuffs and hem, and joggers, pulled on, with an elastic waist and ankles; the seams beside an elastic close instead of staying open; the built-in motions look natural: the walk rolls each foot from heel to toe without sliding or hopping, the hips and trunk turn against each other, the arms hang and swing loosely; Turn is made in steps; Raise arms goes above the head; the body keeps its shape at the shoulders, elbows and knees when they bend; an elastic waist now holds trousers up, in jersey too; quilted garments come to rest; a piece you draw gets a 1 cm seam allowance and joins the garment added last; Check for updates on the start screen opens a window: the daily check on or off, and Check now; portable .zip packages for macOS and Windows, to run without installing; the Agents guide saves an extension for Claude Desktop (.mcpb); several agents can be connected at once, so an assistant's chat and coding sessions both reach the project; quilted and topstitched garments draw much faster in the 3D view; zoomed out, the pattern draws stitches as a line; the Agents button opens a guide to connect an AI agent: allow it, copy the server's settings for your agent with r3d's full path, and see it connect; the shirt template has a yoke across its back, topstitched, and its collar and cuffs interfaced; quilting: select a piece and choose Quilting in the action bar, diamonds or channels over a padding; the garment puffs between the stitched lines in 3D and in photos, and the tech pack lists the padding; interfacing: select a piece and choose Interfacing in the action bar, or pick an edge for a band along it; light, medium or heavy, it makes the cloth hold its shape, and the tech pack lists its area; lines inside a piece may now cross; topstitching: pick an edge, then Topstitch in the action bar: one to three rows of stitches, seen on the pattern, on the 3D garment, in photos and in the tech pack; photos now show buttons and zippers; elastic: pick an edge, then Elastic in the action bar, and it gathers when draped; Shirring adds rows of elastic across a piece; the tech pack lists the elastic per size; moving a piece on the pattern no longer moves it on the avatar; the action bar's buttons are larger, with a short name under each, and it says Nothing picked when nothing is; a piece selected alone, the action bar offers Rotate, Scale, Mirror, Join, Cut out and Delete, no longer in the inspector; the action bar above the pattern and the garment: pick corners, edges, notches or construction lines, as many as you like, and it offers the constraints and dimensions that fit; a dimension added from it opens its value field on the board; drag a dimension's label to move it, click it to type a new value; dimensions can be named and used in formulas; a notch keeps its distance from the start or the end of its edge; corners on a construction line or at a distance from it follow the line; drawing snaps to where two lines cross; every piece's dimensions are shown; more body measurements: neck, shoulders, arm length, upper arm, wrist, back and front waist lengths, waist and hip heights, crotch depth, inside leg, thigh, knee and ankle, estimated until you type your own, in the Body panel under More measurements; the avatar follows the neck, shoulders, arms, back length, hip height, inside leg and thigh; formulas can use them; the shirt's sleeve comes from the arm length and the T-shirt's and shirt's neckline from the neck; with Select, drag a piece to move it, the corners of its frame to scale it and the round handle above it to turn it; Rotate and Scale in the inspector for an exact angle or percentage; the Draw tool also draws on a piece of the pattern, not only on the 3D garment; a shape added from the Shapes menu comes in selected, with the Select tool on; in French, the circle skirt is now called « jupe soleil »; patterns and measurement files from Seamly2D and Valentina come in drafted for their measurements, graded in every size, and can be drafted again for the avatar; the tape measure goes round a slanting sleeve square to it, and measures a length between two points of the garment; the start screen tells you when a new version is out, and the check can be turned off; Installing: a disk image for macOS, one for Intel and Apple silicon, an installer for Windows, and the first launch on recent macOS, through Privacy & Security; the shirt, T-shirt and dress take their shoulders, neckline and armholes from the avatar's body, and their hem fits over the hips: large sizes no longer have drooping shoulders; the avatar now takes the measurements given in every size of the charts, up to the women's 54: large sizes came out too slim and trousers fell off them; changing the size or the body while draping starts the drape again around the new body; trousers end just above the floor whatever the body; garments worn in layers lie close together: a shirt worn under trousers is tucked into them, and nothing shows through a garment worn over another; the trousers template is a little shorter; a selected piece's panel shows its garment's fabric; the Construction line and Tape measure tools have new icons; the tool rail in four sections, drawing, measuring, sewing and trims, with Dart and Pleat under Darts and pleats and Hole a tool of its own; the guided flow's step bar and banner removed; a more natural walk and a new motion, Stand at ease; captured motions whose rest pose lies flat play upright; pieces can be renamed from the inspector; a moving avatar keeps its hands clear of its
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

### Installing
Download the package for your system from app.rustle3d.com:

- **macOS** (one download for Intel and Apple silicon Macs, macOS 12 or
  later): open the `.dmg` file and drag **Rustle3D** onto **Applications**.
- **Windows** (10 or 11, x64; Windows on Arm runs it too): run the
  `-setup.exe` installer. It installs Rustle3D for you alone (no
  administrator rights needed), adds it to the Start menu and, if you
  tick the box, to the desktop. Uninstall it from Settings > Apps.
- **Linux** (x64, with GTK 3): unpack the `.tar.gz` and keep the folder
  where you like.

Rather not install? macOS and Windows also have a portable `.zip`
(`-macos-universal.zip`, `-windows-x64.zip`): unpack it anywhere, a USB
stick too, and open Rustle3D from there. To remove it, delete the
folder. Your projects and settings are kept in your user folder either
way, as with the installed app.

Each package holds the app, `r3d` (the command-line tool, for scripts
and AI agents), the licences and a README. On macOS they are inside the
app: `/Applications/Rustle3D.app/Contents/MacOS/r3d`; on Windows, in the
folder Rustle3D is installed (or unpacked) in.

The packages are not signed yet, so your system asks you to confirm the
first launch:

- **macOS**: open Rustle3D from Applications. macOS says it cannot check
  the app: click **Done** (not Move to Trash). Open **System Settings >
  Privacy & Security**, scroll down to the message about Rustle3D, click
  **Open Anyway** and confirm with your password. On macOS 14 or older,
  right-clicking the app and choosing **Open**, then **Open** again, does
  the same. If macOS says the app is damaged, run `xattr -dr
  com.apple.quarantine /Applications/Rustle3D.app` in Terminal, then open
  it.
- **Windows**: if Windows protected your PC when you run the installer
  (or `rustle3d.exe` from the portable folder), click **More info**, then
  **Run anyway**.
- **Linux**: run `./rustle3d` in the folder.

You only do this once. Each download comes with a checksum in
`SHA256SUMS.txt` on the release page, to check the file arrived intact.

### The start screen
- **What will you make?** Pick a template: T-shirt, shirt (buttoned down
  the front, long sleeves with buttoned cuffs and a stand collar),
  trousers (with a fly zipper), hoodie (jersey, a hood, gathered at the
  wrists and the hem), joggers (jersey, an elastic waist and elastic
  ankles, no fly), jeans (denim, five pockets, a back yoke, a fly and a
  waistband with belt loops), shorts (ending a hand above the knee),
  dress, straight skirt, A-line skirt or tote bag (the easiest start),
  or a blank project. Templates are drafted to the body's measurements.
  For a shirt, the sleeve length includes the cuff; trousers, joggers,
  jeans and shorts are measured from the waist to the hem.
- **Recent projects** show a thumbnail; **All projects** lists the rest.
  You can also open a project from anywhere on disk, or open the projects
  folder.
- The **⋯** button on a project's card renames it, exports it as a .zip
  file (to send it to someone or keep a copy), or deletes it. Deleting
  asks first: tick the box saying you understand, then **Delete**. The
  project folder is deleted for good, not moved to the trash.
- **Import .zip…** adds a project someone sent you as a .zip file and
  opens it.
- Links to app.rustle3d.com for news and updates, and **Support Rustle3D**
  to make a donation.

### Updates
Once a day at most, when it starts, the app asks app.rustle3d.com whether a
new version is out. If one is, a card on the start screen says so (never
while you work on a project): **Download** opens the download page;
install the new version over this one, your projects are kept. **Skip
this version** hides the card until the next version. The app never
downloads or installs anything by itself.

The request only says which version of Rustle3D asks and on which system
(macOS, Windows or Linux): no name, no project, no identifier.
**Check for updates** at the bottom of the start screen opens a small
window: turn **Check for updates once a day** off (or on again), or
click **Check now** to ask at once, even with the daily check off; it
says whether you have the latest version, and offers the download if
not. Offline, nothing happens.

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

### Learn
**Learn** (top right) opens short how-tos. **Show me** points at the part
of the screen where it happens; it never changes your project. A first
tour is shown on the first launch.

### Language
English or French, from the language button in the top bar.

## The body
**Body › Measurements…** opens the measurements: choose a size (women's
or men's presets, babies', girls' and boys') or enter height, bust,
waist and hips. Save your own presets. The garment drapes on the new body: changing the body (or the
size shown, see Sizes) while the garment drapes starts the drape again
around it. The avatar takes your measurements to within a centimetre,
up to a women's 54 or a men's 60; larger, its arms would meet its body,
and it stays smaller than asked (a women's 56 about 5 cm small).

### Children
Choose **Child** at the top of the Body panel and type the child's
**Age** in years (0 to 18), then **Girl** or **Boy**. The avatar takes a
child's proportions at that age (a larger head, shorter legs), then
your height, bust, waist and hips. The presets give common sizes: babies
3, 6, 12, 18 and 24 months, girls and boys 2, 3, 4, 5, 6, 8, 10, 12 and
14 years. Templates are drafted and placed on the child's body: tops
start at its neck, trousers rise to its crotch. A child older than 14
looks halfway between a child and an adult.

Templates made for children (under **Babies and children** in the
library, and on the start screen):
- **Baby bodysuit**: jersey, short sleeves, closed under the crotch
  (sewn: snaps are not available yet), with leg openings.
- **Pyjama**: a top with long sleeves gathered at the wrists and loose
  bottoms with an elastic waist, added as two garments, the top over the
  bottoms. **Pyjama top** and **Pyjama bottoms** add one of them.
- **Jogging suit**: joggers and a **sweatshirt** worn over them, also two
  garments. The sweatshirt (the hoodie without its hood) is also under
  Tops.

A set's sizes are its top's; the bottoms take the same waist and hips
and their own length. Each garment of a set can be changed, coloured
and graded on its own.

### More measurements
Under **More measurements** in the same panel are fifteen more, in three
groups: **Torso** (neck, shoulder width, shoulder length, back and front
waist length, waist height, hip height, crotch depth), **Arms** (arm
length, upper arm, wrist) and **Legs** (inside leg, thigh, knee, ankle).
Each has a small figure showing where the tape goes. You never have to
fill them in: until you type your own, each is estimated from height,
bust, waist and hips (shown in grey, marked "estimated"). Type a value
and press Enter (or leave the field) to use yours; the circular arrow
brings the estimate back. Changing the preset or the four measurements
keeps the ones you typed.

The avatar takes your neck, shoulder width, arm length, upper arm, back
waist length, hip height, inside leg and thigh. The others (wrist, knee,
ankle, front waist length, waist height, crotch depth) size patterns but
do not change the 3D body. When the avatar cannot reach a value you
typed, the row says what it measures. Templates use them: the shirt's
sleeve is your arm length less 2 cm (cuff included), the T-shirt's 40 %
of it, and both necklines follow your neck.

### Your own body: a scan or a character
**Body › Import body…** drapes on a body of your own: a 3D scan of a
client, a character made in another program. Pick an OBJ, glTF, GLB, PLY
or STL file (a glTF must keep its data in one file: save it as GLB if
it comes with a .bin; PLY and STL are what most body scanners save). You must have the right to use it; it is copied
into the project's `avatars` folder and never leaves it.

A body in another format, such as FBX, opens in Blender (free): use
**File › Import** for its format, then **File › Export › glTF 2.0**
and choose **glTF Binary (.glb)**. Keep **Skinning** ticked if the body
has a skeleton.

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
the measurements taken on yours.

An imported body moves like the built-in one: **Motion** makes it walk,
turn, raise its arms, stand at ease or play a captured motion. It stands
as in its file until the motion starts, then comes into it within a
second. If the file has a skeleton (a game character), the body moves
with it; otherwise Rustle3D places a skeleton through the arrangement
points (the Check body window says which). On a body standing in a
T pose, move the shoulder, elbow and wrist points onto the raised arms
first: the skeleton follows them, and an elbow that bends in the wrong
place means its point is off.

## The pattern (2D)

The left view is the pattern board: drag an empty spot to pan, scroll or pinch to zoom.
The tool rail sits on its left, in four sections set apart by a line:
drawing, measuring, sewing and trims. A button with a small corner opens
a menu of related tools (the last one used stays on the button).

| Tool | Key | What it does |
|---|---|---|
| Select | V | Click a piece, a corner, an edge, a notch or a construction line (⌘-click or ⇧-click adds more, or takes one away); drag a piece to move it; drag a corner or an edge to reshape the piece; a selected piece has a frame: drag a corner of it to scale the piece, the round handle above it to turn it (⇧ turns by 15° steps, or scales both ways at once); a picked curved edge, or a corner between curves, shows its handles: drag them to bend the curve (a smooth point keeps its two handles lined up) |
| Shapes → Rectangle | R | Adds a rectangular piece; like every shape, it comes in selected, with the Select tool on |
| Shapes → Ellipse | E | Adds a circle or an ellipse of the width and height you type |
| Shapes → Circle skirt | | Adds the pieces of a full, half or quarter circle skirt from the waist and the skirt's length |
| Polygon | P | Click the corners of a new piece; click the first corner or press Enter to close; Esc cancels |
| Curve | C | Draws a piece with curves: click for a corner, drag for a smooth point; click the first point or press Enter to close |
| Edit curves → Bend | B | Click an edge, then drag its two handles to bend it |
| Edit curves → Straighten | L | Click a curved edge to make it straight |
| Edit curves → Add point | A | Click an edge to add a point there (the curve keeps its shape) |
| Edit curves → Arc | | Click an edge, type how far it bulges: it becomes a circular arc |
| Cut | K | Click (or drag, for curves) a line across a piece, from outside to outside, then Enter: the piece becomes two pieces sewn together along the line |
| Hole | | Select a piece, click the corners of a hole inside it, Enter: the hole is cut out of the cloth |
| Draw | D | Drag across a piece, on the pattern or on the 3D garment: the line lands on the pattern |
| Construction line | G | Click two points: a line across the board to draw against (never cut or sewn); click it and press Delete to remove it |
| Dimension | T | Click an edge, two corners or two edges, then where the dimension goes; type its value (or a formula) right there |
| Notch | N | Click an edge where the notch goes |
| Darts and pleats → Dart | | Click an edge where the dart goes, type its width and length: it is drawn, folded and sewn |
| Darts and pleats → Pleat | | Click an edge where the pleat starts, type its depth (and how far it is stitched down), knife or box: it is folded and sewn |
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

**Moving, turning and scaling a piece**: with Select, drag the piece
(away from its corners and edges) to move it; where it lies on the
pattern does not change where it hangs on the avatar. A selected piece shows a
frame: drag one of its corners to scale the piece (the opposite corner
stays put; hold ⇧ to keep its proportions), or the round handle above
it to turn it about its middle (hold ⇧ for 15° steps). For an exact
value, **Rotate** and **Scale** in the action bar (the piece selected,
nothing on it picked) ask for an angle or percentages across and up and
down; **Mirror** there adds a mirrored copy that follows the piece, and
**Delete** deletes it. Lines, pins, buttons, holes and the
grainline go with the piece, and each change is one step you can undo.
A linked mirror copy is turned and scaled through its source. A piece
whose dimensions or constraints could not follow (a horizontal edge
turned 30°, a dimension with a formula when scaling) says so: remove
them first.

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
between them, their notches, internal lines, pins, elastics,
topstitching, interfacing, quilting, buttons, zippers, dimensions, size
grading (when the project has the same sizes) and mirror links among
them. Pasted
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
Pick what to constrain on the board with Select: corners, edges,
notches or construction lines, on one piece or several; ⌘-click (or
⇧-click) adds one, or takes it away. The **action bar**, the row above
the pattern and the garment, says what is picked (greyed **Nothing
picked** when nothing is) and offers what fits, each button named under
its icon (hover it to see what it does):
- a piece alone (nothing on it picked): **Rotate**, **Scale**,
  **Mirror**; **Join**, **Cut out** (see below); **Delete**;
- several pieces: **Delete**;
- one corner: **Fixed** (it stays put while the rest adjusts),
  **Smooth** (no corner there: the curves pass through it smoothly) or
  **Corner**, **Remove point**;
- two corners: **Width** (across), **Height** (up and down),
  **Distance** (straight), **Joined** (at the same place, for corners of
  two pieces);
- one edge: **Length**, **Horizontal**, **Vertical**;
- two edges: **Angle**, **Same length**, **Parallel**, **Square**;
- three edges or more: **Same length** or **Parallel**, all like the
  first one picked;
- a corner and an edge: **On the edge**;
- a notch: its distance **From the start** or **From the end** of its
  edge (it keeps it when the edge changes), **Remove notch**;
- a construction line: **Level**, **Upright** (turned about its first
  end), **Remove line**;
- a corner and a construction line: **Distance to the line**, **On the
  line**. Construction lines are the frame you draft from: move a line
  and the corners on it, or at a distance from it, follow. Moving a
  piece takes along the lines only its corners use.

A dimension (width, height, distance, length, angle, a notch's distance,
a distance to a line) keeps the value the drawing has, and its value
field opens on the board at once: type the value (or a formula) and
press Enter, or Esc to keep it.

With **Dimension** (T) you put a dimension straight on the pattern:
click an edge (its length), a corner then another corner (the distance
between them) or two edges (the angle between them), then click where
the dimension goes. Between two corners, pulling it above or below them
gives a **width**, beside them a **height**, in between the straight
distance; the value follows the pointer. A field opens where you
clicked: type the value (or a formula) and press Enter, or Esc to keep
the value it has.

Every piece's constraints and dimensions are drawn on the board (the
selected piece's brighter). Drag a dimension's label to move it (only
the label moves); click it to type a new value. The selected piece's
constraints are listed under **Constraints** in the inspector, a
dimension under its name if it has one. Type a new value
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
- **Dart** (Darts and pleats group, with the sewing tools): click the edge where the dart goes and type
  its width at the edge and its length. It is drawn, folded and sewn in
  one step: when the garment drapes, the dart closes.
- **Pleat**: click the edge where it starts, type its depth and choose
  knife (one fold) or box (two folds facing away). A knife pleat takes up
  twice its depth along the edge. With "Stitched down", the folds are also
  sewn that far into the piece. Notches mark the fold lines.
- **Hole** (with the drawing tools): select a piece, then click the corners of the hole inside it
  and press Enter. There is no cloth in a hole; its edges can be sewn
  (a facing) like any other.
- **Join or cut**: with a piece selected, **Join** or **Cut out** in the
  action bar lists the other pieces: pick one. **Join** makes one piece of the two (they must
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
`bust`, `waist`, `hips`, and the others by their names: `neck`,
`shoulder_width`, `shoulder_length`, `back_waist_length`,
`front_waist_length`, `waist_height`, `hip_height`, `crotch_depth`,
`arm_length`, `upper_arm`, `wrist`, `inside_leg`, `thigh`, `knee`,
`ankle`), numbers with their unit (`mm`, `cm`, `in`),
`+ − * /`, parentheses, `min` and `max`, and your own **variables**
(nothing selected: the **Variables** section of the inspector; **+**
adds one, like `ease = 2 cm`, click one to change it). A length needs a
unit: `waist / 4 + 10` is refused, `waist / 4 + 10 mm` is fine; the
message says what is wrong.

A dimension can have a **name** (the label button next to it in the
inspector's list): formulas then use it like a variable, its formula if
it has one, otherwise its value. Name the armhole's length `armhole`
and give the sleeve cap's length the formula `armhole + 1.5 cm`: the
cap follows when the armhole changes. Renaming a dimension changes the
formulas that use it.

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
  cloth, a zipper as a brass band, in the 3D view and in photos.
- The shirt template has a yoke across the back, its seam topstitched,
  and its collar and cuffs are interfaced (see Construction details).
- The shirt template comes buttoned: buttons down one front, buttonholes
  on the other (left over right on a men's body, right over left
  otherwise), one on each cuff. The trousers have an 18 cm fly zipper,
  closed.

## Elastic and shirring
- **Elastic**: pick an edge of a piece, then **Elastic** in the action
  bar. Say how long the elastic is when relaxed, as a share of the edge
  (75 % unless you change it: a 40 cm edge takes a 30 cm elastic), and
  how wide it is (a wider one pulls harder). The pattern shows it as a
  green zigzag along the edge.
- **Shirring**: pick the edge the rows follow, then **Shirring**: how
  many rows, how far the first is from the edge, the gap between rows
  and the elastic's relaxed length. Each row runs across the whole
  piece.
- Select a piece: its **Construction details** card lists its elastics,
  each with its relaxed length and width. Drag **Relaxed length** to
  gather more or less, and remove one with its ×.
- In 3D, the edge gathers to the elastic's length while the garment
  drapes, the cloth beside it in small folds. On the body, an elastic
  shorter than the arm or the waist it goes round stays stretched on
  it, as a real one does.
- The elastic keeps its share of the edge when the edge changes or in
  another size; the tech pack lists the elastic each garment takes, by
  width, and by size when the pattern is graded.

## Interfacing
- Select a piece alone, then **Interfacing** in the action bar: it is
  fused on the whole piece (a collar, a cuff). Pick an edge instead,
  then **Interfacing**, for a band along it as wide as you say (a
  facing, a waistband, a front band). Choose light, medium or heavy.
- The pattern hatches the interfaced part. The **Construction details**
  card lists it with its area; remove it with its ×. Agents can also
  fuse one inside a closed line drawn on the piece, and choose woven,
  non-woven or knit (which stretches across with the cloth).
- In 3D the interfaced cloth is heavier and much stiffer: a band of
  heavy interfacing stands where the bare fabric folds over.
- Two interfacings on one piece cannot overlap. The tech pack's bill of
  materials lists the interfacing by kind and weight with its area.

## Quilting
- Select a piece alone, then **Quilting** in the action bar: the
  distance between the stitched lines, the padding's thickness, and
  diamonds or channels. Agents can also set the lines' angle, the
  padding's weight, the stitch length, the thread's colour, and quilt
  only the inside of a closed line drawn on the piece.
- The pattern shows the stitched lines as stitches. In 3D and in photos
  the piece puffs out between them, flat on the lines and at its edges;
  it is heavier, thicker and stiffer, and folds along its lines.
- The **Construction details** card lists it; remove it with its ×.
- The tech pack lists the padding (thickness, weight, area) in the bill
  of materials and each quilting in the sewing section.

## Topstitching
- Pick an edge of a piece, then **Topstitch** in the action bar: how
  many rows (1 to 3), how far the first is from the edge, the gap
  between rows, the stitch length, and the thread (sewing thread, or
  the thicker topstitching thread). The rows run inside the piece and
  stop at its outline.
- To topstitch beside a seam (a yoke, a shoulder), pick the edge of the
  piece the stitches show on.
- The thread is a darker shade of the fabric unless you choose its
  colour (agents can set any colour). The pattern shows the stitches at
  their length in the thread's colour (zoomed out, where they would be
  too small to tell apart, as a line); the 3D garment, its pictures and
  photos show them on the cloth.
- The **Construction details** card lists a piece's topstitches with
  their thread colour; remove one with its ×.
- The cloth under the stitches folds a little less, as a real
  topstitched edge stays flatter.
- The tech pack lists each topstitch to sew: piece, length, rows,
  distance from the edge, gap, stitch length and stitches per inch,
  thread and colour; and each elastic with its relaxed length.

## Fabric and garments
With nothing selected, the panel on the right lists the project's
garments: pick each one's fabric (cotton, denim, jersey, silk, wool).
A selected piece's panel shows its garment's fabric too; changing it
there changes every piece of that garment.
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
Add one from the pattern library (**From the library…**, below the
garments) or a blank one (**Add a garment**): it comes as a garment of
its own, worn over the others. The list goes from
the body out; the arrows wear a garment under or over its neighbour. To
move a piece to another garment, pick it in the piece's **Garment**
menu. Deleting a garment deletes its pieces too (undo brings them back).
To see the others without one of them, take it out of the drape (the
layers button on its card): it stays in the project and on the
pattern, but is no longer draped nor shown on the avatar; the same
button puts it back.

Garments drape in that order, as close together as their fabrics are
thick: a garment stays over the ones under it. To tuck a shirt into
trousers, wear it under them (the arrows); worn over them, it hangs
outside.

### The pattern library
**From the library…**, below the garments, or **Library** in the top
bar, opens the pattern library:
the built-in patterns by kind (tops, bottoms, dresses and skirts,
accessories, babies and children), each with a picture of its pieces, and **My patterns**,
the ones you saved. Click one to add it to the project: it comes as a
new garment, worn over the others, drafted for the avatar (and graded
in every size of a size range).

**Library** on the start screen, next to New, opens your own patterns
without a project: rename, export, delete or import them there.

To keep a garment for other projects, click the bookmark button on its
card (**Save to library**), give it a name and a kind. Everything comes
with it: pieces, seams, notches, lines, elastics, topstitching,
interfacing, quilting, buttons, zippers, dimensions and formulas,
fabric, colour, print and notes. Added to another project, a garment
that came from a template, a Seamly2D or Valentina pattern, or that is
drawn with formulas is drafted again for that project's body, keeping
the changes you made to its pieces; a garment drawn freehand comes as
drawn. Your patterns are shared by every project (they are kept with
your body presets and motions); the **⋯** button on one renames it,
exports it as a .zip file to share it, or deletes it (projects it was
added to keep their garments). **Import…** adds a pattern someone sent
you.

## Sizes
A pattern can come in a range of sizes, for example 36 to 52. It is
drawn in one size, the **base size**; each point says how it moves from
size to size (its grade), and the other sizes follow.
- With nothing selected, **Sizes** › **Add a size range** in the panel on
  the right: pick a chart (women's 32 to 56 or men's 44 to 60, European
  sizes; girls' and boys' by age, 2 to 14 years; babies' 3 to 24
  months), the smallest and largest size, and the base size. Each size
  has a body with its measurements, built from your avatar; in a
  children's range each size is also a year (or a few months) older or
  younger.
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
  while the garment drapes: **Walk** on the spot at an easy pace, as
  on a treadmill (the feet roll from heel to toe and never slide, the
  hips rise and fall smoothly, the arms swing loosely), **Stand at
  ease** (the weight shifting from one leg to the other, breathing),
  **Turn** half round and back in small steps, the head leading,
  **Raise arms** high above the head in a V and down; **Stand still**
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
- **Tape measure** (with the measuring tools). Its card has two modes:
  - **Round**: click on the draped garment. The tape goes round the
    garment there and is drawn on it: level round the body, square to
    the sleeve on a slanting sleeve, as you would hold a tape. The card
    gives the height of the point, the **garment's** circumference
    (along the cloth, folds included), the **body's** inside it (as a
    tape around the body reads it), the **ease** between them and the
    **gap** between the cloth and the body where you clicked. Where the
    garment does not go all round (an open front, a hem), there is no
    ease.
  - **Length**: click two points of the garment, for example the
    shoulder and the hem. The way from one to the other across the
    cloth is drawn on it; the card gives its length **along the
    cloth** (folds included), with the **tape pulled taut** (it spans
    the folds) and in a **straight** line. When the cloth does not join
    the two points (two garments, an opening between), only the
    straight length is given.

  The tape is taken when you click: click again after the drape has
  moved. Drag to turn the view; the × or another tool puts the tape
  away.
- The drape runs on the graphics card, about five times faster than on
  the processor; the status bar says **GPU** and the time of a step.
  Without a usable graphics card, or if it fails, the drape carries on
  from where it was on the processor (**CPU**). To use the processor
  anyway, start the app with the environment variable `R3D_SOLVER=cpu`.

## Photos
**Photo** (the camera button at the top right of the 3D view) opens the
photo scene: a panel takes the inspector's place, and the 3D view shows
the photo as it will be, getting cleaner within a few seconds. Drag in
the view to turn around the garment and scroll to come closer or move
away: the photo follows at once (a little blurred while you move, sharp
as soon as you stop). The cloth stays as it was when you opened the
scene. In the panel:
- **Lighting**: **Studio** (a main light, a softer one on the other side
  and a light from behind, on a light grey backdrop), **Soft** (large
  soft lights on a white backdrop, few shadows, like a shop's product
  photos) or **Dramatic** (one hard light on a dark backdrop), or
  **Place**: a real place lights the garment and shows behind it.
- **Turn** turns the lights, or the place, around the garment (to bring
  a window or the sun where you want it); **Brightness** makes them
  brighter or darker.
- **Place size** (with Place): how far the place's walls stand from the
  garment at floor level, from 1 to 100 m. About 3 m (the usual) suits
  a room; take more outdoors. If the bottom of the place looks stretched
  or torn, make it smaller. The walls never come closer than the camera:
  a very small place shows when you scroll in close.
- **Lens**: from 18 mm (wide: more of the place around, a stronger
  perspective) to 200 mm (narrow: less around, a flatter look), as on a
  camera. Use it with the scroll wheel to frame the garment.
- **Default settings** puts the turn, brightness, place size and lens
  back as they were at first (0°, × 1, 3 m, 38 mm).
- **Size**: as the 3D view, 1920 × 1080, 1080 × 1350 (portrait, for
  social media), 2048 × 2048 or 3840 × 2160 (4K). The view shows the
  photo in that shape, so you see what will be in it.
- **Quality**: **Quick** (a first look in seconds), **Good** (clean
  enough to show, the usual choice) or **Best** (for print and close-ups;
  it takes several times longer).
- **Advanced** (folded): samples per pixel (more is cleaner and slower),
  light bounces, and whether to remove the remaining noise.

**Place** uses an HDR image: a panorama of the light all around a
place, in the Radiance format (`.hdr`, as free sites such as Poly Haven
offer them). Pick one of the five built-in photo studios (Ferndale,
Kontrast, Neon, Pav, Wooden) or **Add .hdr…** to bring your own; it is
copied into the project's `environments` folder, so the project keeps
it. The garment stands on the place's floor, which shows its shadow;
the lights, windows and colours of the place light the cloth and show
in shiny fabrics. Every image is first brought to the same exposure,
so a sunny outdoor scene and a dim room both come out well lit. The
place and the lights keep their position relative to your view.

**Render** makes the photo at the chosen size and quality, framed as
the view showed it. It gets cleaner as it renders, with its progress at
the bottom of the view. **Stop** keeps the image as it is; **Save
image…** saves it as a PNG file; **×** puts it away and the scene comes
back (so does any change in the panel). **×** at the top of the panel
closes the scene: the 3D view gets its own angle back. The scene (the
lighting, turn, brightness, lens, size and where the camera stands) is
saved with the project, for the next photos and for agents; Undo
brings the previous one back. The garment's colour in the photo is its
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
  bones, with the walk, turn, raise-arms and stand-at-ease motions and
  the project's captured motions as animations, so it moves with a
  character. An imported body is skinned too, standing as in its file.
  **Include the body** adds the body too.

The garment is exported as the 3D view shows it, put back in the body's
standing pose when skinned. Each garment is one mesh per level of
detail, named `Shirt_LOD0`, `Shirt_LOD1`…, and its pieces are laid out
in one image (with the print baked into it). The file is in metres with
Y up, as glTF wants: the engines convert it themselves.
- **Godot 4**: drop the `.glb` into the project and open it: the
  skeleton, one mesh per level (show one, or set visibility ranges) and
  an AnimationPlayer with `walk`, `turn`, `raise_arms`, `idle` and your
  captured motions (named after their files).
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

### Seamly2D and Valentina
Patterns made in **Seamly2D** (`.sm2d`) or **Valentina** (`.val`) come
in drafted for their measurements, the way you drafted them:
- **Import** the pattern: the measurement file it uses (`.vit`, `.vst`,
  `.smis` or `.smms`) comes along when it is in its usual place, next to
  the pattern or where the pattern says. Without a measurement file, the
  pattern is drafted for the avatar's height, bust, waist and hips.
- The pieces keep their curves, seam allowance, notches, grainline and
  internal paths; a piece cut on the fold is unfolded. They make a
  garment of their own, named after the file.
- A few drafting tools are not supported yet (parallel curve, graduated
  curve, elliptical arc by length, joined pieces): the pieces that use
  them are left out, and the message says so.
- With **sizes**, each size is drafted with its own measurements.
- **Draft again for the avatar** (on the garment, in the garment list of
  the panel on the right) runs the pattern again with the avatar's
  measurements. The changes you made to the pieces are kept.
- **Import** also takes a measurement file on its own: the avatar takes
  its height, bust, waist and hips, the other measurements become
  **variables** for your formulas, and a multisize file sets the sizes.

## AI agents
Rustle3D can be driven by an AI agent through MCP (Model Context
Protocol):
- **In the app**: the **Agents** button opens a short guide in three
  steps. Turn on **Allow AI agents**; copy the server's settings for
  your agent (a JSON block for most desktop assistants, or the command
  line for Claude Code or Codex; it names `r3d` by its full path, the
  command-line tool installed with the app) and restart the agent; then
  ask it to work on your project. For Claude Desktop, and other
  assistants that install MCP bundles, choose **Claude Desktop** and
  **Save rustle3d.mcpb**: open the file (or drop it into the
  assistant's Settings > Extensions) to install Rustle3D as an
  extension. It runs `r3d` from where the app is installed: save it
  again if you move the app. Several agents can be connected at once
  (an assistant's chat and its coding sessions, say), taking turns on
  the open project; the button reads "Agent connected" while one is
  attached, even during a long action such as a drape. You see every
  change live and can undo it, or disconnect the agents from the
  guide.
- **Without the app**: `r3d mcp --project My.r3d` serves a project
  headless. Drapes made there (and with `r3d simulate`) run on the
  processor, so they give the same result every time. A drape can make
  the avatar move: `r3d simulate My.r3d --frames 300 --motion walk`
  (`walk`, `turn`, `raise_arms`, `idle`, or a `.bvh` file, copied into the
  project's `motions` folder); pictures and exports then show the
  body and the garment at the last frame.
- An agent can look at its work: a picture of the pattern, of the 3D
  view, or a photo like the app's (`render_photo`). From the command
  line: `r3d render My.r3d --view photo --out photo.png`, with
  `--environment neon` (or a `.hdr` file in the project, copied in with
  `r3d environment My.r3d room.hdr`), `--turn`, `--brightness` and
  `--place-size`. A
  photo taken this way uses the project's photo scene, as set in the
  app, for whatever it is not told: the light, the turn, the
  brightness, the camera and the size. An agent can set the scene too
  (`photo_set_scene`).
- An agent can use the pattern library: `library_list` lists the
  built-in patterns and yours, `template_apply` and `library_add` add
  one, `library_save` saves a garment. From the command line, scripts
  run by `r3d run` reach the app's library, or another folder with
  `--library`.
- An agent can check the fit: `r3d render My.r3d --view 3d --fit-map
  strain --out fit.png` colours the drape (or `gap`), and `r3d fit
  My.r3d` lists each piece's strain, the tight spots and the ease at
  the chest, waist and hips (`fit_report`, `fit_tape` for an agent).
- When a drape looks wrong, `r3d diagnose My.r3d` (`drape_diagnose`
  for an agent) says where and why: the most stretched and crumpled
  places, cloth inside the body or lying on other cloth, seams left
  open. A region of a piece or of the drape can be given.
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
Patterns come in from DXF, SVG, PDF, Seamly2D and Valentina and go out as SVG, PDF and
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
- On a baby under 2 years old, whose arms rest against its chest, the
  cloth may dip into the armpit. A child's walk is an adult's, made
  smaller.
- The folds of one garment keep about 2 cm apart in the simulation (the
  layers of different garments lie close together), so they look a
  little puffy; gathers along an elastic are the exception.
- Game export: the levels of detail are separate meshes (no automatic
  switching inside the file).
- The avatar walks on the spot (not along a path) and raises its arms in
  a V above the head but not straight up. A captured motion that travels
  or turns is played on the spot, facing forward; its fingers are not
  used. An imported body's fingers and extra bones follow its hand or
  limb, and its hands and feet keep the turn of the forearm and shin.
- An imported body is the same in every size of a size range. Its
  arrangement points are guessed for arms down or in an A pose: on a
  T pose, move the shoulder, elbow and wrist points. Compressed glTF
  (Draco, meshopt) cannot be read; files over 2 million triangles are
  refused, and above 300 000 the views and the drape slow down.
- Quilting is simulated as one layer (shell, padding and lining
  together): its puffs are drawn on the garment, not simulated, and a
  quilted piece is meshed finer, so a large one drapes a little slower.
  Holes are cut on their line (no seam
  allowance inside them), and a mirrored or symmetric piece cannot have
  holes.
- Fit maps show the stretch of the cloth, not the force in it (a stiff
  woven pulls harder for the same stretch). The body's circumference is
  read inside the garment's loop: at a sleeve's hem cut on a slant there
  may be none. Near the armpits, the round tape may take in the sleeves
  or pass under the arms: click a little lower for the chest.
- Constraints apply to corners and edges of the outline, notches and
  construction lines, not to internal lines. A construction line never
  moves to satisfy a constraint: the corners follow it. A notch's
  distance is along its edge, not straight to a corner. A symmetric
  piece's axis is straight.
- Sizes: notches keep their distance along the edge in every size, and
  a grade is set per point, not per edge, in the app. The avatar's
  shoulders do not widen with its build: the shirt, T-shirt and dress
  widen theirs by about 0.4 cm per size on their own. A women's 52 shirt
  parts a little at the front below its last button, over the belly.
  A template's pieces graded after they were laid out keep their place
  on the board: the largest sizes of neighbouring pieces can touch.
- Buttons and zippers: a copied piece is pasted without its buttons
  (and its elastics and topstitches). The two pieces a button holds may pass
  through each other near the button (they are allowed to lie on each
  other there). Agents can place a zipper on part of an edge; in the app
  the Zipper tool takes whole edges.
- Prints colour the fabric only: no relief of the weave yet, and the
  same print shows on both sides of the cloth. Transparent parts of an
  image are shown as they are, not see-through. The marker does not
  match stripes or checks between pieces yet.
- Photos: the cloth is a single thin layer (no thickness at the hem).
  HDR images in OpenEXR (`.exr`) cannot be read yet: convert them to
  `.hdr`. A place is shown as a room of the size you choose: a picture
  taken in a long hall or outdoors, near a wall, looks a little bent
  whatever the size. With a place, the floor shows the shadow but not the light
  bounced off the garment, and things standing on the place's floor in
  the image (a chair, a tripod) look flattened onto it.
- Import: a PDF whose pages each hold a different part of the pattern
  (rather than the same drawing shifted) cannot be put back together;
  binary DXF files and protected PDFs cannot be read. Grading rules are
  not read: pieces come in at the size they are drawn.
