import { test, expect } from '@playwright/test';

test('compact HUD uses perimeter mode circles and keeps the active panel bounded', async ({ page }) => {
  await page.setViewportSize({ width: 383, height: 852 });
  await page.goto('/game.html');
  await expect(page.locator('#hud-mode-bar')).toBeVisible();

  const modeBoxes = await page.locator('#hud-mode-bar button').evaluateAll((buttons) =>
    buttons.map((button) => {
      const box = button.getBoundingClientRect();
      return { width: box.width, height: box.height, top: box.top, right: box.right };
    }),
  );
  expect(modeBoxes.every(({ width, height, top, right }) =>
    width <= 38 && height <= 38 && top >= 0 && right <= 383)).toBeTruthy();

  const playControls = await page.locator('#history-log-camera-controls').evaluate((panel) => {
    const box = panel.getBoundingClientRect();
    return { top: box.top, bottom: box.bottom, height: box.height };
  });
  expect(playControls.top).toBeGreaterThanOrEqual(0);
  expect(playControls.bottom).toBeLessThanOrEqual(852);

  await page.locator('#hud-mode-bar button[data-hud-mode="history"]').click();
  const state = await page.evaluate(() => {
    const active = document.getElementById('history-viewer').getBoundingClientRect();
    const controls = document.getElementById('history-log-camera-controls').getBoundingClientRect();
    const visible = (id) => getComputedStyle(document.getElementById(id)).display !== 'none';
    return {
      active: { top: active.top, bottom: active.bottom, left: active.left, right: active.right, height: active.height },
      controls: { top: controls.top, bottom: controls.bottom },
      controlsVisible: getComputedStyle(document.getElementById('history-log-camera-controls')).display !== 'none',
      actionLogVisible: visible('action-log-panel'),
    };
  });
  expect(state.active.left).toBeGreaterThanOrEqual(0);
  expect(state.active.right).toBeLessThanOrEqual(383);
  if (state.controlsVisible) expect(state.active.bottom).toBeLessThanOrEqual(state.controls.top);
  expect(state.active.height).toBeLessThanOrEqual(852 * 0.2);
  expect(state.actionLogVisible).toBeFalsy();
});

test('HUD mode buttons switch the primary open panel', async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 768 });
  await page.goto('/game.html');

  const expectedPanels = {
    play: 'history-log-camera-controls',
    actions: 'action-stack',
    'unit-state': 'action-stack',
    history: 'history-viewer',
    diagnostics: 'diagnostics-panel',
  };
  for (const [mode, expectedPanel] of Object.entries(expectedPanels)) {
    await page.locator(`#hud-mode-bar button[data-hud-mode="${mode}"]`).click();
    await expect(page.locator(`#${expectedPanel}`)).toBeVisible();
    const otherPanels = Object.values(expectedPanels).filter((id) => id !== expectedPanel);
    for (const panel of [...new Set(otherPanels)]) {
      await expect(page.locator(`#${panel}`)).not.toBeVisible();
    }
    const actionLog = page.locator('#action-log-panel');
    if (mode === 'actions') await expect(actionLog).toBeVisible();
    else await expect(actionLog).not.toBeVisible();
  }
});

test('player-turn unit state stays bounded without pushing the HUD dock', async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 768 });
  await page.goto('/game.html');
  await page.waitForFunction(() => document.getElementById('action-menu')?.style.display === 'block', null, {
    timeout: 40000,
  });
  await expect(page.locator('#hud-mode-bar button[data-hud-mode="actions"]')).toHaveAttribute('aria-current', 'true');
  await expect(page.locator('#history-log-camera-controls')).not.toBeVisible();

  const state = await page.evaluate(() => {
    const ids = ['hud-mode-bar', 'history-log-camera-controls', 'action-stack', 'unit-state-panel', 'action-menu'];
    const boxes = Object.fromEntries(ids.map((id) => {
      const element = document.getElementById(id);
      const rect = element.getBoundingClientRect();
      return [id, {
        display: getComputedStyle(element).display,
        position: getComputedStyle(element).position,
        top: rect.top,
        bottom: rect.bottom,
        left: rect.left,
        right: rect.right,
      }];
    }));
    return { boxes, viewport: { width: innerWidth, height: innerHeight } };
  });

  for (const box of Object.values(state.boxes)) {
    if (box.display === 'none') continue;
    expect(box.left).toBeGreaterThanOrEqual(-1);
    expect(box.top).toBeGreaterThanOrEqual(-1);
    expect(box.right).toBeLessThanOrEqual(state.viewport.width + 1);
    expect(box.bottom).toBeLessThanOrEqual(state.viewport.height + 1);
  }
  expect(state.boxes['action-stack'].position).toBe('fixed');
});
