.PHONY: install clean build wasm-bindgen-tool build-wasm run-web server watch deploy test test-fast test-browser test-browser-sequential test-static test-integration test-rhai nuke-deploy playwright-install playwright-test playwright reproduce-spritestacks export-sprite-actor-manifest export-sprite-actor-poses report-animation-mappings temporal-bake-plan tla-check tla-worker-check tla-ui-check tla-animation-ack-check tla-simulation-bridge-check tla-casualty-boundary-check tla-lock-check debug-fixture-check check check-func-length check-loc

TEST_LOG := .make-test.log
# Keep the default feedback loop bounded. Browser and model tests should be
# made faster when they approach this budget, not allowed to grow silently.
TEST_BUDGET_SECONDS ?= 360
TEST_TIMEOUT := $(shell command -v timeout 2>/dev/null || command -v gtimeout 2>/dev/null)
WASM_BINDGEN_VERSION := 0.2.126
WASM_BINDGEN ?= $(CURDIR)/.tools/wasm-bindgen/bin/wasm-bindgen
TLA_VERSION := 1.8.0
TLA_TOOLS_DIR := spec/.tla-tools
TLA_TOOLS_JAR := $(TLA_TOOLS_DIR)/tla2tools.jar
TLA_BUILD_DIR := spec/_build
TLA_URL := https://github.com/tlaplus/tlaplus/releases/download/v$(TLA_VERSION)/tla2tools.jar

install:
	@if [ ! -d "assets/.git" ]; then \
		echo "Cloning assets..."; \
		if [ -e "assets" ]; then echo "assets exists but is not a Git checkout"; exit 1; fi; \
		git clone git@github.com:xenacool/xenacool_assets.git assets; \
	else \
		if [ -n "$$(git -C assets status --porcelain)" ]; then \
			echo "Assets checkout has local changes; leaving it untouched."; \
		else \
			git -C assets fetch origin main; \
			git -C assets checkout main; \
			git -C assets pull --ff-only origin main; \
		fi; \
	fi

playwright: playwright-install playwright-test

playwright-install:
	@echo "Installing Playwright and dependencies..."
	npm install
	npx playwright install --with-deps

# PLAYWRIGHT_ARGS passes through to npx playwright test, e.g.
#   make playwright-test PLAYWRIGHT_ARGS=fireball_victory
playwright-test: build-wasm
	@echo "Running Playwright tests..."
	npx playwright test $(PLAYWRIGHT_ARGS)

tla-tools:
	@if [ ! -f "$(TLA_TOOLS_JAR)" ]; then \
		mkdir -p "$(TLA_TOOLS_DIR)"; \
		echo "Downloading TLA+ tools $(TLA_VERSION)..."; \
		curl --fail --location --retry 3 "$(TLA_URL)" --output "$(TLA_TOOLS_JAR)"; \
	fi

tla-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/GameLoop.cfg \
		-metadir "$(TLA_BUILD_DIR)/GameLoop" \
		spec/GameLoop.tla
	@$(MAKE) --no-print-directory tla-worker-check tla-animation-ack-check tla-simulation-bridge-check tla-casualty-boundary-check tla-lock-check

tla-casualty-boundary-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/CasualtyBoundary.cfg \
		-metadir "$(TLA_BUILD_DIR)/CasualtyBoundary" \
		spec/CasualtyBoundary.tla

tla-worker-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/WorkerReplication.cfg \
		-metadir "$(TLA_BUILD_DIR)/WorkerReplication" \
		spec/WorkerReplication.tla

tla-ui-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/GameLoopUi.cfg \
		-metadir "$(TLA_BUILD_DIR)/GameLoopUi" \
		spec/GameLoopUi.tla

tla-animation-ack-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/AnimationAck.cfg \
		-metadir "$(TLA_BUILD_DIR)/AnimationAck" \
		spec/AnimationAck.tla

tla-simulation-bridge-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/SimulationBridge.cfg \
		-metadir "$(TLA_BUILD_DIR)/SimulationBridge" \
		spec/SimulationBridge.tla

# Lock-manager model: coordinates parallel subagents editing independent file
# paths under the browser UI.  Exhaustive safety over a 2-path tree (Root folder
# + F1 child) with two subagents (A1, A2) plus the USER, covering hierarchical
# locks, USER priority, acquire-before-modify, retry soundness, and
# read-your-own-write / happens-before via Lamport clocks.
tla-lock-check: tla-tools
	@mkdir -p "$(TLA_BUILD_DIR)"
	java -cp "$(TLA_TOOLS_JAR)" tlc2.TLC \
		-config spec/LockManager.cfg \
		-metadir "$(TLA_BUILD_DIR)/LockManager" \
		spec/LockManager.tla

