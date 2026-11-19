# Short names for the usual dev tasks, on macOS, Linux and Windows (GNU make,
# e.g. `winget install ezwinports.make`). The work is done by `cargo xtask`,
# which also runs without make. Pass Flutter options with ARGS, e.g.
#   make run ARGS=--dart-define=R3D_DEMO=t_shirt
# Each target's `## ...` comment is its line in `make help`.

.DEFAULT_GOAL := help
.PHONY: help run run-release build build-release test check release docs

# The bash scripts need Git Bash on Windows: a bare `bash` there may be WSL's.
ifeq ($(OS),Windows_NT)
BASH := "$(subst /mingw64/libexec/git-core,/bin/bash.exe,$(shell git --exec-path))"
else
BASH := bash
endif

help:           ## this help
	@cargo xtask make-help

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

check:          ## everything CI checks; run before every commit
	$(BASH) scripts/check.sh

release:        ## packaged release in dist/
	$(BASH) scripts/release.sh

docs:           ## regenerate docs/commands.md
	cargo xtask gen-docs
