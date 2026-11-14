# 07 — Import / export

Provenance: maintainer's functional brief (2026-11-14); open specifications: Wavefront OBJ, Khronos glTF 2.0, W3C SVG 1.1, ISO 32000 (PDF). DXF-AAMA/ASTM D6673 later.

## User story
As a user, I bring in an avatar, and I take out my draped garment and my
patterns to use in other tools or to print and cut.

## Behavior
- **Import avatar**: OBJ or glTF/GLB. The user picks the units if the file
  does not say (glTF = metres). The mesh is auto-centred on the ground and
  checked (non-manifold meshes and holes produce a warning, not a refusal).
- **Export garment**: OBJ or glTF/GLB of the draped mesh (or the arranged
  state if not simulated), with normals and UVs from the 2D pattern
  (1 UV unit = 1 m of pattern), one mesh or node per panel, and optionally the
  avatar.
- **Export patterns**:
  - SVG at 1:1 scale (mm user units), with panels, internal lines, notches,
    grainlines and labels.
  - PDF at 1:1 scale, either as one large page (plotter) or **tiled** to
    A4/Letter with overlap margins, alignment marks, page grid labels and a
    100 mm test square on the first page.
  - Optional seam allowance (constant width per panel) drawn as an outer line.
- All export paths stay inside the project folder (`exports/` by default)
  unless the user chooses a location in a save dialog.

## UX
- One "Export" menu with clear presets ("Print at home (A4 tiles)", "Plotter",
  "3D (glTF)"), and a preview of the tiling before saving.

## Acceptance criteria
- Round-trip: export glTF → re-import as a mesh → same vertex count and
  positions within 1e-5 m.
- The test square measures 100 mm in the exported PDF and SVG (checked by
  parsing the output).
- Exported glTF passes the Khronos glTF validator (CI step).
