# Makefile for xenacool.github.io
# Build mdBook docs, serve locally, and publish to gh-pages

# Submodule paths
DRAGGING_SUBMODULE := libs/bevy_composable_dragging
GRADMESH_SUBMODULE := libs/bevy_gradmesh

# Output directories (relative to repo root)
DRAGGING_OUTPUT := bevy_composable_dragging
GRADMESH_OUTPUT := bevy_gradmesh

# WASM book output directories (live interactive examples)
DRAGGING_WASM_OUTPUT := bevy_composable_dragging/wasm
GRADMESH_WASM_OUTPUT := bevy_gradmesh/wasm

# Gallery showcases: one WASM binary each, built from demos/showcase_runner
SHOWCASES := kart_racer_editor cozy_castle physics_lab
SHOWCASE_RELEASE_DIR := demos/showcase_runner/target/wasm32-unknown-unknown/release
SYNTH_SUBMODULE := libs/composable_dragging_synth_demo
SYNTH_RELEASE_DIR := $(SYNTH_SUBMODULE)/target/wasm32-unknown-unknown/release

# Default port for local server
PORT ?= 8000

.PHONY: all build-dragging build-gradmesh build build-wasm build-wasm-dragging build-wasm-gradmesh build-examples build-examples-dragging build-examples-gradmesh build-wasm-runner build-gallery build-gallery-showcases build-gallery-synth serve clean clean-examples publish push-wasm-gh-pages check-links check-links-local check-versions check-main-pages test-js help

all: build

## Build both mdBook docs (plain text, no images)
build: build-dragging build-gradmesh

## Build bevy_composable_dragging book (plain text, with the variant switch)
build-dragging:
	@echo "Building bevy_composable_dragging book..."
	scripts/build_book.sh $(DRAGGING_SUBMODULE) $(DRAGGING_OUTPUT) plain

## Build bevy_gradmesh book (plain text, with the variant switch)
build-gradmesh:
	@echo "Building bevy_gradmesh book..."
	scripts/build_book.sh $(GRADMESH_SUBMODULE) $(GRADMESH_OUTPUT) plain

## Build the shared WASM runtime and both interactive books
build-wasm: build-wasm-runner build-wasm-dragging build-wasm-gradmesh

## Build the interactive bevy_composable_dragging book (rebuilds the plain book first)
build-wasm-dragging: build-dragging
	@echo "Building interactive bevy_composable_dragging book..."
	scripts/build_book.sh $(DRAGGING_SUBMODULE) $(DRAGGING_OUTPUT) interactive

## Build the interactive bevy_gradmesh book (rebuilds the plain book first)
build-wasm-gradmesh: build-gradmesh
	@echo "Building interactive bevy_gradmesh book..."
	scripts/build_book.sh $(GRADMESH_SUBMODULE) $(GRADMESH_OUTPUT) interactive

## Build the shared WASM runtime (single binary with all examples)
build-wasm-runner:
	@echo "Building shared WASM runtime (build.rs auto-generates examples)..."
	cd demos/wasm_runner && cargo build --target wasm32-unknown-unknown --release
	@mkdir -p demos/wasm
	wasm-bindgen demos/wasm_runner/target/wasm32-unknown-unknown/release/wasm_runner.wasm --out-dir demos/wasm --target web
	wasm-opt -Oz --converge --enable-bulk-memory --enable-nontrapping-float-to-int -o demos/wasm/wasm_runner_bg.wasm demos/wasm/wasm_runner_bg.wasm
	cp demos/wasm_runner/demo-frame.html demos/wasm/
	@# Copy the auto-generated examples.json manifest for the gallery
	@if [ -f demos/wasm/examples.json ]; then \
		echo "Gallery manifest: demos/wasm/examples.json"; \
	fi
	@size=$$(wc -c < demos/wasm/wasm_runner_bg.wasm); \
	echo "Shared WASM runtime: $$size bytes ($$(( size / 1024 / 1024 )) MB)"
	@echo "Done: demos/wasm/"

## Build the gallery: one binary per showcase, the synth demo when it has code, and the manifest
build-gallery: build-gallery-showcases build-gallery-synth
	@mkdir -p demos/gallery
	cp demos/showcase_runner/frame.html demos/gallery/frame.html
	python3 scripts/gen_gallery_manifest.py

## Build the bevy_composable_dragging showcases, each as its own binary
build-gallery-showcases:
	@echo "Building gallery showcases: $(SHOWCASES)"
	cd demos/showcase_runner && cargo build --target wasm32-unknown-unknown --release --bins
	@for showcase in $(SHOWCASES); do \
		scripts/package_showcase.sh $(SHOWCASE_RELEASE_DIR)/$$showcase.wasm || exit 1; \
	done

## Build the synth demo once its submodule has a Cargo.toml; its binary must be named after the repo
build-gallery-synth:
	@if [ -f $(SYNTH_SUBMODULE)/Cargo.toml ]; then \
		echo "Building composable_dragging_synth_demo..."; \
		(cd $(SYNTH_SUBMODULE) && cargo build --target wasm32-unknown-unknown --release) && \
		scripts/package_showcase.sh $(SYNTH_RELEASE_DIR)/composable_dragging_synth_demo.wasm; \
	else \
		echo "Skipping composable_dragging_synth_demo: $(SYNTH_SUBMODULE) has no Cargo.toml yet"; \
	fi

