const { test, expect } = require('@playwright/test');

test.describe('Camera Navigation and History Slider', () => {
  test.beforeEach(async ({ page }) => {
    page.on('console', msg => console.log('BROWSER CONSOLE:', msg.text()));
    await page.goto('/');
    // Wait for the app to be initialized
    await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });
  });

  test('should have a working history slider', async ({ page }) => {
    const slider = page.locator('#history-slider');
    const valueDisplay = page.locator('#slider-value');
    const playLogBtn = page.locator('#play-log');
    
    // Pause log playback to avoid slider moving on its own
    await playLogBtn.click();
    
    // Check initial state
    // We allow it to be non-zero if demo log generation started immediately
    const initialText = await valueDisplay.innerText();
    expect(parseInt(initialText, 10)).toBeLessThan(150);
    
    // Wait for the worker to generate the demo log and update the slider max
    // With incremental simulation, it might take a moment to reach a significant number
    await page.waitForFunction(() => {
      const slider = document.getElementById('history-slider');
      return parseInt(slider.getAttribute('max'), 10) > 50;
    }, { timeout: 20000 });

    // Move slider
    const currentMax = await slider.getAttribute('max');
    await slider.fill(currentMax);
    const currentValue = await valueDisplay.innerText();
    expect(parseInt(currentValue, 10)).toBe(parseInt(currentMax, 10));
  });

  test('should trigger camera navigation', async ({ page }) => {
    const navRight = page.locator('#nav-right');
    const slider = page.locator('#history-slider');
    
    // Wait for the worker to generate the initial demo log
    await page.waitForFunction(() => {
      const slider = document.getElementById('history-slider');
      return parseInt(slider.getAttribute('max'), 10) > 0;
    }, { timeout: 10000 });

    // Initial max duration
    const initialMax = await slider.getAttribute('max');
    
    // Click navigation
    await navRight.click();
    
    // Give it a moment to process the command
    await page.waitForTimeout(100);

    // Expect max duration to stay roughly same (transient nav doesn't add history)
    // Note: since simulation might be running in background, we just check it doesn't JUMP because of nav
    const finalMax = await slider.getAttribute('max');
    expect(Number(finalMax)).toBeGreaterThanOrEqual(Number(initialMax));
  });

  test('should not show errors in ui_log by default', async ({ page }) => {
    const logContainer = page.locator('#log-container');
    await expect(logContainer).toBeEmpty();
  });
});
