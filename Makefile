# Makefile for xenacool.github.io
# Build mdBook docs, serve locally, and publish to gh-pages

# Submodule paths
DRAGGING_SUBMODULE := libs/bevy_composable_dragging
GRADMESH_SUBMODULE := libs/bevy_gradmesh

# Output directories (relative to repo root)
DRAGGING_OUTPUT := bevy_composable_dragging
GRADMESH_OUTPUT := bevy_gradmesh

# Default port for local server
PORT ?= 8000

.PHONY: all build-dragging build-gradmesh build serve clean publish help

all: build

## Build both mdBook docs
build: build-dragging build-gradmesh

## Build bevy_composable_dragging book
build-dragging:
	@echo "Building bevy_composable_dragging book..."
	rm -rf $(DRAGGING_OUTPUT)
	mkdir -p $(DRAGGING_OUTPUT)
	cd $(DRAGGING_SUBMODULE)/book && mdbook build --dest-dir ../../../$(DRAGGING_OUTPUT)
	mv $(DRAGGING_OUTPUT)/html/* $(DRAGGING_OUTPUT)/
	rm -rf $(DRAGGING_OUTPUT)/html $(DRAGGING_OUTPUT)/linkcheck
	@echo "Done: $(DRAGGING_OUTPUT)/"

## Build bevy_gradmesh book
build-gradmesh:
	@echo "Building bevy_gradmesh book..."
	rm -rf $(GRADMESH_OUTPUT)
	mkdir -p $(GRADMESH_OUTPUT)
	cd $(GRADMESH_SUBMODULE)/book && mdbook build --dest-dir ../../../$(GRADMESH_OUTPUT)
	mv $(GRADMESH_OUTPUT)/html/* $(GRADMESH_OUTPUT)/
	rm -rf $(GRADMESH_OUTPUT)/html $(GRADMESH_OUTPUT)/linkcheck
	@echo "Done: $(GRADMESH_OUTPUT)/"

## Serve the site locally using Python's http.server
serve:
	@echo "Starting local server on http://localhost:$(PORT)"
	python3 -m http.server $(PORT)

## Publish to gh-pages branch
publish: build
	@echo "Publishing to gh-pages..."
	git checkout --orphan gh-pages-publish
	git rm -rf .
	git checkout main -- index.html css/ demos/ .gitattributes .gitmodules Makefile README.md
	git add $(DRAGGING_OUTPUT) $(GRADMESH_OUTPUT)
	git commit -m "Publish site: build books $(shell date +%Y-%m-%d)"
	git checkout main
	git branch -D gh-pages-publish
	@echo "Done. Push with: git push origin gh-pages-publish:gh-pages"

## Clean all built book output
clean:
	@echo "Cleaning built book output..."
	rm -rf $(DRAGGING_OUTPUT)/*
	rm -rf $(GRADMESH_OUTPUT)/*
	@echo "Done."

## Show help
help:
	@echo "xenacool.github.io Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  build          Build both mdBook docs (default)"
	@echo "  build-dragging Build bevy_composable_dragging book only"
	@echo "  build-gradmesh Build bevy_gradmesh book only"
	@echo "  serve          Start local HTTP server on port $(PORT)"
	@echo "  publish        Build books and create gh-pages commit (then push manually)"
	@echo "  clean          Remove all built book output"
	@echo "  help           Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make           Build both books"
	@echo "  make serve     Start local server"
	@echo "  make publish   Build and prepare gh-pages branch"
	@echo "  make PORT=9000 serve  Start server on port 9000"
