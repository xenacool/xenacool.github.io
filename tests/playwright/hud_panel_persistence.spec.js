const { test, expect } = require('@playwright/test');
const { waitForGameReady } = require('./helpers');

test('selected HUD panel persists across runtime trampoline updates', async ({ page }) => {
  await page.goto('/game.html');
  await waitForGameReady(page);

  await page.locator('#hud-mode-bar button[data-hud-mode="diagnostics"]').click();
  await page.evaluate(() => {
    const actions = {
      unit_id: 1, movement: [],
      primary_job: { name: 'Demo', abilities: [] }, secondary_jobs: [],
    };
    window.update_action_menu(JSON.stringify({ available_actions: actions, unit_states: [], game_completed: false }));
    window.update_action_menu(JSON.stringify({
      available_actions: null, unit_states: [], game_completed: false,
    }));
  });

  await expect(page.locator('#hud-dock')).toHaveAttribute('data-hud-mode', 'diagnostics');
  await expect(page.locator('#hud-mode-bar button[data-hud-mode="diagnostics"]'))
    .toHaveAttribute('aria-current', 'true');
});
