#!/usr/bin/env bash
# M17 demo: the cloth solver on the GPU. The reference scenes and every
# starter garment drape on both backends with the same checks, then the
# T-shirt and the dress are timed on each. In the app the drape runs on
# the GPU: the status bar reads "GPU step … ms" (R3D_SOLVER=cpu forces the
# CPU, "CPU step … ms"). Needs a GPU; run from the repository root:
# ./demos/m17/demo.sh
set -euo pipefail

echo "== 1. Reference scenes and starter garments on the GPU (and the CPU)"
cargo test --quiet --release -p r3d-sim-gpu 2>&1 | grep -E "^test result"

echo "== 2. Time per frame (target: T-shirt 16.7 ms, dress 33 ms)"
# Criterion prints the name and the time on one line or two; keep the
# middle estimate.
cargo bench --quiet -p r3d-sim-gpu 2>&1 | awk '
  /^frame\// { name = $0; sub(/ +time:.*/, "", name); sub(/^frame\//, "", name) }
  /time:/ { match($0, /\[[^]]*\]/); split(substr($0, RSTART + 1, RLENGTH - 2), t, " ");
            printf "  %-20s %s %s\n", name, t[3], t[4] }'

echo "== 3. In the app: the T-shirt demo drapes on the GPU"
echo "   (cd app && flutter run --release --dart-define=R3D_DEMO=t_shirt --dart-define=R3D_SIMULATE=true)"