debug-fixture-check:
	@python3 scripts/analyze_history.py debug-fixtures/movement_transition_case.json
	@python3 scripts/analyze_history.py debug-fixtures/control_input_flood_case.json

FN_LIMIT := 200

# Protocol/property coverage is intentionally kept close to the runtime while
# the larger modules are being decomposed. Keep this as a hard upper bound.
LOC_LIMIT := 750

# Outputs owned by the compiler/bindgen/export tools. Keep this list explicit:
# `make clean` must never remove source assets, the separate assets checkout,
# or tracked authored web data.
GENERATED_WEB_ASSETS := \
	web/atlas.json \
	web/spritesheet.png \
	web/sdfsheet.png \
	web/sprite_actor_manifest.json \
	web/pystral_gate.js \
	web/pystral_gate_bg.wasm

clean:
	@for file in $(GENERATED_WEB_ASSETS); do \
		if [ -e "$$file" ]; then echo "Removing $$file"; rm -f "$$file"; fi; \
	done

# Keep the bindgen executable aligned with Cargo.lock and independent of any
# globally installed CLI. WASM_BINDGEN remains overridable for CI/toolchains.
wasm-bindgen-tool:
	@if [ ! -x "$(WASM_BINDGEN)" ]; then \
		echo "Installing wasm-bindgen-cli $(WASM_BINDGEN_VERSION) locally..."; \
		cargo install wasm-bindgen-cli --version "$(WASM_BINDGEN_VERSION)" \
			--locked --root "$(CURDIR)/.tools/wasm-bindgen"; \
	fi
	@actual=$$($(WASM_BINDGEN) --version | awk '{print $$2}'); \
	if [ "$$actual" != "$(WASM_BINDGEN_VERSION)" ]; then \
		echo "wasm-bindgen $$actual found; expected $(WASM_BINDGEN_VERSION)" >&2; exit 1; \
	fi

check-loc:
	@echo "Checking lines of code per file..."
	@failures=""; \
	all_files=""; \
	for file in $$(find crates/ -name "*.rs"); do \
		lines=$$(wc -l < "$$file"); \
		all_files="$$all_files$$lines $$file\n"; \
		if [ $$lines -gt $(LOC_LIMIT) ]; then \
			echo "ERROR: $$file has $$lines lines (max $(LOC_LIMIT))"; \
			failures="$$failures $$file"; \
		fi; \
	done; \
	if [ -n "$$failures" ]; then \
		exit 1; \
	fi; \
	echo "All files are within $(LOC_LIMIT) lines."; \
	echo "Top 5 largest files:"; \
	echo "$$all_files" | sort -rn | head -5 | awk '{print $$1 " lines: " $$2}'

check-func-length:
	@echo "Checking function lengths..."
	@status=0; \
	report=$$(mktemp /tmp/func_check.XXXXXX); \
	trap 'rm -f "$$report"' EXIT; \
	for file in $$(find crates/ -name "*.rs"); do \
		awk '/fn [a-zA-Z_]/ { \
			fn_start=NR; fn_name=$$0; brace_count=0; in_fn=0; \
		} \
		in_fn || /fn [a-zA-Z_]/ { \
			in_fn=1; \
			for(i=1; i<=length($$0); i++) { \
				c=substr($$0,i,1); \
				if(c=="{") brace_count++; \
				if(c=="}") brace_count--; \
			} \
			if(brace_count==0 && in_fn) { \
				fn_len=NR-fn_start+1; \
				print fn_len " " FILENAME ":" fn_start " " fn_name; \
				if(fn_len > $(FN_LIMIT)) { \
					print "ERROR: " FILENAME ":" fn_start " function has " fn_len " lines (max $(FN_LIMIT))" > "/dev/stderr"; \
					print fn_name > "/dev/stderr"; \
					failed=1; \
				} \
				in_fn=0; \
			} \
		} \
		END { exit failed }' "$$file" >> "$$report" || status=1; \
	done; \
	if [ $$status -ne 0 ]; then \
		exit 1; \
	fi; \
	echo "All functions are within $(FN_LIMIT) lines."; \
	echo "Top 5 largest functions:"; \
	sort -rn "$$report" | head -5 | awk '{print $$1 " lines: " $$2 " " substr($$0, index($$0,$$3))}'

check: check-func-length check-loc

