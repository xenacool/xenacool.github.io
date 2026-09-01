const { test, expect } = require('@playwright/test');

test('compact actor manifest exposes explicit stride metadata', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => {
    const module = await import('/web/sprite_actor.js');
    const catalog = await module.loadActorCatalog('/web/sprite_actor_manifest.json');
    const actor = catalog.get('Caveman');
    return {
      actors: catalog.size,
      stride: actor?.slice_stride,
      samples: actor?.slice_influences?.length,
      influence: module.sliceInfluence(actor, 299),
      key: module.actorKey('models/Caveman.glb'),
    };
  });
  expect(result.actors).toBeGreaterThan(0);
  expect(result.stride).toBe(4);
  expect(result.samples).toBe(75);
  expect(result.influence).toEqual(expect.any(Array));
  expect(result.key).toBe('Caveman');
});

test('missing actor manifest is a normal unavailable state', async ({ page }) => {
  await page.goto('/game.html');
  const size = await page.evaluate(async () =>
    (await import('/web/sprite_actor.js')).loadActorCatalog('/missing-manifest.json')
      .then((catalog) => catalog.size));
  expect(size).toBe(0);
});
