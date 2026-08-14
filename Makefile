.PHONY: install build-wasm run-web deploy test nuke-deploy playwright-install playwright-test playwright reproduce-spritestacks tla-check tla-worker-check tla-ui-check tla-animation-ack-check tla-simulation-bridge-check tla-casualty-boundary-check debug-fixture-check

TEST_LOG := .make-test.log
# Keep the default feedback loop bounded. Browser and model tests should be
# made faster when they approach this budget, not allowed to grow silently.
TEST_BUDGET_SECONDS ?= 360
TEST_TIMEOUT := $(shell command -v timeout 2>/dev/null || command -v gtimeout 2>/dev/null)
TLA_VERSION := 1.8.0
TLA_TOOLS_DIR := spec/.tla-tools
TLA_TOOLS_JAR := $(TLA_TOOLS_DIR)/tla2tools.jar
TLA_BUILD_DIR := spec/_build
TLA_URL := https://github.com/tlaplus/tlaplus/releases/download/v$(TLA_VERSION)/tla2tools.jar

install:
	@if [ ! -d "assets" ]; then \
		echo "Cloning assets..."; \
		git clone git@github.com:xenacool/xenacool_assets.git assets; \
	else \
		echo "Assets already installed."; \
	fi

playwright: playwright-install playwright-test

playwright-install:
	@echo "Installing Playwright and dependencies..."
	npm install
	npx playwright install --with-deps

playwright-test: build-wasm
	@echo "Running Playwright tests..."
	npx playwright test

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
	@$(MAKE) --no-print-directory tla-worker-check tla-animation-ack-check tla-simulation-bridge-check tla-casualty-boundary-check

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

debug-fixture-check:
	@python3 scripts/analyze_history.py debug-fixtures/movement_transition_case.json
	@python3 scripts/analyze_history.py debug-fixtures/control_input_flood_case.json

FN_LIMIT := 200

# Protocol/property coverage is intentionally kept close to the runtime while
# the larger modules are being decomposed. Keep this as a hard upper bound.
LOC_LIMIT := 650

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
	@failures=""; \
	all_funcs=""; \
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
					print "ERROR: " FILENAME ":" fn_start " function has " fn_len " lines (max $(FN_LIMIT))"; \
					print fn_name; \
					exit 1; \
				} \
				in_fn=0; \
			} \
		}' "$$file" > /tmp/func_check_$$$$.txt || failures="$$failures $$file"; \
		all_funcs="$$all_funcs$$(cat /tmp/func_check_$$$$.txt)\n"; \
		rm -f /tmp/func_check_$$$$.txt; \
	done; \
	if [ -n "$$failures" ]; then \
		exit 1; \
	fi; \
	echo "All functions are within $(FN_LIMIT) lines."; \
	echo "Top 5 largest functions:"; \
	echo "$$all_funcs" | sort -rn | head -5 | awk '{print $$1 " lines: " $$2 " " substr($$0, index($$0,$$3))}'

build:
	cargo build --package pystral_compiler

build-wasm:
	@if [ ! -f "web/atlas.json" ] || [ ! -f "web/spritesheet.png" ]; then \
		touch crates/compiler/build.rs; \
	fi
	cargo build --target wasm32-unknown-unknown
	mkdir -p web
	wasm-bindgen --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/debug/pystral_gate.wasm

test:
	@if [ -z "$(TEST_TIMEOUT)" ]; then \
		echo "ERROR: make test needs timeout or gtimeout for its $(TEST_BUDGET_SECONDS)s budget"; \
		exit 1; \
	fi; \
	$(TEST_TIMEOUT) --signal=TERM $(TEST_BUDGET_SECONDS) sh -c '\
		status=0; \
		run_step() { \
			label="$$1"; shift; \
			echo "=== $$label ==="; \
			"$$@" || { echo "FAILED: $$label"; status=1; }; \
		}; \
		run_step tla-check $(MAKE) --no-print-directory tla-check; \
		run_step playwright-test $(MAKE) --no-print-directory playwright-test; \
		run_step check-func-length $(MAKE) --no-print-directory check-func-length; \
		run_step check-loc $(MAKE) --no-print-directory check-loc; \
		run_step debug-fixture-check $(MAKE) --no-print-directory debug-fixture-check; \
		run_step cargo-test cargo test; \
		exit $$status' > $(TEST_LOG) 2>&1; \
	status=$$?; \
	echo "Test output written to $(TEST_LOG)"; \
	if [ $$status -eq 124 ]; then echo "ERROR: make test exceeded $(TEST_BUDGET_SECONDS)s"; fi; \
	exit $$status


run-web: build-wasm
	python3 scripts/server.py 8000

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

deploy:
	git checkout deploy || git checkout -b deploy
	git merge main --no-edit
	touch crates/compiler/build.rs
	$(MAKE) build-wasm
	grep -v "web/pystral_gate.js" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	grep -v "web/pystral_gate_bg.wasm" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	grep -v "web/spritesheet.png" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	grep -v "web/atlas.json" .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
	wasm-bindgen --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/debug/pystral_gate.wasm
	git add index.html web/pystral_gate.js web/pystral_gate_bg.wasm web/spritesheet.png web/atlas.json .gitignore
	git commit -m "Update web release artifacts"
	git checkout main

nuke-deploy:
	git branch -D deploy || true
	git checkout -b deploy
	git checkout main
	git push origin deploy -f
