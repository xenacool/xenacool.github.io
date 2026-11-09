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

# Default port for local server
PORT ?= 8000

.PHONY: all build-dragging build-gradmesh build build-wasm build-wasm-dragging build-wasm-gradmesh build-examples build-examples-dragging build-examples-gradmesh serve clean clean-examples publish push-wasm-gh-pages check-links check-links-local check-versions help

all: build

## Build both mdBook docs with static screenshots (default)
build: build-dragging build-gradmesh

## Build bevy_composable_dragging book (static screenshots)
build-dragging:
	@echo "Building bevy_composable_dragging book..."
	rm -rf $(DRAGGING_OUTPUT)
	mkdir -p $(DRAGGING_OUTPUT)
	cd $(DRAGGING_SUBMODULE)/book && mdbook build --dest-dir ../../../$(DRAGGING_OUTPUT)
	mv $(DRAGGING_OUTPUT)/html/* $(DRAGGING_OUTPUT)/
	rm -rf $(DRAGGING_OUTPUT)/html $(DRAGGING_OUTPUT)/linkcheck
	@echo "Injecting WASM version toggle..."
	python3 scripts/postprocess_book.py $(DRAGGING_OUTPUT) bevy_composable_dragging --static-toggle
	@echo "Done: $(DRAGGING_OUTPUT)/"

## Build bevy_gradmesh book (static screenshots)
build-gradmesh:
	@echo "Building bevy_gradmesh book..."
	rm -rf $(GRADMESH_OUTPUT)
	mkdir -p $(GRADMESH_OUTPUT)
	cd $(GRADMESH_SUBMODULE)/book && mdbook build --dest-dir ../../../$(GRADMESH_OUTPUT)
	mv $(GRADMESH_OUTPUT)/html/* $(GRADMESH_OUTPUT)/
	rm -rf $(GRADMESH_OUTPUT)/html $(GRADMESH_OUTPUT)/linkcheck
	@echo "Injecting WASM version toggle..."
	python3 scripts/postprocess_book.py $(GRADMESH_OUTPUT) bevy_gradmesh --static-toggle
	@echo "Done: $(GRADMESH_OUTPUT)/"

## Build both books with WASM canvas post-processing
build-wasm: build-wasm-dragging build-wasm-gradmesh

## Build bevy_composable_dragging book with WASM canvases
build-wasm-dragging:
	@echo "Building bevy_composable_dragging book with WASM canvases..."
	mkdir -p $(DRAGGING_WASM_OUTPUT)
	cd $(DRAGGING_SUBMODULE)/book && mdbook build --dest-dir ../../../$(DRAGGING_WASM_OUTPUT)
	mv $(DRAGGING_WASM_OUTPUT)/html/* $(DRAGGING_WASM_OUTPUT)/
	rm -rf $(DRAGGING_WASM_OUTPUT)/html $(DRAGGING_WASM_OUTPUT)/linkcheck
	@echo "Post-processing for WASM canvases..."
	python3 scripts/postprocess_book.py $(DRAGGING_WASM_OUTPUT) bevy_composable_dragging --depth 3
	@echo "Done: $(DRAGGING_WASM_OUTPUT)/"

## Build bevy_gradmesh book with WASM canvases
build-wasm-gradmesh:
	@echo "Building bevy_gradmesh book with WASM canvases..."
	mkdir -p $(GRADMESH_WASM_OUTPUT)
	cd $(GRADMESH_SUBMODULE)/book && mdbook build --dest-dir ../../../$(GRADMESH_WASM_OUTPUT)
	mv $(GRADMESH_WASM_OUTPUT)/html/* $(GRADMESH_WASM_OUTPUT)/
	rm -rf $(GRADMESH_WASM_OUTPUT)/html $(GRADMESH_WASM_OUTPUT)/linkcheck
	@echo "Post-processing for WASM canvases..."
	python3 scripts/postprocess_book.py $(GRADMESH_WASM_OUTPUT) bevy_gradmesh --depth 3
	@echo "Done: $(GRADMESH_WASM_OUTPUT)/"

## Build all examples to WASM
build-examples: build-examples-dragging build-examples-gradmesh

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
	python3 -m http.server $(PORT)

## Publish: rebuild both static and WASM books, commit, and push to gh-pages
publish: check-versions build build-wasm
	@echo "Publishing to gh-pages..."
	git add $(DRAGGING_OUTPUT) $(GRADMESH_OUTPUT) $(DRAGGING_WASM_OUTPUT) $(GRADMESH_WASM_OUTPUT)
	git commit -m "Rebuild books: $(shell date +%Y-%m-%d)" || echo "No changes to commit"
	git push origin gh-pages
	@echo "Done."

## DANGER: Push WASM examples to gh-pages only — NEVER run automatically
## This target is intentionally excluded from all other targets.
## Usage: make push-wasm-gh-pages (only after confirming you are on gh-pages branch)
push-wasm-gh-pages:
	@if git branch --show-current | grep -q '^gh-pages$$'; then \
		echo "Pushing WASM examples to gh-pages..."; \
		git add demos/examples/; \
		git commit -m "Update WASM examples: $(shell date +%Y-%m-%d)" || echo "No WASM changes to commit"; \
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

## Clean all built book output
clean:
	@echo "Cleaning built book output..."
	rm -rf $(DRAGGING_OUTPUT)
	rm -rf $(GRADMESH_OUTPUT)
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
	@echo "  build          Build both mdBook docs with static screenshots (default)"
	@echo "  build-dragging Build bevy_composable_dragging book (static)"
	@echo "  build-gradmesh Build bevy_gradmesh book (static)"
	@echo "  build-wasm     Build both books with WASM canvas post-processing"
	@echo "  build-wasm-dragging  Build dragging book with WASM canvases"
	@echo "  build-wasm-gradmesh  Build gradmesh book with WASM canvases"
	@echo "  build-examples Build all examples to WASM"
	@echo "  build-examples-dragging  Build dragging examples to WASM"
	@echo "  build-examples-gradmesh  Build gradmesh examples to WASM"
	@echo "  serve          Start local HTTP server on port $(PORT)"
	@echo "  publish        Build both static and WASM books, push to gh-pages"
	@echo "  check-links    Pull submodules, build books, check all links against live site"
	@echo "  check-links-local  Pull submodules, build books, check links against local server"
	@echo "  check-versions Check that book version references match Cargo.toml"
	@echo "  clean          Remove all built book output"
	@echo "  clean-examples Remove all WASM example output"
	@echo "  push-wasm-gh-pages  Push WASM examples to gh-pages (DANGER: gh-pages branch only)"
	@echo "  help           Show this help message"
	@echo ""
	@echo "Book versions:"
	@echo "  Each book has two versions:"
	@echo "    /bevy_gradmesh/              - Static screenshots (default)"
	@echo "    /bevy_gradmesh/wasm/         - Live WASM canvases"
	@echo "    /bevy_composable_dragging/   - Static screenshots (default)"
	@echo "    /bevy_composable_dragging/wasm/ - Live WASM canvases"
	@echo "  Each version has a sidebar link to switch to the other."
	@echo ""
	@echo "Examples:"
	@echo "  make           Build both books (static screenshots)"
	@echo "  make build-wasm  Build both books (WASM canvases)"
	@echo "  make serve     Start local server"
	@echo "  make publish   Build and push both versions"
	@echo "  make build-examples  Compile all examples to WASM"
	@echo "  make check-links    Check all links on live site"
	@echo "  make check-versions Verify book versions match Cargo.toml"
	@echo "  make PORT=9000 serve  Start server on port 9000"
	@echo ""
	@echo "WARNING: push-wasm-gh-pages will FAIL on any branch other than gh-pages."
