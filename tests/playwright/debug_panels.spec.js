const { test, expect } = require('@playwright/test');

test.describe('Debug Panels BDD', () => {
  test.beforeEach(async ({ page }) => {
  await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
  });

  test('Toggle debug mode and verify panels visibility', async ({ page }) => {
    await test.step('Given the application is loaded', async () => {
      await expect(page.locator('#entity-viewer')).not.toBeVisible();
      await expect(page.locator('#history-viewer')).not.toBeVisible();
    });

    await page.locator('#hud-mode-bar button[data-hud-mode="diagnostics"]').click();

    await test.step('Then Diagnostics shows the diagnostic surfaces', async () => {
      await expect(page.locator('#entity-viewer')).toBeVisible();
      await expect(page.locator('#history-viewer')).toBeVisible();
      await expect(page.locator('#heartbeat-panel')).toBeVisible();
    });

    await test.step('When the collision Debug checkbox is checked', async () => {
      await page.check('#debug-checkbox');
    });

    await test.step('Then Diagnostics remains visible', async () => {
      await expect(page.locator('#entity-viewer')).toBeVisible();
      await expect(page.locator('#history-viewer')).toBeVisible();
      await expect(page.locator('#heartbeat-panel')).toBeVisible();
    });

    await test.step('When the collision Debug checkbox is unchecked', async () => {
      await page.uncheck('#debug-checkbox');
    });

    await test.step('Then Diagnostics remains visible', async () => {
      await expect(page.locator('#entity-viewer')).toBeVisible();
      await expect(page.locator('#history-viewer')).toBeVisible();
      await expect(page.locator('#heartbeat-panel')).toBeVisible();
    });
  });

  test('Sync debug panels when scrubbing history', async ({ page }) => {
    await test.step('Given debug mode is active and log scrubbing is paused', async () => {
      await page.locator('#hud-mode-bar button[data-hud-mode="diagnostics"]').click();
      await page.check('#debug-checkbox');
      await page.locator('#hud-mode-bar button[data-hud-mode="play"]').click();
      
      // Ensure log scrubbing and simulation are paused for stable testing
      const playLogBtn = page.locator('#play-log');
      await expect(playLogBtn).toBeVisible();
      if (await playLogBtn.locator('.pause-icon').isVisible()) {
        await playLogBtn.click();
        await expect(playLogBtn.locator('.play-icon')).toBeVisible();
      }
      
      const playSimBtn = page.locator('#play-sim');
      await expect(playSimBtn).toBeVisible();
      if (await playSimBtn.locator('.pause-icon').isVisible()) {
        await playSimBtn.click();
        await expect(playSimBtn.locator('.play-icon')).toBeVisible();
      }

      await page.waitForFunction(() => {
        const slider = document.getElementById('history-slider');
        return parseInt(slider.getAttribute('max'), 10) > 0;
      }, { timeout: 10000 });
    });

    await test.step('When I scrub the history slider', async () => {
      const slider = page.locator('#history-slider');
      await slider.fill('10');
      // Wait for UI to update
      await page.waitForTimeout(500);
    });

    await page.locator('#hud-mode-bar button[data-hud-mode="diagnostics"]').click();

    await test.step('Then the entity viewer should display entity details', async () => {
      const entityViewer = page.locator('#entity-viewer');
      await expect(entityViewer).toContainText('Entities');
      await expect(entityViewer).toContainText('ID:');
    });

    await test.step('And the history viewer should highlight the correct event', async () => {
      const currentEntry = page.locator('#history-entry-10');
      await expect(currentEntry).toHaveClass(/current/);
      await expect(currentEntry).toBeVisible();
    });

    await test.step('When I click a history entry in the viewer', async () => {
        const entry5 = page.locator('#history-entry-5');
        await entry5.click();
        await page.waitForTimeout(500);
    });

    await test.step('Then the history slider and value should update', async () => {
        const slider = page.locator('#history-slider');
        const sliderValue = page.locator('#slider-value');
        expect(await slider.inputValue()).toBe('5');
        expect(await sliderValue.innerText()).toBe('5');
    });
  });
});
