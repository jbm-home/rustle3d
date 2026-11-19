# Sourced by the demo scripts. demo_out NAME prints a fixed output folder
# for that demo, emptied first: demo-out/NAME at the repository root
# (git-ignored), or under $R3D_DEMO_OUT when it is set. The folder is
# also shown on stderr, so the files are easy to find after the run.
demo_out() {
  [ -n "${1:-}" ] || { echo "demo_out: missing demo name" >&2; return 1; }
  local root
  root="${R3D_DEMO_OUT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/demo-out}"
  local dir="$root/$1"
  rm -rf "$dir"
  mkdir -p "$dir"
  echo "Output folder: $dir" >&2
  echo "$dir"
}