# Keep the structural limits in the default development loop so growing code
# gets refactored while its context is still fresh.
build: check
	cargo build --package pystral_compiler

build-wasm: check wasm-bindgen-tool
	@if [ ! -f "web/atlas.json" ] || [ ! -f "web/spritesheet.png" ]; then \
		touch crates/compiler/build.rs; \
	fi
	cargo build --target wasm32-unknown-unknown
	mkdir -p web
	$(WASM_BINDGEN) --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/debug/pystral_gate.wasm

test: check

# Auth-proxy end-to-end (Python): boots the REAL auth_proxy server on a scratch
# credentials/sessions store (never touching the repository's real store) and
# drives the full authentication surface, including the browser 'redirect after
# login' flow (form login -> 303 to the target page, not raw JSON).
selftest:
	python3 .harness-proxy/selftest.py

# The run/test script: boots the real proxy and exercises the auth flow,
# including the redirect-after-login, the open-redirect guard, and the
# websocket upgrade.
test-proxy:
	python3 .harness-proxy/run_proxy_test.py

# Run the actual proxy server (foreground) with the env-var config:
# the proxy listen port, the target/upstream host and port, the LAN
# address used by the certificate, and the bind address. `auth_proxy.py`
# `serve()` reads these env vars.
run-proxy:
	@en0_ip=$$(ipconfig getifaddr en0 2>/dev/null); \
	if [ -z "$$en0_ip" ]; then \
		echo "run-proxy: en0 has no IPv4 address; export PYSTRAL_BIND_HOST manually" >&2; \
		exit 1; \
	fi; \
	PYSTRAL_PROXY_PORT=8443 \
	PYSTRAL_TARGET_HOST=127.0.0.1 \
	PYSTRAL_TARGET_PORT=3080 \
	PYSTRAL_LAN_ADDRESS=$$en0_ip \
	PYSTRAL_BIND_HOST=$$en0_ip \
	python3 .harness-proxy/auth_proxy.py

test:
	@status=0;\
	echo "=== tla-check ==="; $(MAKE) --no-print-directory tla-check || status=1;\
	echo "=== test-browser ==="; $(MAKE) --no-print-directory test-browser || status=1;\
	echo "=== check-func-length ==="; $(MAKE) --no-print-directory check-func-length || status=1;\
	echo "=== check-loc ==="; $(MAKE) --no-print-directory check-loc || status=1;\
	echo "=== debug-fixture-check ==="; $(MAKE) --no-print-directory debug-fixture-check || status=1;\
	echo "=== selftest ==="; $(MAKE) --no-print-directory selftest || status=1;\
	echo "=== test-proxy ==="; $(MAKE) --no-print-directory test-proxy || status=1;\
	echo "=== cargo-test ==="; $(MAKE) --no-print-directory test-integration || status=1;\
	echo "Test output written above; aggregated status $$status";\
	exit $$status

# Independently invocable layers keep the aggregate timeout from hiding which
# verification class is slow or failing.
test-fast: tla-check check debug-fixture-check

test-browser: test-browser-sequential

# The runtime owns a shared WASM/WebGL lifecycle and the browser tests exercise
# that lifecycle heavily.  Serial coverage is the deterministic default;
# callers can still opt into parallel stress with PLAYWRIGHT_WORKERS=N.
PLAYWRIGHT_WORKERS ?= 1
test-browser-sequential:
	$(MAKE) --no-print-directory build-wasm
	npx playwright test --workers=$(PLAYWRIGHT_WORKERS) \
		--grep-invert "should show action buttons|move preview exposes accessible status and returns to the top-level menu|ability descriptors open legal targets and restore focus through the menu path|committed abilities report target count and restore the originating ability focus|Wait ends the player turn through the action protocol"
	npx playwright test --workers=1 \
		--grep "should show action buttons|move preview exposes accessible status and returns to the top-level menu|ability descriptors open legal targets and restore focus through the menu path|committed abilities report target count and restore the originating ability focus|Wait ends the player turn through the action protocol"

test-static: check debug-fixture-check

test-integration: check
	cargo test

test-rhai: check
	cargo test -p pystral_runtime --lib rhai_test_authoring


run-web: build-wasm
	python3 scripts/server.py 8000

# Dev loop. 'make watch' rebuilds the wasm on every rust change; 'make server'
# (re)starts :8080 serving the current build. The server sends
# Cache-Control: no-store, so the browser always picks up fresh assets.
server:
	@lsof -ti tcp:8080 -sTCP:LISTEN | xargs kill 2>/dev/null || true
	@nohup python3 scripts/server.py 8080 > .make-server.log 2>&1 &
	@echo "Server on http://localhost:8080 (log: .make-server.log)"

