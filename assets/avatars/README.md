# Avatars

Body data shipped with Rustle3D. Every file here must have a license that
allows commercial redistribution, and an entry below.

| File | Source | License | Changes |
|------|--------|---------|---------|
| `makehuman-body.r3dbody` | MakeHuman assets, https://github.com/makehumancommunity/makehuman, commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482`: `3dobjs/base.obj`, 38 targets (`macrodetails`, `macrodetails/height`, `measure`), the default skeleton `rigs/default.mhskel` and its weights `rigs/default_weights.mhw` | CC0 1.0 Universal (MakeHuman `LICENSE.md` §C and `LICENSE.ASSETS.md`; the skeleton and weight files also carry `"license": "CC0"`) | Body group only (helper geometry removed), quads split into triangles, decimetres → metres, feet moved to y = 0; the feminine and masculine targets are the average of MakeHuman's african/asian/caucasian "young" targets; the skeleton reduced to 24 bones (fingers, toes, face and twist bones merged into the bone they hang from), its joints moved by the targets like the skin, four skin weights per vertex; packed into our binary format (`r3d_avatar::pack`). |

Rebuild with `cargo xtask fetch-makehuman` (network). The exact list of
source files is stored in the pack's provenance text.

Only MakeHuman's **assets** are used. Its source code is AGPL and is not used:
the body model, the measurements, the fitting, the posing and skinning, and
the motions (walk, turn, raise arms) in `crates/r3d-avatar` are our own. No
motion capture data is used.

Forbidden: SMPL, SMPL-X and any body model with a non-commercial license.
