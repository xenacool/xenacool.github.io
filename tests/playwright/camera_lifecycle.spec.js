const { test, expect } = require('@playwright/test');
const { loadWithFixture } = require('./helpers');

test.describe('Camera lifecycle playback', () => {
  test.beforeEach(async ({ page }) => {
    await loadWithFixture(page, 'player_boundary');
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

  test('can scrub generated history back to position zero', async ({ page }) => {
    await page.locator('#hud-mode-bar button[data-hud-mode="diagnostics"]').click();
    await page.check('#debug-checkbox');
    await page.locator('#hud-mode-bar button[data-hud-mode="play"]').click();
    await pausePlayback(page);
    await page.waitForFunction(() => Number(document.getElementById('history-slider').max) > 0);
    await page.locator('#history-slider').fill('0');
    await expect(page.locator('#history-slider')).toHaveValue('0');
    await expect(page.locator('#slider-value')).toHaveText('0');
  });

});
