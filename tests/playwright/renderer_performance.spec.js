const { test, expect } = require('@playwright/test');

test.describe('Three.js compositor performance contract', () => {
  test.use({
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 2,
  });

  test('keeps the backing store aligned and avoids per-frame resize churn', async ({ page }) => {
    test.setTimeout(45000);
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await page.waitForFunction(() => document.body.dataset.renderer === 'native', { timeout: 15000 });
    await page.waitForFunction(() => window.__pystralThreeAtlasProfile?.().ready, { timeout: 15000 });
    await page.waitForTimeout(2000);

    const result = await page.evaluate(() => {
      const canvas = document.getElementById('canvas');
      const profile = window.__pystralThreeProfile?.();
      return {
        css: [canvas.clientWidth, canvas.clientHeight],
        backing: [canvas.width, canvas.height],
        profile,
      };
    });
    console.log('THREE_PROFILE', JSON.stringify(result));

    expect(result.backing).toEqual(result.css);
    expect(result.profile.frames).toBeGreaterThan(20);
    expect(result.profile.targetFps).toBe(60);
    expect(result.profile.resizeCalls).toBeLessThanOrEqual(2);
    expect(result.profile.totalRenderMs / result.profile.frames).toBeLessThan(20);
    expect(result.profile.nativeAtlasReady).toBe(true);
    expect(result.profile.nativeAtlasRegions).toBeGreaterThan(0);
    const atlasRegion = await page.evaluate(() =>
      window.__pystralThreeResolveAtlasRegion?.('FrostGolem', 0));
    expect(atlasRegion).toMatchObject({ width: 32, height: 32 });
    expect(atlasRegion.u0).toBeGreaterThanOrEqual(0);
    expect(atlasRegion.u0).toBeLessThan(1);
  });

  test('starts the native scene path from the runtime frame contract', async ({ page }) => {
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await expect(page.locator('body')).toHaveAttribute('data-renderer', 'native');
    await page.waitForFunction(() => window.__pystralThreeAtlasProfile?.().ready, { timeout: 15000 });
    await page.waitForFunction(() => window.__pystralThreeNativeAtlasTexture, { timeout: 15000 });
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('pystral-render-frame', {
        detail: {
          version: 1,
          tick: 0,
          entities: [],
          cameras: [],
          map: {
            orientation: 'Pointy',
            hex_size: [1, 1],
            tiles: [
              { q: 0, r: 0, layer: 0, bottom: 0, height: 1, material: 'grass' },
              { q: 1, r: 0, layer: 1, bottom: 1, height: 0.5, material: 'grass' },
            ],
          },
          materials: { grass: { color: [0.2, 0.7, 0.3], roughness: 0.8, metalness: 0, emissive: 0 } },
        },
      }));
      window.dispatchEvent(new CustomEvent('pystral-render-frame', {
        detail: {
          version: 1,
          tick: 1,
          entities: [{
            id: 42,
            asset: 'FrostGolem',
            slice_indices: [0, 1, 2],
            world_position: [0, 0, 0],
            camera_offset: [1, 2, 3],
            scale: 1,
            render_order: 7,
            rotation_y: 0.25,
            rotation_z: 0.5,
          }],
          cameras: [],
          map: null,
          materials: {},
        },
      }));
    });
    const nativeState = await page.evaluate(() => {
      // Inspect synchronously after dispatch so a live frame cannot remove the
      // deterministic fixture before its scene contract is checked.
      window.dispatchEvent(new CustomEvent('pystral-render-frame', {
        detail: {
          version: 1, tick: 1, cameras: [], map: null, materials: {},
          entities: [{ id: 42, asset: 'FrostGolem', slice_indices: [0, 1, 2],
            world_position: [0, 0, 0], camera_offset: [1, 2, 3], scale: 1,
            animation_state: 'attack', animation_time_ms: 125, animation_frame: 2,
            render_order: 7, rotation_y: 0.25, rotation_z: 0.5 }],
        },
      }));
      const nativeProfile = window.__pystralThreeNativeProfile;
      const mesh = window.__pystralThreeNativeMeshes.get('42:0');
      const uvVersionBeforeRepeat = mesh.geometry.getAttribute('uv').version;
      window.dispatchEvent(new CustomEvent('pystral-render-frame', {
        detail: window.__pystralThreeFrame,
      }));
      const e = window.__pystralThreeNativeCamera.matrixWorld.elements;
      const [qx, qy, qz, qw] = mesh.quaternion.toArray();
      // Transform the plane's local +Z normal without relying on Three.js
      // being exposed on window. A horizontal Spracker layer has a vertical
      // normal after the explicit X-axis quarter-turn.
      const normal = [
        2 * (qx * qz + qw * qy),
        2 * (qy * qz - qw * qx),
        1 - 2 * (qx * qx + qy * qy),
      ];
      return {
        nativeProfile,
        nativeSlice: {
          position: mesh.position.toArray(),
          expected: [
            e[0] + e[4] * 2 - e[8] * 3,
            e[1] + e[5] * 2 - e[9] * 3,
            e[2] + e[6] * 2 - e[10] * 3,
          ],
          renderOrder: mesh.renderOrder,
          width: mesh.scale.x,
          depth: mesh.scale.y,
          animation: mesh.userData,
          sharedAtlas: mesh.material.map === window.__pystralThreeNativeAtlasTexture,
          uvVersionBeforeRepeat,
          uvVersionAfterRepeat: mesh.geometry.getAttribute('uv').version,
          normal,
        },
      };
    });
    const nativeProfile = nativeState.nativeProfile;
    expect(nativeProfile).toEqual(expect.objectContaining({
        entities: expect.any(Number),
        mapTiles: expect.any(Number),
    }));
    expect(nativeProfile.mapTiles).toBe(2);
    expect(nativeProfile.entities).toBe(1);
    expect(nativeProfile.nativeMeshCount).toBe(3);
    const nativeSlice = nativeState.nativeSlice;
    expect(nativeSlice.position[0]).toBeCloseTo(nativeSlice.expected[0]);
    expect(nativeSlice.position[1]).toBeCloseTo(nativeSlice.expected[1]);
    expect(nativeSlice.position[2]).toBeCloseTo(nativeSlice.expected[2]);
    expect(nativeSlice.renderOrder).toBe(7000);
    expect(Math.abs(nativeSlice.width)).toBeGreaterThan(0);
    expect(nativeSlice.depth).toBeGreaterThan(0);
    expect(nativeSlice.animation).toEqual({
      animationState: 'attack', animationTimeMs: 125, animationFrame: 2,
    });
    await page.waitForFunction(() => window.__pystralThreeProfile?.().frames > 2, { timeout: 15000 });
    const nativeGpu = await page.evaluate(() => window.__pystralThreeProfile().nativeGpu);
    expect(nativeGpu).toEqual(expect.objectContaining({
      drawCalls: expect.any(Number),
      triangles: expect.any(Number),
      textures: expect.any(Number),
      glErrors: 0,
    }));
    expect(nativeSlice.sharedAtlas).toBe(true);
    expect(nativeSlice.uvVersionAfterRepeat).toBe(nativeSlice.uvVersionBeforeRepeat);
    expect(Math.abs(nativeSlice.normal[1])).toBeGreaterThan(0.9);
  });

  test('keeps the central Caveman on its tactical layer without z diagnostics', async ({ page }) => {
    test.setTimeout(45000);
    await page.goto('/game.html');
    await page.waitForFunction(() => document.body.dataset.renderer === 'native', { timeout: 15000 });
    await page.waitForFunction(() => {
      const frame = window.__pystralThreeFrame;
      return frame?.map?.tiles?.length && frame.entities?.some((entity) => entity.id === 1);
    }, { timeout: 20000 });
    await page.waitForTimeout(500);
    const result = await page.evaluate(() => {
      const frame = window.__pystralThreeFrame;
      const caveman = frame.entities.find((entity) => entity.id === 1);
      const sameHex = frame.map.tiles.filter((tile) => tile.q === caveman.q && tile.r === caveman.r);
      const topAtLayer = Math.max(...sameHex.filter((tile) => tile.layer === caveman.layer)
        .map((tile) => tile.bottom + tile.height));
      const topAtAnyLayer = Math.max(...sameHex.map((tile) => tile.bottom + tile.height));
      return {
        caveman,
        topAtLayer,
        topAtAnyLayer,
        errors: document.getElementById('error-display')?.textContent || '',
        log: document.getElementById('log-container')?.textContent || '',
      };
    });
    expect(result.caveman).toMatchObject({ q: 0, r: 0, layer: 0 });
    expect(result.topAtLayer).toBeCloseTo(1.1);
    expect(result.topAtAnyLayer).toBeCloseTo(5);
    expect(result.caveman.world_position[1]).toBeCloseTo(result.topAtLayer);
    expect(result.errors).toMatch(/^(Error: 0|Info: \d+)$/);
    expect(result.log).not.toContain('Property z not found');
  });

  test('reports simulation and acquiescent FPS with camera-motion buckets', async ({ page }) => {
    test.setTimeout(45000);
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await page.waitForFunction(() => document.body.dataset.renderer === 'native', { timeout: 15000 });

    await page.evaluate(async () => {
      const publish = (status, angle) => {
        window.__pystralWorkerStatus = status;
        if (angle !== 0) window.__pystralThreeMarkCameraMotion?.();
        window.dispatchEvent(new CustomEvent('pystral-render-frame', {
          detail: { version: 1, tick: angle * 100, entities: [], cameras: [{ id: 1, angle }], map: null, materials: {} },
        }));
      };
      const phase = async (status, rotating) => {
        for (let index = 0; index < 80; index += 1) {
          publish(status, rotating ? index * 0.1 : 0);
          await new Promise((resolve) => setTimeout(resolve, 10));
        }
      };
      await phase('Simulating', false);
      await phase('Simulating', true);
      await phase('AwaitingPlayerDecision', false);
      await phase('AwaitingPlayerDecision', true);
    });

    const result = await page.evaluate(() => ({
      fps: window.__pystralThreeFpsProfile?.(),
      profile: window.__pystralThreeProfile?.(),
    }));
    const { fps } = result;
    console.log('THREE_FPS_PROFILE', JSON.stringify(fps));
    for (const phase of ['simulation', 'acquiescent']) {
      for (const motion of ['static', 'rotating']) {
        expect(fps[phase][motion].frames, `${phase}/${motion}`).toBeGreaterThan(0);
        expect(fps[phase][motion].fps, `${phase}/${motion}`).toBeGreaterThan(0);
      }
    }
  });
});
