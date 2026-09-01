import { chromium } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

// Clean-room extraction of factual GLB skin metadata. This tool does not
// import Spracker or copy its implementation; it emits only a compact neutral
// manifest consumed by optional presentation IK/ragdoll overlays.
const JOB_DIR = path.join(process.cwd(), 'assets/gltf/jobs');
const OUTPUT_FILE = path.join(process.cwd(), 'web/sprite_actor_manifest.json');
const SPRACKER_URL = 'http://localhost:5173/';
const SLICE_COUNT = 300;

async function main() {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.route('**/local-models/jobs/**', async (route) => {
    const file = path.basename(new URL(route.request().url()).pathname);
    const filePath = path.join(JOB_DIR, file);
    if (!fs.existsSync(filePath)) return route.abort();
    return route.fulfill({ body: fs.readFileSync(filePath), contentType: 'model/gltf-binary' });
  });
  await page.goto(SPRACKER_URL);
  await page.waitForFunction(() => (window as any).loadGltf !== undefined);

  const actors = await page.evaluate(async ({ files, sliceCount }) => {
    const loadGltf = (window as any).loadGltf;
    const output: Record<string, unknown> = {};
    for (const file of files) {
      const gltf = await loadGltf(`/local-models/jobs/${file}`);
      const bones: any[] = [];
      const meshes: any[] = [];
      gltf.scene.updateMatrixWorld(true);
      gltf.scene.traverse((object: any) => {
        if (object.isBone) bones.push(object);
        if (object.isSkinnedMesh) meshes.push(object);
      });
      const boneIndex = new Map(bones.map((bone, index) => [bone, index]));
      const slices = Array.from({ length: sliceCount }, () => ({ weights: new Map<number, number>(), vertices: 0 }));
      for (const mesh of meshes) {
        const position = mesh.geometry.getAttribute('position');
        const indices = mesh.geometry.getAttribute('skinIndex');
        const weights = mesh.geometry.getAttribute('skinWeight');
        if (!position || !indices || !weights) continue;
        let minY = Infinity; let maxY = -Infinity;
        for (let i = 0; i < position.count; i++) {
          minY = Math.min(minY, position.getY(i)); maxY = Math.max(maxY, position.getY(i));
        }
        const span = Math.max(1e-6, maxY - minY);
        for (let i = 0; i < position.count; i++) {
          const slice = Math.max(0, Math.min(sliceCount - 1,
            Math.floor(((position.getY(i) - minY) / span) * sliceCount)));
          const record = slices[slice]; record.vertices++;
          for (let influence = 0; influence < 4; influence++) {
            const index = indices.getComponent(i, influence);
            const weight = weights.getComponent(i, influence);
            if (weight > 0) record.weights.set(index, (record.weights.get(index) || 0) + weight);
          }
        }
      }
      output[file.replace(/\.glb$/, '')] = {
        version: 1,
        slice_count: sliceCount,
        bones: bones.map((bone, index) => ({
          index, name: bone.name, parent: bone.parent?.name || null,
          rest_position: bone.position.toArray(), rest_quaternion: bone.quaternion.toArray(),
          rest_scale: bone.scale.toArray(),
        })),
        slice_influences: slices.map((slice, index) => ({
          slice: index,
          vertices: slice.vertices,
          bones: [...slice.weights.entries()].sort((a, b) => b[1] - a[1]).slice(0, 2)
            .map(([bone, weight]) => ({ bone, weight })),
        })),
      };
    }
    return output;
  }, { files: fs.readdirSync(JOB_DIR).filter((file) => file.endsWith('.glb')).sort(), sliceCount: SLICE_COUNT });
  fs.writeFileSync(OUTPUT_FILE, `${JSON.stringify({ version: 1, actors }, null, 2)}\n`);
  await browser.close();
}

main().catch((error) => { console.error(error); process.exitCode = 1; });
