const { test, expect } = require('@playwright/test');
const { loadWithFixture } = require('./helpers');

test('history scrub and restart establish a fresh presentation epoch', async ({ page }) => {
  await loadWithFixture(page, 'player_boundary');
  await page.goto('/game.html');
  await page.waitForFunction(() => Number(document.getElementById('history-slider')?.max || 0) > 4,
    { timeout: 15000 });

  const play = page.locator('#play-log');
  const slider = page.locator('#history-slider');
  await slider.fill('0');
  await expect(play.locator('.play-icon')).toBeVisible();
  await page.waitForFunction(() => {
    const debug = window.__pystralThreeFrame?.presentation_debug;
    return debug?.history_index === 0 && debug?.playing_log === false;
  });
  const scrubbed = await page.evaluate(() => window.__pystralThreeFrame.presentation_debug);
  expect(scrubbed.movement_tweens).toEqual([]);

  await play.click();
  await page.waitForFunction((epoch) => {
    const debug = window.__pystralThreeFrame?.presentation_debug;
    return debug?.playing_log === true && debug.playback_epoch > epoch && debug.history_index > 0;
  }, scrubbed.playback_epoch);
  const resumed = await page.evaluate(() => window.__pystralThreeFrame.presentation_debug);
  expect(resumed.movement_tweens.every((tween) => tween.playback_epoch === resumed.playback_epoch)).toBe(true);

  await slider.fill('0');
  await page.waitForFunction((epoch) => {
    const debug = window.__pystralThreeFrame?.presentation_debug;
    return debug?.history_index === 0 && debug.playing_log === false
      && debug.playback_epoch > epoch && debug.movement_tweens.length === 0;
  }, resumed.playback_epoch);
});
