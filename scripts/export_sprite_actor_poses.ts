import { chromium } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const ROOT = process.cwd();
const ANIMATION_DIR = path.join(ROOT, 'assets/gltf/animation');
const OUTPUT = path.join(ROOT, 'web/sprite_actor_poses.json');
const FPS = 12;

async function main() {
  const browser = await chromium.launch(); const page = await browser.newPage();
  await page.route('**/local-models/**', async (route) => {
    const rel = decodeURIComponent(new URL(route.request().url()).pathname).replace(/^\/local-models\//, '');
    const file = path.join(ANIMATION_DIR, rel.replace(/^animation\//, ''));
    if (!fs.existsSync(file)) return route.abort();
    return route.fulfill({ body: fs.readFileSync(file), contentType: 'model/gltf-binary' });
  });
  await page.goto('http://localhost:5173/');
  await page.waitForFunction(() => (window as any).loadGltf !== undefined);
  const files = ['Rig_Large', 'Rig_Medium'].flatMap((rig) =>
    fs.readdirSync(path.join(ANIMATION_DIR, rig)).filter((f) => f.endsWith('.glb')).map((file) => ({ rig, file })));
  const poses = await page.evaluate(async ({ files, fps }) => {
    const load = (window as any).loadGltf; const round = (n: number) => Math.round(n * 10000) / 10000; const out: any[] = [];
    for (const entry of files) { const gltf = await load(`/local-models/animation/${entry.rig}/${entry.file}`); const bones: any[] = [];
      gltf.scene.traverse((o: any) => { if (o.isBone) bones.push(o); });
      for (const clip of gltf.animations || []) { const mixer = new (window as any).THREE.AnimationMixer(gltf.scene); mixer.clipAction(clip).play(); const count = Math.max(1, Math.ceil(clip.duration * fps)); const frames = [];
        for (let frame = 0; frame <= count; frame++) { mixer.setTime(Math.min(clip.duration, frame / fps)); frames.push({ frame, bones: bones.map((b) => ({ p: b.position.toArray().map(round), q: b.quaternion.toArray().map(round) })) }); }
        out.push({ rig: entry.rig, clip: clip.name, duration_ms: Math.round(clip.duration * 1000), fps, frames }); }
    } return out;
  }, { files, fps: FPS });
  fs.writeFileSync(OUTPUT, `${JSON.stringify({ version: 1, fps: FPS, poses })}\n`); await browser.close();
  const bytes = fs.statSync(OUTPUT).size; if (bytes > 1_500_000) throw new Error(`sprite actor poses too large: ${bytes}`);
  console.log(`Exported ${poses.length} clips (${bytes} bytes).`);
}
main().catch((error) => { console.error(error); process.exitCode = 1; });
