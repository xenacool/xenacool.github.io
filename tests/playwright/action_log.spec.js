const { test, expect } = require('@playwright/test');

test('should show action buttons and log clicks', async ({ page }) => {
  page.on('console', msg => console.log('BROWSER CONSOLE:', msg.text()));
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const actionControls = page.locator('#action-controls');
  const logContainer = page.locator('#log-container');
  
  // Wait for simulation to progress and show action buttons
  console.log('Waiting for action buttons to become visible...');
  await expect(actionControls).toBeVisible({ timeout: 20000 });
  
  const actionConfirm = page.locator('#action-confirm');
  await expect(actionConfirm).toBeVisible();
  
  // Click action button
  console.log('Clicking action button...');
  await actionConfirm.click();
  
  // Check if log contains the message
  console.log('Waiting for log update...');
  await page.waitForFunction(() => {
    const container = document.getElementById('log-container');
    return container && container.innerText.includes('Action input: confirm');
  }, { timeout: 5000 });
  
  const logText = await logContainer.innerText();
  expect(logText).toContain('Action input: confirm');
});
