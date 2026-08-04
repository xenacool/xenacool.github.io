// Ad-hoc: load render_synthetic fixture, dump entity/asset state from worker.
const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  await page.route('**/web/scripts/pg_rpg.rhai', async (route) => {
    const response = await route.fetch();
    const source = await response.text();
    route.fulfill({
      response,
      body: source.replace(
        '@include "scenarios/skirmish.rhai"',
        '@include "fixtures/render_synthetic.rhai"',
      ),
    });
  });
  await page.route('**/web/scripts/manifest.json', async (route) => {
    const response = await route.fetch();
    const manifest = JSON.parse(await response.text());
    if (!manifest.files.includes('fixtures/render_synthetic.rhai')) {
      manifest.files.push('fixtures/render_synthetic.rhai');
    }
    route.fulfill({ response, body: JSON.stringify(manifest) });
  });
  await page.goto('http://localhost:8080/game.html');
  for (let i = 0; i < 24; i++) {
    await page.waitForTimeout(5000);
    const st = await page.evaluate(() => window.__pystralWorkerStatus || '');
    if (st.includes('AwaitingPlayerDecision')) break;
  }
  console.log('status:', await page.evaluate(() => window.__pystralWorkerStatus));
  const info = await page.evaluate(() => {
    const state = window.__pystralLastTransientState;
    if (!state) return { error: 'no transient state' };
    const out = { topKeys: Object.keys(state) };
    if (state.asset_collections) {
      out.asset_collections = Object.fromEntries(
        Object.entries(state.asset_collections).map(([k, v]) => [k, (v && v.length) || v]),
      );
    }
    const ents = state.entities || state.agents || state.units;
    if (ents) {
      out.entities = ents.map((e) => ({
        id: e.id,
        cell: e.cell ?? e.position,
        properties: e.properties,
      }));
    }
    if (state.projectiles) out.projectiles = state.projectiles.length;
    return out;
  });
  console.log(JSON.stringify(info, null, 2));
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
