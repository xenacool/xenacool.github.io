# web/scripts

Rhai scenario scripts for the pg_rpg runtime. `manifest.json` lists every file
fetched by production loads; `@include` paths are relative to the including
file, and a leading `/` is root-relative (bundle-root based).

- `scenarios/` — production scenarios; the only scenario in `manifest.json`.
- `fixtures/` — test-only overlays. Never listed in `manifest.json`; specs load
  them via the route intercept in `tests/playwright/helpers.js`. Each file's
  first line names its consuming spec(s).

`tests/playwright/fixture_coverage.spec.js` enforces: every fixture is
referenced by a spec, every spec-referenced fixture exists, and no fixture
leaks into the production manifest.
