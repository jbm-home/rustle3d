# 32 — Pattern library

Provenance: maintainer's functional brief (ROADMAP M38, 2026-11-25: a
library of patterns to add to the project one is working on — T-shirt,
shirt, hoodie, trousers, jeans, joggers, shorts, skirt, dress — where one
can also save one's own work to reuse it in new projects; a library
panel by kind with thumbnails; jeans with five pockets, a yoke, a fly
and a waistband with belt loops; shorts; "my patterns" saved with a
name, a kind and a thumbnail, drafted again for the new body, renamed,
deleted, exported and imported as a file; the same through commands;
the storage the prints' and fabrics' libraries will use); general
garment-making knowledge (five-pocket jeans: scooped front pockets over
facings, a coin pocket on the right facing, patch pockets on the back,
a back yoke, a fly, a waistband buttoned over the fly, belt loops).

## Status

Done (M38, 2026-11-25).

- The command layer: `pattern.add` adds the garments of a saved
  pattern (a project document) to a project, drafted again for its
  body; `library.list`, `library.add`, `library.save`,
  `library.rename`, `library.delete`, `library.export`,
  `library.import` on the user's library folder (`r3d --library`, by
  default the app's); `template.list` and `library.list` give each
  built-in pattern's kind. `panel.copy` and `panel.paste` carry
  construction details, buttons, zippers, constraints on notches and
  construction lines, and dimension names since.
- Jeans and shorts (spec 11).
- The app: **From the library…** under the garments opens the library
  (a dialog: My patterns, then the built-in patterns by kind, with
  their thumbnails); the bookmark button on a garment's card saves it;
  rename, export and delete on a saved pattern, Import… in the
  dialog's header.
- Not done: the jeans' waistband is not buttoned (its two halves are
  sewn at the centre front over the fly): a lap buttoned over the
  other end did not close round the body in the drape.

## User story

As a designer, I add a jean and a short from the library to the project
I am working on, and I save a skirt I made in one project to add it to
another one, drafted for that project's body.

## Behavior

- **Kinds**: tops, bottoms, dresses and skirts, accessories. Built-in
  patterns are the templates (spec 11), each with its kind: T-shirt,
  shirt, hoodie (tops); trousers, jeans, joggers, shorts (bottoms);
  dress, straight and A-line skirts (dresses and skirts); tote bag
  (accessories); M39 (spec 33): sweatshirt (tops), bodysuit, pyjama,
  pyjama top, pyjama bottoms, jogging suit (babies and children).
- **Adding** a pattern makes a new garment in the open project, worn
  over the garments there (on an empty board, the project's own
  garment becomes it), drafted for the avatar and every size of the
  range. A built-in pattern is drafted from scratch (`template.apply`).
  A saved one comes with everything it was saved with
  (`pattern.add`): pieces, seams, notches, internal lines, pins,
  construction details, buttons, zippers, constraints (a dimension
  keeps its name unless the project has one by that name), variables
  (a variable the project has keeps the project's formula), fabric,
  colour, print (its image copied into the project's `textures/`) and
  notes; then:
  - pieces drafted from a template are drafted again for the project's
    body: each point and curve handle moves as the template moves it
    between the two bodies, so what was done to the pieces since is
    kept; the body measurements the template was given (chest, waist,
    hips) change as the body does, lengths and ease stay; a piece whose
    outline gained or lost points stays as drawn;
  - pieces imported from Seamly2D/Valentina are drafted again the same
    way (`garment.redraft`, spec 28);
  - dimensions given by formulas are solved for the new body;
  - the other pieces come as drawn;
  - in a size range, the garment is graded (templates and Seamly2D
    patterns drafted in every size; drawn pieces keep their grade rules
    when the range has as many sizes and the same base).
  A project without an avatar takes every piece as drawn.
- **Saving** (`library.save`) keeps one garment of the project under a
  name and a kind: the garment alone, with the body it is drafted for,
  the size range and the variables. A name already in the library
  (case aside) is refused unless replaced. Saving never changes the
  project.
- **Managing**: rename, delete (projects the pattern was added to keep
  their garments), export as a zip file to share it, import one (a
  name already taken gets a number).
- **Storage**: the user's library folder (the app's support folder,
  shared with the body presets and motions), `patterns/<Name>.r3d/`: a
  project folder holding the garment alone (`project.json`, the print
  image in `textures/`), and `library.json` with its kind. Being a
  project, it migrates with the schema, opens, renders its thumbnail
  and packs like any project. The prints' and fabrics' libraries will
  sit beside it the same way.
- **Agents** (CLI, MCP): `library.list` lists the built-in patterns and
  the user's own; `template.apply` adds a built-in one, `library.add`
  a saved one; `library.save`, `library.rename`, `library.delete`
  (with `confirm`), `library.export` and `library.import` (zip files
  inside the project folder). `r3d --library <folder>` gives the
  library folder; by default the desktop app's. Without one, library
  steps fail with `library.none`.

## UX

- **From the library…**, below the garments in the inspector (nothing
  selected), opens the library: My patterns, then the built-in patterns
  by kind, each a card with its thumbnail (the pattern drawn by our
  renderer). One click adds a pattern and closes the library; the drape
  starts again from around the body.
- On a garment's card, the bookmark button (Save to library) asks for a
  name and a kind; a name already in the library asks before replacing.
- On a saved pattern's card, **⋯**: rename, export, delete;
  **Import…** in the library's header adds a shared pattern file.

## Acceptance criteria

- In an open project, a jean and a short are added from the library.
- A skirt saved from one project is added to another, drafted for its
  body: its pieces match the template drafted for that body, the edits
  made to them are kept.
- Every library step is reachable from the CLI and MCP.
