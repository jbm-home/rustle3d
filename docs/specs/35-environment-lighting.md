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
Haven (licence recorded in `assets/environments/README.md`).

## Status (2026-11-26, M42)

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
- The environment is a photo setting, like the studio: it is not saved
  in the project document.
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
  it should be.
- **Exposure**: the environment is scaled so that its mean luminance
  over the whole sphere is 0.8 (a lit studio), so bright outdoor
  pictures and dim rooms come out exposed alike; then multiplied by the
  **brightness** (0.05–20, default 1).
- Pictures wider than 4096 pixels are halved (averaging 2 × 2 texels)
  until they fit, for the GPU's memory.

## UX
- The Photo dialog's lighting row gets a fourth choice, **Place**.
  Choosing it shows a list of the built-in environments and the
  project's `.hdr` files, an **Add .hdr…** button (a file dialog,
  copied into `environments/`), and the **Turn** (−180° to 180°) and
  **Brightness** (×0.25 to ×4) sliders. The dialog remembers the
  choice for the next photo.
- The photo's progress bar and result are as for the studios (spec 14).

## Commands and tools
- MCP `render_photo` and `r3d render --view photo` take `environment`
  (a built-in name or a project path), `turn_deg` and `brightness`. With
  `environment`, `lighting` is ignored. The text result names the
  environment, its turn and brightness instead of the lighting.
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
