const { test, expect } = require('@playwright/test');

test.describe('Camera Navigation and History Slider', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for the app to be initialized
    await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });
  });

  test('should have a working history slider', async ({ page }) => {
    const slider = page.locator('#history-slider');
    const valueDisplay = page.locator('#slider-value');
    
    // Check initial state
    await expect(valueDisplay).toHaveText('0');
    
    // Wait for the worker to generate the demo log and update the slider max
    // The initial max is 0 (after main thread init), we expect it to be much higher (e.g., > 150)
    await page.waitForFunction(() => {
      const slider = document.getElementById('history-slider');
      return parseInt(slider.getAttribute('max'), 10) > 150;
    }, { timeout: 10000 });

    // Move slider
    await slider.fill('150');
    await expect(valueDisplay).toHaveText('150');
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
    
    // Expect max duration to increase as a new tween event is added
    const newMax = await slider.getAttribute('max');
    expect(Number(newMax)).toBeGreaterThan(Number(initialMax));
  });

  test('should not show errors in ui_log by default', async ({ page }) => {
    const logContainer = page.locator('#log-container');
    await expect(logContainer).toBeEmpty();
  });
});
