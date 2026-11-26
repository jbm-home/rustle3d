# 35 — Environment lighting for photos

Provenance: maintainer's functional brief (2026-11-26, ROADMAP M42: a
photo render lit by an environment the user brings, in Radiance `.hdr`,
next to the three built-in studios; the environment seen behind the
model with a floor that only catches shadows; a turn and a brightness;
the file copied into the project's `environments/` folder; a built-in
set of CC0 environments); the Radiance picture format (Ward 1991, "Real
Pixels", Graphics Gems II, open and documented with Radiance); image-based
lighting with importance sampling of a latitude-longitude map by a
piecewise-constant 2D distribution (Pharr, Jakob and Humphreys,
*Physically Based Rendering*, 3rd ed., §13.6.7 and §14.2.4); multiple
importance sampling (Veach and Guibas 1995); CC0 environments from Poly
Haven (licence recorded in `assets/environments/README.md`); the photo
scene from the maintainer's feedback on M42 (2026-11-26: the turn and
brightness were set blind, before the render; a zoom was missing; set
the scene with the result shown in the 3D view, the image left in view
while setting it; the scene saved with the project and used by the
studios too); focal lengths on a 24 × 36 mm frame (general photography
knowledge).

## Status (2026-11-26, M42, photo scene in M42b)

Implemented: everything below.

Not yet: OpenEXR files; light bounced off the garment onto the floor;
objects standing on the picture's ground stay flattened onto the floor.

## User story
As a designer, I light the photo of my garment with a real place (a
studio, a room, outdoors) that I bring as an HDRI file or pick from a
built-in set, and see the garment standing in that place with its
shadow on the ground.

## Behavior
- **Environment**: instead of one of the three studios, a photo can be
  lit by an environment: a latitude-longitude (equirectangular) picture
  of the light all around, in Radiance `.hdr` (RGBE; flat, old or new
  run-length encoding; stored top-down or bottom-up; at most 8192
  pixels a side). It is either:
  - a **built-in** environment, by name: `ferndale`, `neon`, `pav`,
    `kontrast`, `wooden` (five photo studios from Poly Haven, CC0, 1024
    × 512, shipped inside the application);
  - a **file** in the project folder (relative path, no `..`), usually
    in `environments/`. The app and `r3d environment FILE` check the
    file and copy it there (a second file with the same name is
    numbered, an identical one reused), so the project stays
    self-contained (spec 06). Agents over MCP put the file in the
    project folder and name it.
- The environment, like the studio, is part of the project's **photo
  scene** (below), saved in the project document.
- **What the photo shows**: the environment behind the model, sharp (as
  the camera sees it), and a floor at the model's feet (height 0) that
  shows the picture's ground as seen from 1.6 m above the model (so the
  model stands on it, and it meets the horizon far away) and catches
  the shadows: it darkens where the model and the garment hide the
  light from it. The floor shows nothing else (no reflection, no colour
  bleeding) and is invisible to everything but the camera. The
  environment lights everything: the garment, the avatar, the shadows,
  the reflections (no soft boxes).
- **Placement**: the environment keeps its place relative to the camera,
  as the studios do: at turn 0 the picture's centre is behind the model
  as seen from the camera. **Turn** (degrees, any finite value) turns it
  counter-clockwise seen from above, to bring a window or the sun where
  it should be. The turn turns a studio's soft boxes around the model
  the same way.
- **Exposure**: the environment is scaled so that its mean luminance
  over the whole sphere is 0.8 (a lit studio), so bright outdoor
  pictures and dim rooms come out exposed alike; then multiplied by the
  **brightness** (0.05–20, default 1). The brightness scales a studio's
  soft boxes and sky the same way.
- Pictures wider than 4096 pixels are halved (averaging 2 × 2 texels)
  until they fit, for the GPU's memory.

