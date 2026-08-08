const { test, expect } = require('@playwright/test');

test.describe('Camera lifecycle playback', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
  });

  async function pausePlayback(page) {
    const playLog = page.locator('#play-log');
    if (await playLog.locator('.pause-icon').isVisible()) await playLog.click();
    const playSim = page.locator('#play-sim');
    if (await playSim.locator('.pause-icon').isVisible()) await playSim.click();
  }

  async function expectNoErrors(page) {
    await expect(page.locator('#error-display')).toHaveText(/^(Error: 0|Info:)/);
  }

  test('starts history at position zero', async ({ page }) => {
    await page.check('#debug-checkbox');
    await pausePlayback(page);
    await page.waitForFunction(() => Number(document.getElementById('history-slider').max) > 0);
    await expect(page.locator('#history-slider')).toHaveValue('0');
    await expect(page.locator('#slider-value')).toHaveText('0');
  });

  test('removes despawned cameras and rewinds without UI log errors', async ({ page }) => {
    await page.check('#debug-checkbox');
    await pausePlayback(page);
    await page.waitForFunction(
      () => document.getElementById('history-viewer').innerText.includes('DespawnEntity'),
      { timeout: 15000 },
    );

    const despawnEntry = page.locator('.history-entry').filter({ hasText: 'DespawnEntity' }).first();
    await expect(despawnEntry).toBeVisible();
    const despawnIndex = Number((await despawnEntry.getAttribute('id')).replace('history-entry-', ''));

    await page.locator('#history-slider').fill(String(despawnIndex + 1));
    await page.waitForTimeout(500);
    await expect(page.locator('#entity-viewer')).not.toContainText('ID: 100 (camera)');
    await expectNoErrors(page);

    await page.locator('#history-slider').fill(String(Math.max(0, despawnIndex - 1)));
    await page.waitForTimeout(500);
    await expect(page.locator('#entity-viewer')).toContainText('ID: 100 (camera)');
    await expectNoErrors(page);

    await page.locator('#history-slider').fill(String(despawnIndex + 1));
    await page.waitForTimeout(500);
    await expectNoErrors(page);
  });
});
