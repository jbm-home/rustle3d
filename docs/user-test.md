# The 10-minute test (before v0.1)

Goal (`docs/ux.md` §5, ROADMAP M10): a first-time user makes a skirt and a
T-shirt with the app, unaided, in under 10 minutes each. We note where they
hesitate or get stuck, fix that, and test again. At least three people who
have never used a 3D garment tool.

## Setup

- A release build (`./scripts/release.sh`) on a clean account, or remove
  the app's data first so the start screen and the first-launch tour
  appear as for a new user (macOS:
  `~/Library/Containers/dev.rustle3d.rustle3d/Data/`; Linux:
  `~/.local/share/dev.rustle3d.rustle3d/` and `~/Documents/Rustle3D/`).
- Screen recording on; a stopwatch; this sheet.
- Say only: "Think aloud. I can't help you; if you are really stuck for
  more than a minute, say so and we move on." Do not point at anything.

## Tasks

1. **A skirt for yourself.** "Make an A-line skirt for a body close to
   yours, and look at it on the body." Done when it has draped.
2. **Change it.** "Make it 5 cm shorter, then try it in denim."
   Done when the shorter denim skirt has draped.
3. **A T-shirt.** "Make a T-shirt for a man's L body with longer sleeves,
   and look at it from the back." Done when it has draped, seen from the
   back.
4. **Optional, harder.** "Draw a line across the front where a pocket
   could go."

## What to note, per task

| | Time | Stuck? (where, how long) | Wrong turns | Quotes |
|---|---|---|---|---|
| 1 | | | | |
| 2 | | | | |
| 3 | | | | |
| 4 | | | | |

Watch in particular: whether they find the templates and the sizes sheet;
whether the step bar and the guide banner are read; whether they find
Simulate; how they try to shorten (pattern points, ⇧-drag in 3D, Learn);
whether the fabric swatches are found with nothing selected; whether the
camera views (1–4 or the Front/Side/Back buttons) are found; anything they
say is confusing or ugly.

## After

- Pass: tasks 1–3 each under 10 minutes for every tester, with no stop
  longer than a minute.
- File each stuck point as a fix (UI text, layout, a Learn card, a
  default), fix them, and test again with someone new.
- Then bump the workspace version to 0.1.0 and build the release
  packages.
