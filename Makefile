.PHONY: install build-wasm run-web deploy test nuke-deploy playwright-install playwright-test playwright

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

FN_LIMIT := 200

LOC_LIMIT := 500

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
	cargo build

build-wasm:
	cargo build --target wasm32-unknown-unknown
	mkdir -p web
	wasm-bindgen --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/debug/pystral_gate.wasm

test: playwright-test check-func-length check-loc
	cargo test


run-web: build-wasm
	python3 scripts/server.py 8000

slice-demo:
	@echo "Automation removed. Assets are now manually defined in pystral_compiler."

deploy: build-wasm
	git checkout deploy || git checkout -b deploy
	git merge main --no-edit
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