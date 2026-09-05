// playwright.config.js
const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests/playwright',
  // The production Rhai scenario is intentionally exercised end-to-end and
  // can take ~40s to reach a settled player boundary on a cold WASM start.
  // Keep one shared budget so tests fail on behavior, not startup variance.
  timeout: 90000,
  use: {
    baseURL: 'http://localhost:8080',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'python3 scripts/server.py 8080',
    port: 8080,
    reuseExistingServer: true,
  },
});
