// One-off debug: load game.html with render_synthetic fixture, dump UI log.
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const FIXTURE = process.argv[2] || 'render_synthetic.rhai';

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const logs = [];
  page.on('console', (msg) => logs.push(`[console.${msg.type()}] ${msg.text()}`));
  page.on('pageerror', (err) => logs.push(`[pageerror] ${err.message}`));

  const fixture = FIXTURE.replace(/\.rhai$/, '');
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

  const t0 = Date.now();
  await page.goto('http://localhost:8080/game.html');
  let settled = false;
  for (let i = 0; i < 24; i++) {
    await page.waitForTimeout(5000);
    const st = await page.evaluate(() => window.__pystralWorkerStatus || '');
    console.log(`t+${Math.round((Date.now() - t0) / 1000)}s: ${st}`);
    if (st.includes('AwaitingPlayerDecision')) { settled = true; break; }
  }
  console.log(`settled=${settled} at t+${Math.round((Date.now() - t0) / 1000)}s`);

  const state = await page.evaluate(() => ({
    status: window.__pystralWorkerStatus || '',
    uiLog: document.getElementById('log-container')?.textContent || '',
    traces: window.__pystralDebugTraces || [],
  }));
  console.log('=== WORKER STATUS ===');
  console.log(state.status);
  console.log('=== UI LOG ===');
  console.log(state.uiLog);
  console.log('=== CONSOLE ===');
  console.log(logs.join('\n'));
  await page.evaluate(() => {
    for (const el of document.body.children) {
      if (el.id !== 'canvas') el.style.display = 'none';
    }
  });
  await page.locator('#canvas').screenshot({ path: 'tests/playwright/debug_fixture.png' });
  await page.evaluate(() => {
    for (const el of document.body.children) el.style.display = '';
  });
  console.log('screenshot: tests/playwright/debug_fixture.png (UI hidden)');
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