## Photo scene (M42b)
- The project keeps how its photos are taken (`photo` in the document,
  schema v28; absent until set): the **light** (`studio`, `soft`,
  `dramatic`, or an `environment` by name), its **turn** and
  **brightness**, the **camera** (the point it looks at in mm, its turn
  and height angle in degrees, its distance in mm, its **focal length**
  in mm on a 24 × 36 mm frame whose 24 mm side lies along the image's
  shorter side, 10–300 mm) and the image **size** (16–4096 pixels a
  side, at most 3840 × 2160 pixels in all; absent: the 3D view's).
- The command `photo.set_scene` sets it (`scene: null` forgets it),
  checking every range and that the environment is a built-in name or a
  relative `.hdr` path without `..` (the file itself is read only when
  rendering). One undo step.
- In the app, the scene is set while the 3D view shows the photo as it
  will be: a path-traced preview of the same scene, framed in the
  chosen size's shape (the rest of the view stays empty), refined up to
  64 samples per pixel. Every change starts it over: dragging in the
  view turns the camera around the model, the wheel brings it closer or
  farther, and the panel's settings apply at once. While changes come,
  it refines at half the size a side; 0.25 s after the last one, at
  full size. The drape it shows is the one when the scene opened, or
  when the light last changed.
- Rendering the photo uses the same camera and light, at the chosen
  size and quality: the result is framed as the preview was.

## UX
- **Photo** (the camera button of the 3D view) opens the photo scene: a
  panel takes the inspector's place, and the 3D view shows the preview.
  The camera bar and the simulation's controls are hidden; dragging in
  the view only turns the camera (no pulling, drawing or measuring).
- The panel: **Lighting** (Studio, Soft, Dramatic, Place; Place shows a
  list of the built-in environments and the project's `.hdr` files, and
  an **Add .hdr…** button: a file dialog, copied into `environments/`),
  **Turn** (−180° to 180°), **Brightness** (×0.25 to ×4), **Lens** (18
  to 200 mm, on a log scale), **Size**, **Quality** and the advanced
  settings, and **Render**.
- The scene opens with the project's settings and camera; without them,
  a studio through the 3D view's own lens (about 38 mm). Closing the
  panel (×) or rendering saves the scene in the project when it
  changed, and closing gives the 3D view back its own camera and lens.
- **Render** shows the photo refining in the 3D view with its progress
  bar (spec 14); a change in the panel, or **×** on the bar, puts it
  away and the preview comes back. The panel remembers the quality and
  the advanced settings for the next photo.

## Commands and tools
- MCP `render_photo` and `r3d render --view photo` take `environment`
  (a built-in name or a project path), `turn_deg` and `brightness`. With
  `environment`, `lighting` is ignored. The text result names the
  environment or the lighting, the turn, the brightness, the camera
  (an angle, or `scene`) and the size.
- When the project has a photo scene, what the arguments leave out
  comes from it: the light (unless `lighting` or `environment` is
  given), the turn, the brightness, the camera (unless `camera` or
  `frame` is given) and the size (unless a width, a height or `--size`
  is given).
- `photo.set_scene` sets the scene, as every command, over MCP, in
  scripts and with `r3d run`.
- `r3d environment FILE --project X.r3d` copies a `.hdr` into the
  project's `environments/` folder and prints its path in the project.
- Adding a file is not a document command (commands do not touch files).

## Acceptance criteria
- A `.hdr` file from Poly Haven and one written flat are read; a
  truncated, XYZ or rotated picture is refused with an error, never a
  crash.
- A photo in each built-in environment renders, deterministically (the
  same image twice).
- Under a uniform white environment, the floor under a box is darker
  than the floor far from it, and the floor far from it shows the
  environment's colour (within noise).
- The importance-sampling densities integrate to 1 over the sphere, and
  a uniform environment's density is 1 / 4π everywhere (tests).
- Turning the environment by 360° gives the same image.
- A photo whose renderer is aimed again (camera, turn, brightness) and
  resized gives the same image, bit for bit, as one rendered there
  from the start, under a studio and under an environment; a camera
  leaving the studio's backdrop asks for the scene to be built again.
- A photo scene is saved and read back with the project; out-of-range
  numbers, an unknown environment name and a path out of the project
  are refused (tests); `render_photo` without arguments takes the
  scene's light, turn, brightness, camera and size.
- In the app: Photo opens the scene with the preview refining, a
  change of lens shows at once, Render saves the scene and renders the
  photo, × on the photo goes back to the preview and × on the panel
  closes the scene (integration test).
