# 09 — MCP connector

Provenance: maintainer's functional brief (2026-11-14); Model Context Protocol specification (open).

## Status (2026-11-17)

Implemented: headless (M2) and attached (M9). Attached: the app's Agents
menu allows agents (off by default; nothing listens before); the app
then listens on a loopback TCP port and writes the port and a secret
token to `agent.json` in its support folder. `r3d mcp --attach
[--endpoint file]` reads it, sends `R3D-ATTACH <token>` and relays stdio;
a wrong token is refused. One agent at a time; its changes are saved,
go into the app's undo history and show at once (pattern, inspector,
live drape). The app shows "Agent connected" and can disconnect it; it
asks for the status and the agent's changes on a timer, as the views do
not redraw while an agent's long action holds the project, and never
waits for the project to ask. `avatar_get` gives the body's waist, hips
and crotch heights, to place hand-drafted skirts and trousers.
`render_3d` and `render_photo` in attached mode still draw the cached or
starting drape, not the app's live one; a long action (e.g. `simulate`) holds the project
while it runs.

## User story
As an AI agent, I can inspect, create and modify garments, run the simulation
and look at the result, either headless or live in the user's open app.

## Behavior
- **Tools generated from the command and query registry**, with the same
  names, schemas and descriptions as `docs/commands.md`.
- **Read tools**: `project_summary`, `list_panels`, `list_seams`,
  `list_materials`, `get_panel_geometry`, `get_sim_state`, `measure`.
- **Write tools**: one per command, plus `batch` (one undoable transaction,
  with an optional `label` for the history), `history_undo`,
  `history_redo`, `history_list` and `history_go_to`.
- **Long tools** (M26): when the client sends a progress token, a graded
  template or a drape sends `notifications/progress` (parts done, total,
  and a message: the task code and the size). Cancelling the request
  (`notifications/cancelled`) stops the step; nothing is changed. Tool
  calls run off the server's async threads.
- **Visual feedback**: `render_board` and `render_3d` return PNG images
  (`width`, `height`; implemented in M4). `render_3d` shows the last
  `simulate` result if the document has not changed since, otherwise the
  starting positions, and says which. `render_photo` (M14, spec 14)
  path-traces the same garment in a photo studio: `quality`
  (quick/good/best), `lighting` (studio/soft/dramatic), `camera`
  (front/three_quarter/side/back), `frame` (garment/body), and advanced
  `samples`, `bounces`, `denoise`; the text result gives the time taken.
  Highlighting of ids comes later.
- **Fit** (M24, spec 24): `render_3d` takes `fit_map` (`strain` or
  `gap`) to colour the cloth by how tight it is, a scale bar at the right
  edge; the text result gives the scale and the tightest value with its
  piece. `fit_report` and `fit_tape` are the `fit.report` and `fit.tape`
  actions: strain per piece, tight spots, ease at the chest, waist and
  hips; a tape at a height or a point.
- **Prints** (M21, spec 25): `garment_set_print`, `garment_remove_print`
  and `panel_set_print_placement` are the commands; an image must
  already be in the project folder (the server reads and writes nothing
  outside it), e.g. `textures/name.jpg`. `render_board`, `render_3d`,
  `render_photo` and the glTF/OBJ exports show the print.
- **Imported avatars** (M31, spec 27): `avatar_import` reads an OBJ,
  glTF or GLB body already in the project folder (e.g.
  `avatars/scan.glb`) and drapes on it; `avatar_check` reports what to
  fix before draping; `avatar_orient`, `avatar_set_point`,
  `avatar_guess_points` and `avatar_remove_mesh` correct it. `simulate`
  adds `body_warnings` and refuses a `motion` on an imported body.
- **Trims** (M20, spec 26): `button_add`, `button_set`,
  `button_set_closed`, `button_remove`, `zipper_add`, `zipper_set_open`,
  `zipper_remove` and `trim_list`; `simulate` reports `closures` and
  `max_closure_gap_mm`; `render_board` and `render_3d` draw buttons and
  zippers. An agent attached to the app opens or closes them while the
  drape runs.
- **Tech pack** (M30, spec 23): `export_techpack` is the
  `export.techpack` action with one more parameter, `views` (`3d`
  default, `photo`, `none`): the server renders the garment's front and
  back first and puts them at the top of the PDF.
- **Naming**: tool names are the operation names with underscores
  (`panel_create_rect`). Errors are tool results with `isError` and the
  structured error (`code`, `args`, `text`).
- **Modes**:
  - Headless (M2): `r3d mcp --project <dir>` over stdio; creates the
    project if missing and saves after every successful change.
  - Attached: `r3d mcp --attach` relays to the running desktop app over a
    local socket. Changes appear live in the UI and are undoable there.
- **Safety**: file paths, read (`import.pattern`) or written (exports), are
  confined to the project folder; destructive
  commands require `confirm: true`; the app shows an indicator while an agent
  is attached, and the user can disconnect it.

## Acceptance criteria
- An end-to-end test drives a headless server through an MCP client: create a
  rectangle skirt, sew, arrange, simulate N steps, render a PNG, and get a
  non-empty image with the expected size.
- The tool list is identical to the registry (snapshot test).
- A path outside the project folder is refused with an explicit error.
