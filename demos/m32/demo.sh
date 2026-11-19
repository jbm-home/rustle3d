#!/usr/bin/env bash
# M32 demo: editor finishing. What M27–M29 left to agents and scripts is
# done with the mouse in the desktop app. The integration tests drive the
# real app:
# - sewing_test: a patch pocket's three edges, Ctrl-clicked in any order,
#   sewn to its U-shaped placement line; a stretch dragged along an edge
#   sewn to another edge;
# - dimension_test: an edge's length placed with the Dimension tool (T)
#   and typed where it was placed; a height pulled out beside two corners
#   and given the formula `waist / 2`;
# - handles_test: a curved edge's handle and a corner's handle dragged
#   with Select, one undo step per drag.
# Run from the repository root: ./demos/m32/demo.sh
set -euo pipefail

case "$(uname -s)" in
  Darwin) DEVICE=macos ;;
  Linux) DEVICE=linux ;;
  *) DEVICE=windows ;;
esac
cd app
for t in sewing dimension handles; do
  echo "== $t"
  flutter test "integration_test/${t}_test.dart" -d "$DEVICE" </dev/null
done
echo "M32 demo passed."
