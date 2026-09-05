const { test, expect } = require('@playwright/test');

test('game exposes an accessible loading state before controls become usable', async ({ page }) => {
  await page.route('**/web/atlas.json', async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 500));
    await route.continue();
  });

  await page.goto('/game.html');
  const loading = page.getByRole('heading', { name: 'Loading game' });
  await expect(loading).toBeVisible();
  await expect(page.locator('#action-controls')).not.toBeVisible();
  await expect(page.getByRole('progressbar', { name: 'Game loading progress' })).toHaveAttribute('aria-valuenow', '25');

  await expect(loading).toBeHidden({ timeout: 15000 });
  await expect(page.locator('#action-controls')).toBeVisible();
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
});

test('loading failure names the failed asset and retry recovers', async ({ page }) => {
  let attempts = 0;
  await page.route('**/web/atlas.json', async (route) => {
    attempts += 1;
    // The renderer and Rust loader both request atlas.json during one
    // startup attempt. Fail both consumers so the test observes the
    // authoritative loader error rather than racing the renderer's optional
    // atlas lookup.
    if (attempts <= 2) {
      await route.abort('failed');
      return;
    }
    await route.continue();
  });

  await page.goto('/game.html');
  await expect(page.getByRole('alert')).toContainText('Asset loading failed');
  await expect(page.getByRole('alert')).toContainText('web/atlas.json');
  const retry = page.getByRole('button', { name: 'Retry' });
  await expect(retry).toBeVisible();
  await retry.click();
  await expect(page.locator('#loading-panel')).toBeHidden({ timeout: 15000 });
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
  expect(attempts).toBeGreaterThanOrEqual(3);
});

test('repeated retry replaces the attempt without duplicating controls or reloading', async ({ page }) => {
  let attempts = 0;
  await page.route('**/web/atlas.json', async (route) => {
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
  await expect(page.locator('#loading-panel')).toBeHidden({ timeout: 15000 });
  await expect(page.locator('body')).toHaveAttribute('data-loading-state', 'ready');
  expect(await page.evaluate(() => ({
    attempts: window.__pystralRunAttempts,
    reloads: window.performance.getEntriesByType('navigation').length,
    controlsBound: window.__pystralControlsBound,
  }))).toEqual({ attempts: 3, reloads: 1, controlsBound: true });
});
