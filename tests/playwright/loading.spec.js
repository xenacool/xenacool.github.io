const { test, expect } = require('@playwright/test');

test('game exposes an accessible loading state before controls become usable', async ({ page }) => {
  await page.route('**/web/glb_manifest.json', async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 500));
    await route.continue();
  });

  await page.goto('/game.html', { waitUntil: 'domcontentloaded' });
  const loading = page.getByRole('heading', { name: 'Loading game' });
  await expect(loading).toBeVisible();
  await expect(page.locator('#action-controls')).not.toBeVisible();
  await expect(page.getByRole('progressbar', { name: 'Game loading progress' })).toHaveAttribute('aria-valuenow', '30');

  await expect(loading).toBeHidden({ timeout: 30000 });
  await expect(page.locator('#action-controls')).toBeVisible();
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
});

test('initial live actors establish a drawable GLB happens-before readiness', async ({ page }) => {
  let releaseMage;
  const mageBlocked = new Promise((resolve) => { releaseMage = resolve; });
  await page.route('**/web/assets/models/jobs/Mage.glb', async (route) => {
    await mageBlocked;
    await route.continue();
  });

  await page.goto('/game.html', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => {
    const readiness = window.__pystralThreeInitialActors?.();
    return readiness?.required?.some((actor) => actor.asset === 'Mage')
      && readiness.assets.some((asset) => asset.kind === 'model' && asset.name === 'Mage'
        && asset.status === 'loading');
  }, { timeout: 15000 });
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'loading');
  expect(await page.locator('#action-controls').isVisible()).toBe(false);

  releaseMage();
  await page.waitForFunction(() => window.__pystralThreeInitialActors?.().status === 'ready',
    { timeout: 15000 });
  const readiness = await page.evaluate(() => window.__pystralThreeInitialActors());
  expect(readiness.attached.sort((a, b) => a - b)).toEqual(readiness.required.map((actor) => actor.id).sort((a, b) => a - b));
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
});

test('required actor model retries before releasing the live renderer', async ({ page }) => {
  test.setTimeout(20000);
  let attempts = 0;
  await page.route('**/web/assets/models/jobs/Mage.glb', async (route) => {
    attempts += 1;
    if (attempts <= 3) await route.abort('failed');
    else await route.continue();
  });

  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeInitialActors?.().assets.some((asset) =>
    asset.kind === 'model' && asset.name === 'Mage' && asset.status === 'retrying' && asset.attempt === 3),
  { timeout: 10000 });
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'loading');
  await page.waitForFunction(() => window.__pystralThreeInitialActors?.().status === 'ready',
    { timeout: 15000 });
  expect(attempts).toBe(4);
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
});

test('exhausted required actor load names its terminal presentation failure', async ({ page }) => {
  test.setTimeout(20000);
  await page.route('**/web/assets/models/jobs/Mage.glb', (route) => route.abort('failed'));

  await page.goto('/game.html');
  await expect(page.getByRole('alert')).toContainText('Required GLB asset failed: Mage', { timeout: 15000 });
  await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'error');
});

test('loading failure names the failed asset and retry recovers', async ({ page }) => {
  let attempts = 0;
  await page.route('**/web/glb_manifest.json', async (route) => {
    attempts += 1;
    // The Rust startup loader requests the GLB manifest authoritatively;
    // the renderer also consumes it for model caching.
    if (attempts <= 2) {
      await route.abort('failed');
      return;
    }
    await route.continue();
  });

  await page.goto('/game.html');
  await expect(page.getByRole('alert')).toContainText('Asset loading failed');
  await expect(page.getByRole('alert')).toContainText('web/glb_manifest.json');
  const retry = page.getByRole('button', { name: 'Retry' });
  await expect(retry).toBeVisible();
  await retry.click();
  await expect(page.locator('#loading-panel')).toBeHidden({ timeout: 30000 });
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
  expect(attempts).toBeGreaterThanOrEqual(3);
});

test('repeated retry replaces the attempt without duplicating controls or reloading', async ({ page }) => {
  let attempts = 0;
  await page.route('**/web/glb_manifest.json', async (route) => {
    attempts += 1;
    if (attempts < 3) {
      await route.abort('failed');
      return;
    }
    await route.continue();
  });

  await page.goto('/game.html');
  await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.locator('#loading-panel')).toBeHidden({ timeout: 30000 });
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
  expect(await page.evaluate(() => ({
    attempts: window.__pystralRunAttempts,
    reloads: window.performance.getEntriesByType('navigation').length,
    controlsBound: window.__pystralControlsBound,
  }))).toEqual({ attempts: 3, reloads: 1, controlsBound: true });
});
