const { test, expect } = require('@playwright/test');

test.describe('Camera lifecycle playback', () => {
  test.beforeEach(async ({ page }) => {
  await page.goto('/game.html');
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

});