watch:
	cargo watch -x build-wasm

reproduce-spritestacks:
	@set -e; \
	npm --prefix assets/spracker run dev -- --host 127.0.0.1 > /tmp/pystral-spracker.log 2>&1 & \
	server_pid=$$!; \
	trap 'kill $$server_pid 2>/dev/null || true' EXIT INT TERM; \
	for attempt in $$(seq 1 60); do \
		if curl --silent --fail http://127.0.0.1:5173/ >/dev/null; then break; fi; \
		sleep 1; \
		if [ $$attempt -eq 60 ]; then \
			cat /tmp/pystral-spracker.log; \
			exit 1; \
		fi; \
	done; \
	node --experimental-strip-types scripts/reproduce_spritestacks.ts

export-sprite-actor-manifest: install
	@set -e; \
	npm --prefix assets/spracker run dev -- --host 127.0.0.1 > /tmp/pystral-spracker.log 2>&1 & \
	server_pid=$$!; \
	trap 'kill $$server_pid 2>/dev/null || true' EXIT INT TERM; \
	for attempt in $$(seq 1 60); do \
		if curl --silent --fail http://127.0.0.1:5173/ >/dev/null; then break; fi; \
		sleep 1; \
		if [ $$attempt -eq 60 ]; then cat /tmp/pystral-spracker.log; exit 1; fi; \
	done; \
	node --experimental-strip-types scripts/export_sprite_actor_manifest.ts; \
	bytes=$$(wc -c < web/sprite_actor_manifest.json); \
	if [ "$$bytes" -gt 700000 ]; then \
		echo "sprite actor manifest is too large ($$bytes bytes; limit 700000)" >&2; exit 1; \
	fi

export-sprite-actor-poses: install
	@set -e; npm --prefix assets/spracker run dev -- --host 127.0.0.1 > /tmp/pystral-spracker.log 2>&1 & \
	server_pid=$$!; trap 'kill $$server_pid 2>/dev/null || true' EXIT INT TERM; \
	for attempt in $$(seq 1 60); do curl --silent --fail http://127.0.0.1:5173/ >/dev/null && break; sleep 1; done; \
	node --experimental-strip-types scripts/export_sprite_actor_poses.ts

# One-time metadata export through the local Spracker page. The runtime only
# consumes web/animation_catalog.json and has no Spracker dependency.
export-animation-catalog:
	@set -e; \
	npm --prefix assets/spracker run dev -- --host 127.0.0.1 > /tmp/pystral-spracker.log 2>&1 & \
	server_pid=$$!; \
	trap 'kill $$server_pid 2>/dev/null || true' EXIT INT TERM; \
	for attempt in $$(seq 1 60); do \
		if curl --silent --fail http://127.0.0.1:5173/ >/dev/null; then break; fi; \
		sleep 1; \
		if [ $$attempt -eq 60 ]; then cat /tmp/pystral-spracker.log; exit 1; fi; \
	done; \
	node --experimental-strip-types scripts/export_animation_catalog.ts

# Dry-run the runtime state to source-clip mapping before temporal baking.
report-animation-mappings:
	node --experimental-strip-types scripts/report_animation_mappings.ts

# Validate the temporal manifest and report generated-layer cost without
# starting Spracker or mutating the separate assets repository.
temporal-bake-plan:
	node --experimental-strip-types scripts/temporal_bake_plan.ts

deploy:
	git checkout deploy || git checkout -b deploy
	git merge main --no-edit
	touch crates/compiler/build.rs
	$(MAKE) build-wasm
	grep -v "web/pystral_gate.js" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	grep -v "web/pystral_gate_bg.wasm" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	grep -v "web/spritesheet.png" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	grep -v "web/atlas.json" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	$(WASM_BINDGEN) --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/debug/pystral_gate.wasm
	# Keep every static entry point and its worker in the Pages artifact. All
	# asset URLs are document-relative, so this works at / locally and at a
	# repository subpath on github.io.
	git add index.html game.html editor.html rhai_worker.js web/enable-threads.js web/pystral_gate.js web/pystral_gate_bg.wasm web/spritesheet.png web/atlas.json .gitignore
	git commit -m "Update web release artifacts"
	git checkout main

nuke-deploy:
	git branch -D deploy || true
	git checkout -b deploy
	git checkout main
	git push origin deploy -f
