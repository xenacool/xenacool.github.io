const { test, expect } = require('@playwright/test');

test.describe('Camera Navigation Availability', () => {
  test.beforeEach(async ({ page }) => {
  await page.goto('/game.html');
    // Wait for the app to be initialized
    await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });
  });

  test('should only have visible navigation buttons when neighbors exist', async ({ page }) => {
    const slider = page.locator('#history-slider');
    const logContainer = page.locator('#log-container');
    
    // Wait for the worker to generate the initial pg_rpg log
    await page.waitForFunction(() => {
      const slider = document.getElementById('history-slider');
      return parseInt(slider.getAttribute('max'), 10) > 0;
    }, { timeout: 10000 });

    // Go to the end of the history log
    const maxVal = await slider.getAttribute('max');
    await slider.fill(maxVal);
    
    // Wait for state to be applied
    await page.waitForTimeout(500);

    const directions = ['up', 'down', 'left', 'right'];
    
    for (const dir of directions) {
      const btn = page.locator(`#nav-${dir}`);
      const isVisible = await btn.isVisible();
      
      if (isVisible) {
        console.log(`Clicking visible button: #nav-${dir}`);
        await btn.click();
        
        // Check that no error message like "Camera navigation error: No ... neighbor found" appeared
        const logContent = await logContainer.innerText();
        expect(logContent).not.toContain(`Camera navigation error: No ${dir} neighbor found`);
      } else {
        console.log(`Button #nav-${dir} is not visible, skipping click.`);
      }
    }
  });
});