## Build all examples to WASM (deprecated; use build-wasm-runner instead)
build-examples: build-wasm-runner

## Build bevy_composable_dragging examples to WASM
build-examples-dragging:
	@echo "Building bevy_composable_dragging examples to WASM..."
	@mkdir -p demos/examples/bevy_composable_dragging
	# Examples requiring pbr feature
	@for example in hello toggle three_way reorder_list ring_of_beads slider clock scheduler lists_in_lists tessellations chain_of_cells blend_square; do \
		echo "  Building $$example (pbr)..."; \
		mkdir -p demos/examples/bevy_composable_dragging/$$example; \
		cd $(DRAGGING_SUBMODULE) && \
		cargo build --target wasm32-unknown-unknown --release --example $$example --features pbr 2>&1 && \
		cd ../.. && \
		wasm-bindgen $(DRAGGING_SUBMODULE)/target/wasm32-unknown-unknown/release/examples/$$example.wasm --out-dir demos/examples/bevy_composable_dragging/$$example --target web && \
		wasm-opt -Oz --converge --enable-bulk-memory --enable-nontrapping-float-to-int -o demos/examples/bevy_composable_dragging/$$example/$${example}_bg.wasm demos/examples/bevy_composable_dragging/$$example/$${example}_bg.wasm && \
		echo "  Done: $$example"; \
	done
	# Examples requiring shapes-2d feature
	@for example in tactile_tessellations gradient_kanban_2d; do \
		echo "  Building $$example (shapes-2d)..."; \
		mkdir -p demos/examples/bevy_composable_dragging/$$example; \
		cd $(DRAGGING_SUBMODULE) && \
		cargo build --target wasm32-unknown-unknown --release --example $$example --features shapes-2d 2>&1 && \
		cd ../.. && \
		wasm-bindgen $(DRAGGING_SUBMODULE)/target/wasm32-unknown-unknown/release/examples/$$example.wasm --out-dir demos/examples/bevy_composable_dragging/$$example --target web && \
		wasm-opt -Oz --converge --enable-bulk-memory --enable-nontrapping-float-to-int -o demos/examples/bevy_composable_dragging/$$example/$${example}_bg.wasm demos/examples/bevy_composable_dragging/$$example/$${example}_bg.wasm && \
		echo "  Done: $$example"; \
	done
	# Examples requiring shapes-3d feature
	@for example in gradient_hanoi_3d gizmo_3d cozy_castle_3d kart_racer_editor physics_lab; do \
		echo "  Building $$example (shapes-3d)..."; \
		mkdir -p demos/examples/bevy_composable_dragging/$$example; \
		cd $(DRAGGING_SUBMODULE) && \
		cargo build --target wasm32-unknown-unknown --release --example $$example --features shapes-3d 2>&1 && \
		cd ../.. && \
		wasm-bindgen $(DRAGGING_SUBMODULE)/target/wasm32-unknown-unknown/release/examples/$$example.wasm --out-dir demos/examples/bevy_composable_dragging/$$example --target web && \
		wasm-opt -Oz --converge --enable-bulk-memory --enable-nontrapping-float-to-int -o demos/examples/bevy_composable_dragging/$$example/$${example}_bg.wasm demos/examples/bevy_composable_dragging/$$example/$${example}_bg.wasm && \
		echo "  Done: $$example"; \
	done
	@echo "Done: demos/examples/bevy_composable_dragging/"

## Build bevy_gradmesh examples to WASM
build-examples-gradmesh:
	@echo "Building bevy_gradmesh examples to WASM..."
	@mkdir -p demos/examples/bevy_gradmesh
	@for example in shapes styles mix_spaces pbr_effects gradient_2d sdf_2d sdf_dash_gallery immediate retained instanced standalone curves polyline sweep road composed_shapes terrain deformation appearance arbitrary_shapes; do \
		echo "  Building $$example..."; \
		mkdir -p demos/examples/bevy_gradmesh/$$example; \
		cd $(GRADMESH_SUBMODULE) && \
		cargo build --target wasm32-unknown-unknown --release --example $$example 2>&1 && \
		cd ../.. && \
		wasm-bindgen $(GRADMESH_SUBMODULE)/target/wasm32-unknown-unknown/release/examples/$$example.wasm --out-dir demos/examples/bevy_gradmesh/$$example --target web && \
		wasm-opt -Oz --converge --enable-bulk-memory --enable-nontrapping-float-to-int -o demos/examples/bevy_gradmesh/$$example/$${example}_bg.wasm demos/examples/bevy_gradmesh/$$example/$${example}_bg.wasm && \
		echo "  Done: $$example"; \
	done
	@echo "Done: demos/examples/bevy_gradmesh/"

## Serve the site locally using Python's http.server
serve:
	@echo "Starting local server on http://localhost:$(PORT)"
	@open http://localhost:$(PORT)
	python3 -m http.server $(PORT)

