const { test, expect } = require('@playwright/test');

test('actor mask dilation expands a composed silhouette deterministically', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => {
    const { dilateMask } = await import('/web/actor_mask.js');
    return Array.from(dilateMask([0, 0, 0, 0, 255, 0, 0, 0, 0], 3, 3, 1));
  });
  expect(result).toEqual([255, 255, 255, 255, 255, 255, 255, 255, 255]);
});

test('actor mask exposes WebGL2 alpha and dilation shaders', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => {
    const mask = await import('/web/actor_mask.js');
    return [mask.ACTOR_MASK_FRAGMENT_SHADER, mask.ACTOR_MASK_DILATE_FRAGMENT_SHADER];
  });
  expect(result[0]).toContain('discard');
  expect(result[1]).toContain('texel');
  expect(result[1]).toContain('1.-c');
});

test('actor mask pass bundles the three render stages', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => (await import('/web/actor_mask.js')).createMaskPass(64, 32));
  expect(result).toMatchObject({ width: 64, height: 32 });
  expect(result.dilationShader).toContain('mask');
});

test('actor mask compositor allocates a live render target', async ({ page }) => {
  await page.goto('/game.html?actor-mask=true');
  await page.waitForFunction(() => window.__pystralThreeMaskProfile, null, { timeout: 10000 });
  const profile = await page.evaluate(() => window.__pystralThreeMaskProfile);
  expect(profile.target[0]).toBeGreaterThan(0);
  expect(profile.target[1]).toBeGreaterThan(0);
});
