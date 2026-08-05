const { test, expect } = require('@playwright/test');

test.describe('Camera Navigation Errors', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for the app to be initialized
    await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });
  });

  test('clicking a missing camera direction should show error in ui_log', async ({ page }) => {
    // By default, arrows without neighbors are hidden.
    // We need to bypass the visibility check for this test to confirm that IF it were clicked, it would error.
    // Or we can find an arrow that IS visible but points to a non-existent camera if we can set up such a state.
    
    // For now, let's force a click on a hidden button via dispatching an event if needed, 
    // or just make them visible for the test.
    
    const navUp = page.locator('#nav-up');
    
    // Ensure it's hidden initially (demo log usually only has right/left)
    // await expect(navUp).toBeHidden(); // Playwright's toBeHidden checks CSS visibility: hidden or display: none
    
    // Force click it anyway
    await navUp.evaluate(node => node.dispatchEvent(new MouseEvent('click', { bubbles: true })));
    
    const logContainer = page.locator('#log-container');
    await expect(logContainer).toContainText('Camera navigation error');
  });
});
