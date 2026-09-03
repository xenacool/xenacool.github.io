const { test, expect } = require('@playwright/test');

test.fixme('demo_integ: NPCs commit an affordable targeted ability when available', async ({ page }) => {
  test.setTimeout(90000);
  await page.goto('/game.html');
  const log = page.locator('#action-log');
  await page.waitForFunction(() => {
    const text = document.getElementById('action-log')?.innerText || '';
    return /NPC unit \d+ used (Fireball|Icebolt|Attack)/.test(text);
  }, { timeout: 60000 });
  const text = await log.innerText();
  expect(text).toMatch(/NPC unit \d+ used (Fireball|Icebolt|Attack)/);
});
