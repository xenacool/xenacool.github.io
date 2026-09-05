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

  test('camera arrows use axial q/r deltas without changing gameplay history', async ({ page }) => {
    await page.waitForFunction(() => window.__pystralCameraAxial !== undefined);
    await page.waitForFunction(() => document.body.dataset.historyReady === 'true');
    await page.waitForFunction(() => {
      const current = Number(document.getElementById('history-slider').max);
      const previous = window.__cameraHistoryMax;
      window.__cameraHistoryMax = current;
      window.__cameraHistoryStable = previous === current
        ? (window.__cameraHistoryStable || 0) + 1 : 0;
      return current > 0 && window.__cameraHistoryStable >= 3;
    });
    const before = await page.evaluate(() => ({
      axial: { ...window.__pystralCameraAxial },
      history: document.getElementById('history-slider').max,
      replayInputs: window.__pystralReplayInputs.length,
    }));
    const left = page.locator('#nav-left');
    const right = page.locator('#nav-right');
    await expect.poll(() => page.evaluate(() => ({
      left: document.getElementById('nav-left').style.display,
      right: document.getElementById('nav-right').style.display,
    })), { timeout: 15000 }).toEqual({ left: 'block', right: 'block' });

    await left.click();
    await expect.poll(() => page.evaluate(() => ({
      ...window.__pystralCameraAxial,
      replayInputs: window.__pystralReplayInputs.length,
    }))).toEqual({ q: before.axial.q - 1, r: before.axial.r, replayInputs: before.replayInputs });

    await page.keyboard.press('ArrowRight');
    await expect.poll(() => page.evaluate(() => ({
      ...window.__pystralCameraAxial,
      replayInputs: window.__pystralReplayInputs.length,
    }))).toEqual({ q: before.axial.q, r: before.axial.r, replayInputs: before.replayInputs });

    await page.keyboard.press('ArrowDown');
    await expect.poll(() => page.evaluate(() => ({
      ...window.__pystralCameraAxial,
      replayInputs: window.__pystralReplayInputs.length,
    }))).toEqual({ q: before.axial.q, r: before.axial.r + 1, replayInputs: before.replayInputs });
  });

});
