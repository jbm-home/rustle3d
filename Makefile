# Short names for the usual dev tasks, on macOS, Linux and Windows (GNU make,
# e.g. `winget install ezwinports.make`). The work is done by `cargo xtask`,
# which also runs without make. Pass Flutter options with ARGS, e.g.
#   make run ARGS=--dart-define=R3D_DEMO=t_shirt

.PHONY: run run-release build build-release check test release docs

run:            ## desktop app, debug (hot reload)
	cargo xtask run -- $(ARGS)

run-release:    ## desktop app, release (real frame rates)
	cargo xtask run --release -- $(ARGS)

build:          ## CLI + desktop app, debug
	cargo xtask build -- $(ARGS)

build-release:  ## CLI + desktop app, release
	cargo xtask build --release -- $(ARGS)

test:           ## Rust tests
	cargo test --workspace

check:          ## everything CI checks (needs bash: Git Bash on Windows)
	bash scripts/check.sh

release:        ## packaged release in dist/ (needs bash)
	bash scripts/release.sh

docs:           ## regenerate docs/commands.md
	cargo xtask gen-docs
