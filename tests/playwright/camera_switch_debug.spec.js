const { test, expect } = require('@playwright/test');

test('should switch camera and not show errors in ui_log', async ({ page }) => {
  page.on('console', msg => console.log('BROWSER CONSOLE:', msg.text()));
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const logContainer = page.locator('#log-container');
  const navRight = page.locator('#nav-right');

  console.log('Waiting for nav-right button...');
  await expect(navRight).toBeVisible({ timeout: 10000 });
  
  console.log('Clicking nav-right...');
  await navRight.click();
  
  // Wait for log update
  await page.waitForFunction(() => {
    const container = document.getElementById('log-container');
    return container && container.innerText.includes('Switched to camera') && container.innerText.includes('matrix:');
  }, { timeout: 10000 });

  const logText = await logContainer.innerText();
  console.log('Log content after switch:', logText);
  
  expect(logText).toContain('Switched to camera');
  expect(logText).not.toContain('Active camera');
  expect(logText).not.toContain('not found in state');
});
