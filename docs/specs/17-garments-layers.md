# 17 — Several garments and layers

Provenance: maintainer's functional brief (ROADMAP M19, 2026-11-17); general garment-making knowledge (garments worn in layers, a shirt tucked in or worn out); self-collision limits from `docs/physics.md` §5.

## Status (2026-11-17)

Implemented: several garments per project (`garment.add`, `garment.rename`,
`garment.remove`, `garment.set_layer`, `panel.set_garment`), each with its
own fabric and colour, used by the drape, the 3D view, photos and the 3D
exports (one material per garment); a template applied to a project with
pieces becomes a garment of its own, worn over the others; the garments
list in the app's inspector. Project files: schema v9 (`layer`).

Not yet: a garment worn closely over another, such as a shirt tucked into
fitted trousers. Cloth layers repel each other particle to particle, two
particle radii apart (about 2 cm at the default mesh spacing), so close
layers either start 2 cm apart and pull in through each other, or
interpenetrate where they touch (docs/physics.md, "Several garments").
This needs thin cloth–cloth contacts (vertex–triangle) in both solvers.

## User story
As a designer, I dress the avatar in a shirt and trousers, each in its own
fabric and colour, and choose whether the shirt is worn over the trousers
or tucked in.

## Behavior
- A project holds one or more garments; every piece belongs to one. A
  garment has a name, a fabric, an optional colour and a layer.
- Layers are worn from the body out: layer 0 is next to the body; a
  garment drapes over the ones on lower layers. Changing a garment's
  layer moves the others to make room.
- Adding a template to a project that already has pieces adds a garment
  named after the template, worn over the others; the first template in
  an empty project names the project's garment.
- Moving a piece to another garment moves its linked mirror copies with
  it. Deleting a garment deletes its pieces; a project keeps at least one.
- The simulation gives each garment its fabric. Each garment starts
  outside the ones under it (a layer's gap further out), and with several
  garments the pieces start just outside the body near their own height,
  so seams do not close across a gap through another garment.

## UX
- With nothing selected, the inspector lists the garments from the body
  out, each with its fabric swatches, arrows to wear it under or over its
  neighbour, and a delete button; "Add a garment" below.
- A piece's inspector has a Garment menu when the project has several.

## Acceptance criteria
- Every new command has an apply → undo → redo test; old project files
  load (migration v8 → v9).
- Trousers with a shirt worn over them drape with closed seams, each in
  its fabric, the shirt starting outside the trousers.
