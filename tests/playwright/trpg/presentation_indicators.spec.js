const { test, expect } = require('@playwright/test');

test('authoritative facing indicator is visible above the actor scene', async ({ page }) => {
  page.on('pageerror', (error) => console.log('pageerror', error.message));
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.models);
  const marker = await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent('pystral-render-frame', { detail: {
      entities: [{
        id: 77, kind: 'character', asset: 'caveman', world_position: [0, 0, 0],
        facing: 'north', render_order: 1, scale: 1,
        indicator: { kind: 'facing', color: [1, 0.5, 0], state: 'committed', direction: 'north' },
      }, {
        id: 78, kind: 'projectile', asset: 'caveman', world_position: [3, 0, 3], facing: 'south',
        indicator: { kind: 'facing', color: [1, 1, 1], state: 'committed', direction: 'south' },
      }],
      cameras: [], map: null, materials: {}, camera_pose: null,
    } }));
    const group = window.__pystralThreeNativeMarkers.get('77');
    return {
      children: group?.children.length || 0,
      renderOrder: group?.renderOrder || 0,
      y: group?.position.y || 0,
      depthTest: group?.children[0]?.material?.depthTest,
      arrowHeight: group?.children[1]?.geometry?.parameters?.height,
    };
  });
  expect(marker.children).toBe(2);
  expect(marker.renderOrder).toBeGreaterThan(100000);
  expect(marker.y).toBeGreaterThan(0);
  expect(marker.depthTest).toBe(false);
  expect(marker.arrowHeight).toBeGreaterThan(0.3);
});

test('GLB actor and direction indicator share the six logical headings', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.models);
  const headings = await page.evaluate(() => {
    const facings = ['north', 'northeast', 'southeast', 'south', 'southwest', 'northwest'];
    return facings.map((facing, index) => {
      const id = 810 + index;
      window.dispatchEvent(new CustomEvent('pystral-render-frame', { detail: {
        version: 1, tick: id, cameras: [], map: null, materials: {}, entities: [{
          id, kind: 'character', asset: 'test-facing-primitive', world_position: [index, 0, 0],
          facing, rotation_y: 0.125, scale: 1,
          indicator: { kind: 'facing', color: [1, 1, 1], state: 'committed', direction: facing },
        }],
      } }));
      const actor = window.__pystralThreeNativeMeshes.get(String(id)).group;
      const marker = window.__pystralThreeNativeMarkers.get(String(id));
      return { facing, actorYaw: actor.rotation.y, markerYaw: marker.rotation.y };
    });
  });
  for (const heading of headings) {
    expect(heading.actorYaw - heading.markerYaw).toBeCloseTo(Math.PI + 0.125, 8);
  }
  expect(new Set(headings.map(({ actorYaw }) => actorYaw.toFixed(6))).size).toBe(6);
});

test('transient projectile presentation moves between logical render ticks and is removed', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.models);
  const result = await page.evaluate(() => {
    const frame = (tick, entities) => window.dispatchEvent(new CustomEvent('pystral-render-frame', {
      detail: { version: 1, tick, cameras: [], map: null, materials: {}, entities },
    }));
    frame(901, [{ id: 1000001, kind: 'projectile', asset: 'FireballOrb', scale: 0.35,
      world_position: [0, 1.8, 0], facing: 'northeast' }]);
    const start = window.__pystralThreeNativeMeshes.get('1000001').group.position.toArray();
    frame(902, [{ id: 1000001, kind: 'projectile', asset: 'FireballOrb', scale: 0.35,
      world_position: [3, 1.8, -2], facing: 'northeast' }]);
    const end = window.__pystralThreeNativeMeshes.get('1000001').group.position.toArray();
    frame(903, []);
    return { start, end, removed: !window.__pystralThreeNativeMeshes.has('1000001') };
  });
  expect(result.end).not.toEqual(result.start);
  expect(result.removed).toBe(true);
});

test('tactical compass anchors to the lowest tile and exposes six directions', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.models);
  await page.evaluate(() => window.dispatchEvent(new CustomEvent('pystral-render-frame', { detail: {
    entities: [], cameras: [], materials: {}, camera_pose: null,
    map: { orientation: 'flat', hex_size: [1, 1], tiles: [{ q: 2, r: -1, layer: 0, bottom: 0, height: 1, material: 'grass' }] },
  } })));
  const compass = await page.evaluate(() => ({
    children: window.__pystralThreeCompass?.children.length || 0,
    y: window.__pystralThreeCompass?.position.y || 0,
  }));
  expect(compass.children).toBe(7);
  expect(compass.y).toBeGreaterThan(1);
});

test('waypoint preview shares marker geometry and cleans up replaced paths', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.__pystralThreeGlbManifest?.models);
  const first = await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent('pystral-render-frame', { detail: {
    entities: [], cameras: [], map: null, materials: {}, camera_pose: null,
    presentation: {
      reachable_color: [0.1, 0.2, 0.3], path_color: [0.3, 0.6, 0.9],
      selected_color: [1, 0.8, 0.1], reachable_opacity: 0.25,
      path_opacity: 0.6, selected_opacity: 0.95, marker_scale: 1,
    },
    waypoint_preview: {
      unit_id: 7,
      reachable: [{ q: 1, r: 0, layer: 0, world_position: [1, 0, 0] }],
      path: [{ q: 1, r: 0, layer: 0, world_position: [1, 0, 0] }],
      selected_destination: { q: 2, r: -1, layer: 0, world_position: [2, 0, -1] },
    },
    } }));
    return {
      count: window.__pystralThreeNativeWaypointMarkers.size,
      reachable: window.__pystralThreeNativeWaypointMarkers.get('reachable:1:0:0')?.children[0]?.material?.opacity,
    };
  });
  expect(first.count).toBe(3);
  expect(first.reachable).toBeCloseTo(0.25);

  const second = await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent('pystral-render-frame', { detail: {
    entities: [], cameras: [], map: null, materials: {}, camera_pose: null,
    waypoint_preview: {
      unit_id: 7, reachable: [], path: [],
      selected_destination: { q: 3, r: -2, layer: 0, world_position: [3, 0, -2] },
    },
    } }));
    return [...window.__pystralThreeNativeWaypointMarkers.keys()];
  });
  expect(second).toEqual(['selected:3:-2:0']);
});
