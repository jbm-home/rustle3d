# 25 — Textures and prints

Provenance: maintainer's functional brief (ROADMAP M21: UVs from the pattern pieces aligned with the grainline; the user's own textures and a default set; shown in the 3D view, the photo renderer and the exports; a striped shirt whose stripes follow the grainline on every piece); general garment-making knowledge (a print or a woven stripe is laid out on the fabric before cutting, so each piece shows it along its own grainline, at the fabric's real scale; checks, stripes and dots are matched by placing pieces on the fabric); open standards: glTF 2.0 (`baseColorTexture`, samplers, embedded PNG/JPEG images), Wavefront OBJ/MTL (`map_Kd`), PNG (ISO/IEC 15948), JPEG (ITU-T T.81 / JFIF); mip-mapping (Williams, *Pyramidal Parametrics*, SIGGRAPH 1983).

## Status (2026-11-19, M21)

Implemented: a print per garment (four built-in prints drawn by our own
code, or the user's PNG/JPEG copied into the project), its repeat size
and angle, a shift and turn per piece; texture coordinates from the
pattern along each piece's grainline; shown in the app's 3D view, the
headless 3D renders, the path-traced photos, the board's piece fills, and
the glTF (embedded image) and OBJ (MTL with `map_Kd`) exports.

Not yet: placing pieces on the print by dragging (stripe matching
across seams by hand), a print in the marker (the marker still nests
pieces as on a plain fabric), normal or roughness maps, prints with
transparency (the alpha channel is ignored).

## User story
As a designer, I make a shirt in a blue-and-white stripe: the stripes
run along the grainline on the fronts, the back and the sleeves, at the
fabric's real width (one stripe every 2 cm), in the 3D view, in a photo
and in the glTF I send to a client. Then I load a photo of a printed
cotton I bought, say one repeat is 32 cm wide, and see the shirt in it.

## Behavior

### A garment's print
- A garment has at most one **print**: a built-in one or an image, its
  **repeat** (the width and height of one tile of the image on the
  fabric, mm), and an **angle** (degrees, the print turned from the
  grainline, counter-clockwise; 0 = the image's up along the grain).
  No print: the garment's plain colour, as before.
- **Built-in prints**, drawn by our code in the garment's colour on an
  off-white ground, so they follow `garment.set_color`:
  - `stripes`: a stripe of colour and one of ground, along the grain
    (default repeat 20 mm);
  - `gingham`: crossing bands of colour, darker where they cross
    (default repeat 12 mm);
  - `dots`: dots on the ground in staggered rows, two per repeat
    (default repeat 16 mm);
  - `twill`: a 2/1 twill weave, diagonal ribs of the colour and a
    lighter thread (default repeat 4 mm).
- **Images**: a PNG or JPEG file in the project folder (relative path,
  no `..`). The app and `r3d texture` copy the file the user picks into
  `textures/` (a second file with the same name is numbered), so the
  project stays self-contained (spec 06). The image is shown as it is
  (the garment's colour is not applied to it); its alpha is ignored.
  Images above 2048 pixels a side are halved until they fit for display
  (16384 at most are read); the exports carry the original file. A file
  that is missing or cannot be read shows the garment's plain colour,
  without an error (the inspector's preview stays empty).
- Repeat: 1–5000 mm each way; when only the width is given, the height
  keeps the image's proportions (built-ins are square). Angle: any
  finite value, kept in (−180, 180].

### On each piece
- Texture coordinates come from the pattern: a point of a piece at `p`
  (mm, on the board) maps to the fabric's frame turned by the piece's
  grainline angle plus the print's angle plus the piece's own turn:
  `up = (cos θ, sin θ)`, `right = (sin θ, −cos θ)`,
  `u = (p − shift)·right / width`, `v = −(p − shift)·up / height` (in
  repeats; the image's top toward `up`). So a stripe that runs along the
  grain runs along every piece's grainline, at the same physical scale
  on every piece and in every size.
- A piece may **shift** (mm) and **turn** (degrees) the print on itself
  (`panel.set_print_placement`), to centre a motif on a front or match
  stripes across a seam. Linked mirror copies keep their own shift and
  turn. Board coordinates are those of the piece's outer face, so a
  mirrored pair shows the print the right way round on both.
- Without a print, exports still carry texture coordinates along the
  grainline, one unit per metre of fabric.

### Where it shows
- The app's 3D view and headless renders (`r3d render --view 3d`,
  `render_3d`): the print on the cloth, mip-mapped, lit as the fabric
  (sheen over it). A fit map replaces it while shown.
- The photo renderer: the print is the cloth's albedo at each hit; the
  denoiser is guided by it, so the print stays sharp.
- The board: each piece is filled with its garment's print, faded like
  the plain fills, so stripes can be checked against the grainline.
- glTF: one image per textured garment (the original file, or a PNG of
  the built-in), a repeating sampler, `baseColorTexture`, base colour
  factor white. OBJ: an MTL file next to the OBJ (one material per
  garment with `Kd` and, when textured, `map_Kd`) and the texture files
  beside it.

### Commands
- `garment.set_print` (`garment`, `print`: `{"builtin": "stripes"}` or
  `{"image": "textures/liberty.jpg"}`, `width_mm`, optional `height_mm`,
  `angle_deg`): sets or replaces the print. One undo step; merges while
  the same garment's repeat or angle is dragged.
- `garment.remove_print`: back to the plain colour.
- `panel.set_print_placement` (`panel`, `offset_mm`, `angle_deg`).
- `project.summary` lists each garment's print, `panel.get` each
  piece's placement.
- Adding an image is not a command (commands do not touch files): the
  CLI's `r3d texture FILE` and the app check the file, copy it into
  `textures/` (`Workspace::adopt_texture`), then run `garment.set_print`;
  agents over MCP put the file in the project folder and name it. The
  app's previews come from the bridge (`Engine::print_preview`).

## UX
- In the garment's card of the inspector, under the fabric swatches, a
  **Print** row: None, the four built-ins (each shown as a small swatch
  in the garment's colour) and **Image…** (a file dialog for PNG/JPEG).
- With a print: a preview tile of one repeat, the repeat's width and
  height (cm) and the angle (°) in number fields.
- With a piece selected, its own **shift** (x, y in cm) and **turn**
  (°) for the print.

## Acceptance criteria
- On a shirt with stripes, the texture direction on every piece's cloth
  triangles follows that piece's grainline (test: the gradient of `v` is
  along the grain, within 1°), and one repeat measures `height_mm` on the
  pattern.
- `garment.set_print`, `garment.remove_print` and
  `panel.set_print_placement` round-trip apply → undo → redo; invalid
  repeats, paths (absolute, `..`, other extensions) are refused with
  `invalid.*` codes.
- Schema v16 adds the print (migration from v15, golden file v16).
- The glTF export validates (Khronos validator, when available) and
  carries the image; the OBJ export writes an MTL with `map_Kd`.
- A headless 3D render and a photo of the striped shirt show stripes;
  a missing image renders in the plain colour without an error.
