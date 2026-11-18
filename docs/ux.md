# UX and visual design principles

Status: living document (2026-11-14), revisable as we learn from users.

Two product goals have the same weight as the features:

1. **The UI is beautiful.** It should feel like a modern creative app, calm and
   polished, not like an engineering tool.
2. **The learning curve is very short.** Someone who has never used a 3D
   garment tool can drape a simple skirt or T-shirt in under 10 minutes, and
   can be productive on their own garments within an hour.

Every feature spec has a "UX" section that applies these principles.

## 1. Learnability

- **Start from something, never from nothing.** The start screen offers
  "New garment" with visual templates (skirt, T-shirt, dress, trousers, tote
  bag) and "Blank project". A template opens already sewn and arranged, so
  the user presses *Simulate* and sees cloth move in the first minute.
- **Guided flow, not a wizard prison.** The "new garment" flow is a short
  step bar (*Measurements → Pattern → Sewing → Fabric → Drape*). It highlights
  the next useful action, but every step can be skipped and every tool stays
  reachable.
- **Progressive disclosure.** The default inspector shows only the essentials
  (e.g. fabric: preset and color). An "Advanced" fold shows the physical
  parameters. Expert tools sit behind a clearly labelled "More tools" area.
- **Plain words.** "Sew these edges", "Fabric", "Stiffness". Technical terms
  (XPBD, compliance) never appear in the UI. Garment-making terms (notch,
  grainline, seam allowance) appear with a one-line hint the first time.
- **Inline help everywhere.** Every tool has a tooltip with its name, a
  one-sentence explanation, its shortcut and a tiny animated or illustrated
  example. Empty states explain what to do ("Draw a rectangle with R, or start
  from a template").
- **Show, don't warn.** Seam length mismatch, edge direction and penetration
  are shown visually in context (colors, arrows, badges), with a one-click
  fix when one exists ("Flip direction").
- **Safe to explore.** Every action can be undone, history is visible, nothing
  destructive happens without a confirmation, and "Reset simulation" never
  loses pattern work.
- **Built-in onboarding.** An optional 3-minute interactive tour on first
  launch, and a "Learn" panel with short task cards ("Add a dart", "Sew a
  sleeve"). Each card is a scripted command sequence that can play as a demo,
  reusing the command layer.

- **Edit where you look.** Pattern edits update the 3D drape live, and
  adjustments made on the garment in 3D update the pattern. Both views are
  always in sync; there is no "3D mode" to enter.

## 2. Efficiency (fast progression to expert speed)

- Every tool has a one-key shortcut, shown in tooltips and in a cheat sheet
  (`?`).
- A **command palette** (`Ctrl/Cmd+K`) searches every command by name, using
  the same registry as the CLI and MCP.
- Few modes: one selection tool does most of the work (click a point or edge,
  drag, double-click a curve to add a point), and other tools return to
  selection after use unless they are locked.
- Direct manipulation over dialogs: lengths are typed inline while dragging
  (`Tab` to type an exact value), and properties are edited in place.
- Synchronized 2D and 3D: hovering or selecting in one view highlights the
  same element in the other.
- **Never frozen** (M26): a step that can take seconds runs off the UI
  thread. Past 300 ms a modal card says what is being done, with a bar
  when the length is known (the size being drafted) and an
  indeterminate one otherwise, and **Stop** where stopping changes
  nothing. Work the user did not ask for (fitting the body the 3D view
  needs) shows as a small note in that view, never a modal card.
- Undo is cheap to reach: the arrows name what they will undo, the
  history lists every step, and undoing selects what changed.

## 3. Visual design

- **Own identity.** Our own palette, icons, layout and names. Nothing is
  modelled on existing garment tools (see the clean-room rules).
- **Design tokens.** Colors, spacing scale (4 px base), radii, elevation,
  typography and motion durations are defined once in `ui_kit::tokens`. No
  literal colors in screen code.
- **Light and dark themes** from the start, both first-class.
- **Typography**: an embedded open-license UI font (e.g. Inter, OFL) at
  consistent sizes, with tabular numerals for measurements.
- **Icons**: our own SVG icon set, drawn on a consistent grid. Each icon's
  provenance is recorded if it comes from an open set (e.g. Lucide, ISC
  license).
- **Calm canvas.** Neutral backgrounds, with color reserved for meaning:
  selection, seams (one hue per seam pair), warnings, fabric colors.
- **Motion.** Short (120–200 ms) eased transitions for panels, hovers and
  selection, never blocking input.
- **Beautiful 3D by default.** Soft studio lighting, a ground shadow, good
  fabric shading and anti-aliasing. The default render is good enough for a
  screenshot.

## 4. Layout (initial proposal, to be validated in M3)

```
┌─────────────────────────────────────────────────────────────────┐
│  Project name   ·  step bar (Pattern › Sew › Fabric › Drape)  ⌘K│
├──┬───────────────────────────────┬───────────────────────┬──────┤
│T │                               │                       │ Insp-│
│o │        2D pattern board       │       3D view         │ ector│
│o │                               │                       │      │
│l │                               │   ▶ Simulate  ⟲ Reset │      │
│s │                               │                       │      │
├──┴───────────────────────────────┴───────────────────────┴──────┤
│  hints · measurements · status (sim fps, agent connected)       │
└─────────────────────────────────────────────────────────────────┘
```

The split between the two views is adjustable, and either view can be
maximized (`Tab` toggles).

## 5. How we validate

- **M3 visual gate**: before building most tools, render the main screens
  (start screen, editor with a template, inspector, dark and light) and get
  the maintainer's approval.
- **10-minute test** before v0.1 (end of M10): a first-time user makes a
  skirt and a T-shirt from the templates. We record where they get stuck
  and fix it before v0.1. Protocol and note sheet: [`user-test.md`](user-test.md).
- Offscreen screenshots of key screens are kept as review artifacts in CI, so
  visual regressions are visible in PRs.
