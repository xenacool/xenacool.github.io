const { test, expect } = require('@playwright/test');

test('every packaged job GLB loads through the native compositor', async ({ page }) => {
  // Loading every packaged actor also warms the shared animation bundles and
  // can legitimately exceed the normal browser-test timeout on cold caches.
  test.setTimeout(120000);
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

  // The loader API is the authoritative native-compositor asset boundary;
  // entity insertion is exercised by renderer_performance.spec.js and must
  // not race the initial frame listener in this package-wide load contract.
  await page.waitForFunction((asset) =>
    window.__pystralThreeGlbLoadedModels?.get(asset)?.scene,
  jobNames[0]);
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
