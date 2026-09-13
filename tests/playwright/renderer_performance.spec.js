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
    await page.waitForFunction(() => window.__pystralThreeGlbProfile?.().ready, { timeout: 15000 });
    await page.waitForFunction(() => window.__pystralRenderWorkerProfile?.samples > 0, { timeout: 15000 });
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
    expect(result.profile.nativeGlbReady).toBe(true);
    expect(result.profile.nativeGlbModels).toBeGreaterThan(0);
    const renderWorkerProfile = await page.evaluate(() => window.__pystralRenderWorkerProfile);
    expect(renderWorkerProfile.samples).toBeGreaterThan(0);
    expect(renderWorkerProfile.average_tick_ms).toBeGreaterThanOrEqual(0);
    expect(renderWorkerProfile.p95_tick_ms).toBeGreaterThanOrEqual(0);
    expect(renderWorkerProfile.phases_ms).toEqual(expect.objectContaining({
      commands: expect.any(Number),
      playback_history: expect.any(Number),
      state_logic: expect.any(Number),
      frame_build_publish: expect.any(Number),
      hud_and_ack: expect.any(Number),
      debug_panels: expect.any(Number),
    }));
    const ingressProfile = await page.evaluate(() => window.__pystralRenderIngressProfile);
    expect(ingressProfile.received).toBeGreaterThan(0);
    expect(ingressProfile.overwritten).toBeGreaterThanOrEqual(0);
    const glbManifest = await page.evaluate(() => window.__pystralThreeGlbManifest);
    expect(glbManifest.models.Mage.url).toContain('/assets/models/jobs/Mage.glb');
  });

  test('starts the native scene path from the runtime frame contract', async ({ page }) => {
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await expect(page.locator('body')).toHaveAttribute('data-renderer', 'native');
    await page.waitForFunction(() => window.__pystralThreeGlbProfile?.().ready, { timeout: 15000 });
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
            asset: 'Mage',
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
          entities: [{ id: 42, asset: 'Mage',
            world_position: [0, 0, 0], camera_offset: [1, 2, 3], scale: 1,
            animation_state: 'attack', animation_time_ms: 125, animation_frame: 2,
            render_order: 7, rotation_y: 0.25, rotation_z: 0.5 }],
        },
      }));
      const nativeProfile = window.__pystralThreeNativeProfile;
      const mesh = window.__pystralThreeNativeMeshes.get('42').group;
      window.dispatchEvent(new CustomEvent('pystral-render-frame', {
        detail: window.__pystralThreeFrame,
      }));
      const e = window.__pystralThreeNativeCamera.matrixWorld.elements;
      const [qx, qy, qz, qw] = mesh.quaternion.toArray();
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
          depth: mesh.scale.z,
          animation: mesh.userData,
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
    expect(nativeProfile.nativeMeshCount).toBe(1);
    const nativeSlice = nativeState.nativeSlice;
    expect(nativeSlice.position[0]).toBeCloseTo(nativeSlice.expected[0]);
    expect(nativeSlice.position[1]).toBeCloseTo(nativeSlice.expected[1]);
    expect(nativeSlice.position[2]).toBeCloseTo(nativeSlice.expected[2]);
    expect(nativeSlice.renderOrder).toBe(7000);
    expect(Math.abs(nativeSlice.width)).toBeGreaterThan(0);
    expect(nativeSlice.depth).toBeGreaterThan(0);
    expect(nativeSlice.animation).toBeDefined();
    await page.waitForFunction(() => window.__pystralThreeProfile?.().frames > 2, { timeout: 15000 });
    const nativeGpu = await page.evaluate(() => window.__pystralThreeProfile().nativeGpu);
    expect(nativeGpu).toEqual(expect.objectContaining({
      drawCalls: expect.any(Number),
      triangles: expect.any(Number),
      textures: expect.any(Number),
      glErrors: 0,
    }));
  });

  test('preserves zero-height terrain and uses a board-fitting authored camera', async ({ page }) => {
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await page.waitForFunction(() => document.body.dataset.renderer === 'native', { timeout: 15000 });
    await page.waitForFunction(() => {
      const camera = window.__pystralThreeFrame?.cameras?.[0];
      return camera && camera.distance >= 32 && camera.height >= 30;
    }, { timeout: 15000 });

    const result = await page.evaluate(() => {
      const frame = window.__pystralThreeFrame;
      const camera = frame.cameras[0];
      window.dispatchEvent(new CustomEvent('pystral-render-frame', {
        detail: {
          version: 1, tick: 991, entities: [], cameras: [], map: {
            orientation: 'Pointy', hex_size: [1, 1],
            tiles: [{ q: 0, r: 0, layer: 0, bottom: 3, height: 0, material: 'grass' }],
          }, materials: { grass: { color: [0.2, 0.7, 0.3] } },
        },
      }));
      const tile = [...window.__pystralThreeNativeTileMeshes.values()][0];
      return {
        camera: { distance: camera.distance, height: camera.height, target: camera.target },
        tileY: tile.position.y,
        tileHeight: tile.scale.y,
      };
    });
    expect(result.camera.distance).toBeGreaterThanOrEqual(32);
    expect(result.camera.height).toBeGreaterThanOrEqual(30);
    expect(result.camera.target).toHaveLength(3);
    expect(result.tileY).toBeCloseTo(3);
    expect(result.tileHeight).toBeCloseTo(0);
  });

  test('normalizes GLB actors to a cell-centered feet-on-ground contract', async ({ page }) => {
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await page.waitForFunction(() => window.__pystralThreeGlbProfile?.().ready, { timeout: 15000 });
    await page.waitForFunction(() => window.__pystralThreeGlbLoadedModels?.has('Mage'), { timeout: 15000 });
    await page.evaluate(() => window.dispatchEvent(new CustomEvent('pystral-render-frame', {
      detail: {
        version: 1, tick: 900, cameras: [], map: {
          orientation: 'Pointy', hex_size: [1, 1], tiles: [
            { q: 0, r: 0, layer: 0, bottom: 0, height: 0.1, material: 'grass' },
          ],
        }, materials: { grass: { color: [0.2, 0.7, 0.3] } },
        entities: [{ id: 900, asset: 'Mage', world_position: [0, 0.1, 0], scale: 0.6 }],
      },
      })));
    await page.waitForFunction(() => window.__pystralThreeNativeActorContracts?.get('900')?.asset === 'Mage', { timeout: 15000 });
    const actor = await page.evaluate(() => window.__pystralThreeNativeActorContracts.get('900'));
    expect(actor.modelHeight).toBeCloseTo(2, 6);
    expect(actor.bounds.min[1]).toBeCloseTo(0, 4);
    expect(actor.bounds.min[0]).toBeCloseTo(-actor.bounds.max[0], 3);
    expect(actor.bounds.min[2]).toBeCloseTo(-actor.bounds.max[2], 3);
  });

  test('does not attach an obsolete GLB load after an entity asset is replaced', async ({ page }) => {
    await page.route('**/web/assets/models/jobs/Mage.glb', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 250));
      await route.continue();
    });
    await page.goto('/game.html');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
    await page.waitForFunction(() => window.__pystralThreeGlbProfile?.().ready, { timeout: 15000 });
    const frame = (asset, tick) => ({ version: 1, tick, cameras: [], map: null, materials: {},
      entities: [{ id: 901, asset, world_position: [0, 0, 0], scale: 0.6 }] });
    const actor = await page.evaluate(async (frames) => {
      for (let attempt = 0; attempt < 80; attempt += 1) {
        for (const detail of frames) {
          window.dispatchEvent(new CustomEvent('pystral-render-frame', { detail }));
        }
        await new Promise((resolve) => setTimeout(resolve, 25));
        const contract = window.__pystralThreeNativeActorContracts?.get('901');
        if (contract?.asset === 'Caveman') return contract;
      }
      return null;
    }, [frame('Mage', 901), frame('Caveman', 902)]);
    expect(actor).not.toBeNull();
    expect(actor.asset).toBe('Caveman');
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

  test('keeps the six opening units at their authoritative distinct cells', async ({ page }) => {
    test.setTimeout(45000);
    await page.goto('/game.html');
    await page.waitForFunction(() => document.body.dataset.renderer === 'native', { timeout: 15000 });
    await page.waitForFunction(() => {
      const frame = window.__pystralThreeFrame;
      const units = frame?.entities?.filter((entity) => entity.id >= 1 && entity.id <= 6);
      return units?.length === 6 && units.every((entity) => Array.isArray(entity.world_position));
    }, { timeout: 30000 });
    await page.waitForFunction(() => {
      const meshes = window.__pystralThreeNativeMeshes;
      return [1, 2, 3, 4, 5, 6].every((id) => meshes?.get(String(id))?.loaded);
    }, { timeout: 30000 });

    const result = await page.evaluate(() => {
      const units = window.__pystralThreeFrame.entities
        .filter((entity) => entity.id >= 1 && entity.id <= 6)
        .map((entity) => {
          const record = window.__pystralThreeNativeMeshes.get(String(entity.id));
          return {
            id: entity.id,
            cell: [entity.q, entity.r, entity.layer],
            world: entity.world_position,
            group: record.group.position.toArray(),
            loaded: record.loaded,
            modelHeight: record.modelHeight || null,
            instanceChildren: record.instance?.children?.length || 0,
          };
        });
      return { units, uniqueCells: new Set(units.map((unit) => unit.cell.join(':'))).size };
    });
    console.log('OPENING_UNIT_POSITIONS', JSON.stringify(result));
    expect(result.units).toHaveLength(6);
    expect(result.uniqueCells).toBeGreaterThan(1);
    for (const unit of result.units) {
      expect(unit.loaded).toBe(true);
      expect(unit.group).toEqual(unit.world);
    }
  });

  test('gives each opening skinned actor an independent skeleton at its rendered position', async ({ page }) => {
    test.setTimeout(45000);
    await page.goto('/game.html');
    await page.waitForFunction(() => document.body.dataset.renderer === 'native', { timeout: 15000 });
    await page.waitForFunction(() => [1, 2, 3, 4, 5, 6].every((id) =>
      window.__pystralThreeNativeMeshes?.get(String(id))?.loaded), { timeout: 30000 });
    await page.waitForTimeout(100);

    const actors = await page.evaluate(() => {
      const camera = window.__pystralThreeNativeCamera;
      const transform = (matrix, [x, y, z, w = 1]) => [
        matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12] * w,
        matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13] * w,
        matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14] * w,
        matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15] * w,
      ];
      return [1, 2, 3, 4, 5, 6].map((id) => {
      const record = window.__pystralThreeNativeMeshes.get(String(id));
      const skinned = [];
      record.instance.traverse((node) => { if (node.isSkinnedMesh) skinned.push(node); });
      const primary = skinned[0];
      const meshWorld = primary?.matrixWorld.elements.slice(12, 15) || null;
      const viewPoint = meshWorld && transform(camera.matrixWorldInverse.elements, meshWorld);
      const clipPoint = viewPoint && transform(camera.projectionMatrix.elements, viewPoint);
      return {
        id,
        groupWorld: record.group.matrixWorld.elements.slice(12, 15),
        effectiveHeight: (record.modelHeight || 0) * record.group.scale.y,
        skinnedMeshCount: skinned.length,
        skeletonId: primary?.skeleton?.uuid || null,
        bonesAreOwnedByActor: primary?.skeleton?.bones.every((bone) =>
          record.instance.getObjectById(bone.id) === bone) || false,
        meshWorld,
        viewport: clipPoint && [clipPoint[0] / clipPoint[3], clipPoint[1] / clipPoint[3]],
      };
      });
    });

    expect(actors.every((actor) => actor.skinnedMeshCount > 0)).toBe(true);
    expect(actors.every((actor) => actor.bonesAreOwnedByActor)).toBe(true);
    expect(new Set(actors.map((actor) => actor.skeletonId)).size).toBe(actors.length);
    expect(new Set(actors.map((actor) => actor.groupWorld.join(':'))).size).toBe(actors.length);
    expect(new Set(actors.map((actor) => actor.meshWorld.join(':'))).size).toBe(actors.length);
    expect(actors.every((actor) => Math.abs(actor.effectiveHeight - 2) < 0.000001)).toBe(true);
    expect(actors.every((actor) => actor.viewport
      && Math.abs(actor.viewport[0]) < 1 && Math.abs(actor.viewport[1]) < 1)).toBe(true);
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
      const phaseFrames = fps[phase].static.frames + fps[phase].rotating.frames;
      expect(phaseFrames, `${phase}/any motion`).toBeGreaterThan(0);
      for (const motion of ['static', 'rotating']) {
        if (fps[phase][motion].frames === 0) continue;
        expect(fps[phase][motion].fps, `${phase}/${motion}`).toBeGreaterThan(0);
        expect(fps[phase][motion].p95FrameIntervalMs, `${phase}/${motion} p95`)
          .toBeGreaterThan(0);
        expect(fps[phase][motion].maxFrameIntervalMs, `${phase}/${motion} max`)
          .toBeGreaterThanOrEqual(fps[phase][motion].p95FrameIntervalMs);
        expect(fps[phase][motion].droppedFrames, `${phase}/${motion} dropped`)
          .toBeGreaterThanOrEqual(0);
      }
    }
    expect(result.profile.frameIntervalsMs.length).toBeGreaterThan(0);
    expect(result.profile.droppedFrames).toBeGreaterThanOrEqual(0);
  });
});
