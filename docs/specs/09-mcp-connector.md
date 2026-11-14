# 09 — MCP connector

Provenance: maintainer's functional brief (2026-11-14); Model Context Protocol specification (open).

## User story
As an AI agent, I can inspect, create and modify garments, run the simulation
and look at the result, either headless or live in the user's open app.

## Behavior
- **Tools generated from the command and query registry**, with the same
  names, schemas and descriptions as `docs/commands.md`.
- **Read tools**: `project_summary`, `list_panels`, `list_seams`,
  `list_materials`, `get_panel_geometry`, `get_sim_state`, `measure`.
- **Write tools**: one per command, plus `batch` (one undoable transaction),
  `undo` and `redo`.
- **Visual feedback**: `render_board` and `render_3d` return PNG images
  (size, camera preset or explicit camera, optional highlight of ids).
- **Modes**:
  - Headless: `r3d mcp [--project path]` over stdio.
  - Attached: `r3d mcp --attach` relays to the running desktop app over a
    local socket. Changes appear live in the UI and are undoable there.
- **Safety**: file paths are confined to the project folder; destructive
  commands require `confirm: true`; the app shows an indicator while an agent
  is attached, and the user can disconnect it.

## Acceptance criteria
- An end-to-end test drives a headless server through an MCP client: create a
  rectangle skirt, sew, arrange, simulate N steps, render a PNG, and get a
  non-empty image with the expected size.
- The tool list is identical to the registry (snapshot test).
- A path outside the project folder is refused with an explicit error.
