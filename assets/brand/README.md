# Brand

The Rustle3D mark: a piece of cloth hanging in folds, made of two panels
(lilac and white) sewn down the middle with a running stitch. It says
draping (cloth under gravity), pattern pieces and sewing.

Provenance: our own design (2026), drawn by hand in SVG for this project.
Same license as the code (MIT). The wordmark is set in Manrope ExtraBold
(SIL OFL 1.1, see `THIRD_PARTY_NOTICES.md`), converted to outlines.

## Files

| File | Use |
|------|-----|
| `mark.svg` | The mark on its rounded tile, for 48 px and up. |
| `mark-small.svg` | Simplified for 32 px and down: no folds, three bold stitches, cloth slightly larger. The app's in-window logo and the site's header and favicon use it. |
| `app-icon-macos.svg`, `app-icon-macos-small.svg` | The same two on Apple's icon grid (824 px tile on a 1024 px canvas, with a shadow). |
| `logo.svg`, `logo-dark.svg` | Mark and wordmark side by side, for light and dark backgrounds. |
| `social-preview.svg`, `.png` | GitHub social preview, 1280 × 640 (repository Settings > Social preview). |

## Colours

| Role | Colour |
|------|--------|
| Tile | `#7B5CFF` → `#5634E6` (top to bottom); flat `#6D4AFF` where a gradient is awkward |
| Right panel | `#FFFFFF` |
| Left panel | `#D9D0FF` |
| Folds | `#6D4AFF` at 24 % |
| Stitch | `#D9409A` |
| Wordmark | `#14161B` (light), `#ECEEF3` (dark) |

## Regenerate the icons

After changing a master SVG:

```sh
./scripts/brand-icons.sh ../rustle3dweb   # the site's folder is optional
```

It writes the macOS app icons, the Windows `.ico`, the Linux window icon,
the social preview PNG and, with the site's folder, its favicon, Apple touch
icon and manifest icons. The app's `Logo` widget
(`app/lib/src/start_screen.dart`) and the site's `LogoMark` draw
`mark-small.svg` in code: keep them in step by hand.
