const { test, expect } = require('@playwright/test');

test('render frame carries explicit pose identity without state inference', async ({ page }) => {
  await page.goto('/game.html');
  const source = await page.evaluate(async () => (await fetch('/web/renderer.js')).text());
  expect(source).toContain('entity.rig || null');
  expect(source).toContain('entity.animation_clip || null');
  expect(source).not.toContain('poseClip = entity.animation_state');
});