## Publish: rebuild both books and WASM runtime, commit, and push to gh-pages
publish: check-versions build build-wasm build-gallery
	@echo "Publishing to gh-pages..."
	git add $(DRAGGING_OUTPUT) $(GRADMESH_OUTPUT) $(DRAGGING_WASM_OUTPUT) $(GRADMESH_WASM_OUTPUT) demos/wasm/ demos/gallery/
	git commit -m "Rebuild books: $(shell date +%Y-%m-%d)" || echo "No changes to commit"
	git push origin gh-pages
	@echo "Done."

## DANGER: Push WASM examples to gh-pages only; NEVER run automatically
## This target is intentionally excluded from all other targets.
## Usage: make push-wasm-gh-pages (only after confirming you are on gh-pages branch)
push-wasm-gh-pages:
	@if git branch --show-current | grep -q '^gh-pages$$'; then \
		echo "Pushing WASM files to gh-pages..."; \
		git add demos/wasm/ demos/examples/ demos/gallery/; \
		git commit -m "Update WASM runtime: $(shell date +%Y-%m-%d)" || echo "No WASM changes to commit"; \
		git push origin gh-pages; \
		echo "Done."; \
	else \
		echo "ERROR: You are on '$(shell git branch --show-current)' branch."; \
		echo "WASM files must ONLY be committed to gh-pages branch."; \
		echo "Switch with: git checkout gh-pages"; \
		exit 1; \
	fi

## Pull submodules and check all links against live GitHub Pages site
check-links:
	@echo "Pulling submodules..."
	git submodule update --init --recursive
	@echo "Building books..."
	$(MAKE) build
	@echo "Checking links against live site..."
	python3 scripts/check_links.py

## Pull submodules and check all links against local server
check-links-local:
	@echo "Pulling submodules..."
	git submodule update --init --recursive
	@echo "Building books..."
	$(MAKE) build
	@echo "Checking links against local server on port $(PORT)..."
	@echo "Note: Run 'make serve' in another terminal first, then this target."
	python3 scripts/check_links.py --local --port $(PORT)

## Check that book version references match Cargo.toml versions
check-versions:
	@echo "Checking book versions..."
	python3 scripts/check_versions.py

## Check that main and index pages never mention WASM
check-main-pages:
	python3 scripts/check_main_pages.py

## Run the JavaScript unit tests (Node built-in runner)
test-js:
	node --test 'js/*.test.js' 'js/book_demos/*.test.js' 'js/gallery/*.test.js'

## Clean all built book output and WASM runtime
clean:
	@echo "Cleaning built book output..."
	rm -rf $(DRAGGING_OUTPUT)
	rm -rf $(GRADMESH_OUTPUT)
	rm -rf demos/wasm/
	rm -rf demos/gallery/
	@echo "Done."

## Clean all WASM example output
clean-examples:
	@echo "Cleaning WASM example output..."
	rm -rf demos/examples/
	@echo "Done."

## Show help
help:
	@echo "xenacool.github.io Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  build          Build both plain text books (no images)"
	@echo "  build-dragging Build the plain bevy_composable_dragging book"
	@echo "  build-gradmesh Build the plain bevy_gradmesh book"
	@echo "  build-wasm     Build the WASM runtime and both interactive books"
	@echo "  build-wasm-dragging  Build the interactive bevy_composable_dragging book"
	@echo "  build-wasm-gradmesh  Build the interactive bevy_gradmesh book"
	@echo "  build-wasm-runner Build shared WASM runtime (single binary, all examples)"
	@echo "  build-gallery  Build one binary per gallery showcase and the gallery manifest"
	@echo "  test-js        Run the JavaScript unit tests"
	@echo "  check-main-pages Check that main and index pages never mention WASM"
	@echo "  serve          Start local HTTP server on port $(PORT)"
	@echo "  publish        Build both versions, push to gh-pages"
	@echo "  check-links    Pull submodules, build books, check all links"
	@echo "  check-versions Check that book version references match Cargo.toml"
	@echo "  clean          Remove all built book output"
	@echo "  push-wasm-gh-pages  Push WASM to gh-pages (DANGER: gh-pages branch only)"
	@echo "  help           Show this help message"
	@echo ""
	@echo "Book versions:"
	@echo "  /bevy_gradmesh/              - Plain text (default)"
	@echo "  /bevy_gradmesh/wasm/         - Interactive (click-to-run demos)"
	@echo "  /bevy_composable_dragging/   - Plain text (default)"
	@echo "  /bevy_composable_dragging/wasm/ - Interactive (click-to-run demos)"
	@echo "  /demos/gallery.html          - Showcases, one download each"
	@echo ""
	@echo "Examples:"
	@echo "  make           Build both books (plain text)"
	@echo "  make build-wasm  Build both + WASM runtime"
	@echo "  make serve     Start local server"
	@echo "  make publish   Build and push both versions"
	@echo "  make PORT=9000 serve  Start server on port 9000"
	@echo ""
	@echo "WARNING: push-wasm-gh-pages will FAIL on any branch other than gh-pages."
