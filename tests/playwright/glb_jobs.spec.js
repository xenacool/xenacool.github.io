const { test, expect } = require('@playwright/test');

test('every packaged job GLB loads through the native compositor', async ({ page }) => {
  test.setTimeout(45000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.models
    && window.__pystralThreeLoadGlbModels);

  const jobNames = await page.evaluate(() => Object.keys(window.__pystralThreeGlbManifest.models));
  expect(jobNames.length).toBe(63);

  const loaded = await page.evaluate((names) =>
    window.__pystralThreeLoadGlbModels(names).then((models) => models.map((model) => ({
      hasScene: Boolean(model.scene),
      animationCount: model.animations.length,
    }))), jobNames);
  expect(loaded).toHaveLength(jobNames.length);
  expect(loaded.every((model) => model.hasScene)).toBe(true);

  // Confirm a loaded job enters the native compositor path as an entity.
  await page.evaluate((asset) => window.dispatchEvent(new CustomEvent('pystral-render-frame', {
    detail: {
      version: 1,
      tick: 1,
      entities: [{ id: 1000, kind: 'character', asset, world_position: [0, 0, 0], scale: 1 }],
      cameras: [], map: null, materials: {},
    },
  })), jobNames[0]);
  await page.waitForFunction(() => window.__pystralThreeNativeMeshes?.get('1000')?.loaded);
});

test('packaged rig animation bundles expose the idle, walk, and combat clips', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.animation_bundles
    && window.__pystralThreeLoadAnimationBundles);
  const bundles = await page.evaluate(() => window.__pystralThreeLoadAnimationBundles(
    Object.keys(window.__pystralThreeGlbManifest.animation_bundles),
  ).then((loaded) => loaded.map((bundle) => bundle.clips.map((clip) => clip.name))));
  const clips = new Set(bundles.flat());
  for (const clip of ['Idle_A', 'Walking_A', 'Melee_1H_Attack_Chop',
    'Ranged_Magic_Spellcasting_Long', 'Ranged_Magic_Summon']) {
    expect(clips.has(clip), clip).toBe(true);
  }
});
