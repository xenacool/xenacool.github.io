import { chromium } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

// One-time export tool. The game consumes the generated JSON only; it does
// not import Spracker, its source, or its runtime dependencies.
const JOB_DIR = path.join(process.cwd(), 'assets/gltf/jobs');
const ANIMATION_DIR = path.join(process.cwd(), 'assets/gltf/animation');
const OUTPUT_FILE = path.join(process.cwd(), 'web/animation_catalog.json');
const SPRACKER_URL = 'http://localhost:5173/';
const RETARGET_BONE_ALIASES = {
  Medium: { handslotl: 'handslot.l', handslotr: 'handslot.r' },
  Large: {},
};
const OPTIONAL_SOURCE_NODES = {
  Medium: ['handslotl', 'handslotr'],
  Large: [],
};

const rigFiles = {
  Medium: fs.readdirSync(path.join(ANIMATION_DIR, 'Rig_Medium'))
    .filter((file) => file.endsWith('.glb') && !file.includes('/'))
    .sort(),
  Large: fs.readdirSync(path.join(ANIMATION_DIR, 'Rig_Large'))
    .filter((file) => file.endsWith('.glb'))
    .sort(),
};

async function main() {
  const browser = await chromium.launch();
  const page = await browser.newPage();

  await page.route('**/local-models/**', async (route) => {
    const pathname = decodeURIComponent(new URL(route.request().url()).pathname);
    const relativePath = pathname.replace(/^\/local-models\//, '');
    const filePath = relativePath.startsWith('animation/')
      ? path.join(ANIMATION_DIR, relativePath.replace(/^animation\//, ''))
      : path.join(JOB_DIR, relativePath.replace(/^jobs\//, ''));
    if (!fs.existsSync(filePath)) {
      await route.abort();
      return;
    }
    await route.fulfill({
      body: fs.readFileSync(filePath),
      contentType: 'model/gltf-binary',
    });
  });

  await page.goto(SPRACKER_URL);
  await page.waitForFunction(() => (window as any).loadGltf !== undefined);

  const catalog = await page.evaluate(async ({
    rigFiles,
    jobFiles,
    retargetBoneAliases,
    optionalSourceNodes,
  }) => {
    const loadGltf = (window as any).loadGltf;
    const load = (relativePath: string) => loadGltf(`/local-models/${relativePath}`);
    const nodeNames = (scene: any) => {
      const names = new Set<string>();
      scene.traverse((object: any) => {
        if (object.name) names.add(object.name);
      });
      return [...names].sort();
    };
    const boneNames = (scene: any) => {
      const names = new Set<string>();
      scene.traverse((object: any) => {
        if (object.isBone && object.name) names.add(object.name);
      });
      return [...names].sort();
    };
    const clipsForRig = async (rig: string, files: string[]) => {
      const clips = [] as any[];
      const sourceNodes = new Set<string>();
      const sourceBones = new Set<string>();
      const animatedNodes = new Set<string>();
      for (const file of files) {
        const gltf = await load(`animation/Rig_${rig}/${file}`);
        nodeNames(gltf.scene).forEach((name) => sourceNodes.add(name));
        boneNames(gltf.scene).forEach((name) => sourceBones.add(name));
        for (const clip of gltf.animations || []) {
          clip.tracks.forEach((track: any) => {
            // GLTF track names may include a node path. Retarget against the
            // terminal node actually animated, not every source-scene node.
            animatedNodes.add(track.name.split('.')[0].split('/').pop());
          });
          clips.push({
            name: clip.name,
            duration_ms: Math.round(clip.duration * 1000),
            track_count: clip.tracks.length,
            source: file,
          });
        }
      }
      return {
        skeleton: `Rig_${rig}`,
        nodes: [...sourceNodes].sort(),
        bones: [...sourceBones].sort(),
        animated_nodes: [...animatedNodes].sort(),
        clips,
      };
    };

    const rigs: any = {};
    for (const [rig, files] of Object.entries(rigFiles)) {
      rigs[rig] = await clipsForRig(rig, files);
    }

    const targets: any = {};
    for (const file of jobFiles) {
      const model = await load(`jobs/${file}`);
      const rig = model.scene.getObjectByName('Rig_Large') ? 'Large' : 'Medium';
      const names = new Set(nodeNames(model.scene));
      const bones = new Set(boneNames(model.scene));
      const aliases = (retargetBoneAliases as any)[rig] || {};
      const optionalNodes = (optionalSourceNodes as any)[rig] || [];
      const missingNodes = rigs[rig].nodes.filter((name: string) => !names.has(name));
      const missingBones = rigs[rig].bones.filter(
        (name: string) => !bones.has(name) && !names.has(aliases[name]),
      );
      const missingAnimatedNodes = rigs[rig].animated_nodes.filter(
        (name: string) => !names.has(name)
          && !names.has(aliases[name])
          && !optionalNodes.includes(name),
      );
      targets[file.replace(/\.glb$/, '')] = {
        rig,
        skeleton: `Rig_${rig}`,
        // Mesh/material nodes legitimately differ between a rig mannequin and
        // a job model. Retargeting requires the animated skeleton bones, not
        // identical render-scene node inventories.
        // Only nodes referenced by animation tracks are required for
        // retargeting. Source-only attachment bones may intentionally be
        // absent from a job model.
        compatible: missingAnimatedNodes.length === 0,
        bone_aliases: aliases,
        omitted_optional_nodes: rigs[rig].animated_nodes
          .filter((name: string) => optionalNodes.includes(name)),
        missing_nodes: missingNodes,
        missing_bones: missingBones,
        missing_animated_nodes: missingAnimatedNodes,
      };
    }
    return { version: 1, rigs, targets };
  }, {
    rigFiles,
    jobFiles: fs.readdirSync(JOB_DIR).filter((file) => file.endsWith('.glb')).sort(),
    retargetBoneAliases: RETARGET_BONE_ALIASES,
    optionalSourceNodes: OPTIONAL_SOURCE_NODES,
  });

  const incompatible = Object.entries(catalog.targets)
    .filter(([, target]: any) => !target.compatible);
  if (incompatible.length > 0) {
    throw new Error(`Found incompatible animation targets: ${JSON.stringify(incompatible)}`);
  }
  fs.writeFileSync(OUTPUT_FILE, `${JSON.stringify(catalog, null, 2)}\n`);
  console.log(`Exported ${Object.values(catalog.rigs)
    .reduce((count: number, rig: any) => count + rig.clips.length, 0)} clips for ${Object.keys(catalog.targets).length} targets.`);
  await browser.close();
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
