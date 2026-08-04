// Shared Playwright helpers for pg_rpg browser tests.

/**
 * Route /game.html so the production scenario is replaced by a test-only
 * fixture. The fixture path is merged into the fetched manifest so its
 * @include graph resolves; production loads never fetch fixtures.
 */
async function loadWithFixture(page, fixture) {
  await page.route('**/web/scripts/pg_rpg.rhai', async (route) => {
    const response = await route.fetch();
    const source = await response.text();
    route.fulfill({
      response,
      body: source.replace(
        '@include "scenarios/skirmish.rhai"',
        `@include "fixtures/${fixture}.rhai"`,
      ),
    });
  });
  await page.route('**/web/scripts/manifest.json', async (route) => {
    const response = await route.fetch();
    const manifest = JSON.parse(await response.text());
    if (!manifest.files.includes(`fixtures/${fixture}.rhai`)) {
      manifest.files.push(`fixtures/${fixture}.rhai`);
    }
    route.fulfill({ response, body: JSON.stringify(manifest) });
  });
}

async function waitForGameReady(page) {
  await page.waitForFunction(
    () => document.body.dataset.loadingState === 'ready'
      && document.body.dataset.historyReady === 'true'
      && window.app !== undefined,
    null,
    { timeout: 40000 },
  );
}

module.exports = { loadWithFixture, waitForGameReady };
