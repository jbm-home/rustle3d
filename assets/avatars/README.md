# Avatars

Body data shipped with Rustle3D. Every file here must have a license that
allows commercial redistribution, and an entry below.

| File | Source | License | Changes |
|------|--------|---------|---------|
| `makehuman-body.r3dbody` | MakeHuman assets, https://github.com/makehumancommunity/makehuman, commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482`: `3dobjs/base.obj` and 38 targets (`macrodetails`, `macrodetails/height`, `measure`) | CC0 1.0 Universal (MakeHuman `LICENSE.ASSETS.md`) | Body group only (helper geometry removed), quads split into triangles, decimetres → metres, feet moved to y = 0; the feminine and masculine targets are the average of MakeHuman's african/asian/caucasian "young" targets; packed into our binary format (`r3d_avatar::pack`). |

Rebuild with `cargo xtask fetch-makehuman` (network). The exact list of
source files is stored in the pack's provenance text.

Only MakeHuman's **assets** are used. Its source code is AGPL and is not used:
the body model, the measurements and the fitting in `crates/r3d-avatar` are our
own.

Forbidden: SMPL, SMPL-X and any body model with a non-commercial license.
