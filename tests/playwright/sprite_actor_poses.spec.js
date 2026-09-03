const { test, expect } = require('@playwright/test');

test('pose sampler interpolates adjacent keyframes deterministically', async ({ page }) => {
  await page.goto('/game.html');
  const sample = await page.evaluate(async () => {
    const { loadPoseCatalog, samplePose } = await import('/web/sprite_actor.js');
    const catalog = await loadPoseCatalog();
    const first = catalog.values().next().value;
    const result = samplePose(catalog, first.rig, first.clip, first.duration_ms / 4);
    return { result, frameCount: first.frames.length };
  });
  expect(sample.frameCount).toBeGreaterThan(0);
  expect(sample.result.bones.length).toBeGreaterThan(0);
  expect(sample.result.alpha).toBeGreaterThanOrEqual(0);
  expect(sample.result.alpha).toBeLessThan(1);
  for (const bone of sample.result.bones) {
    expect(Math.hypot(...bone.q)).toBeCloseTo(1, 4);
  }
});

test('missing pose clips are an explicit unavailable state', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => {
    const { loadPoseCatalog, samplePose } = await import('/web/sprite_actor.js');
    return samplePose(await loadPoseCatalog(), 'missing-rig', 'missing-clip', 0);
  });
  expect(result).toBeNull();
});
