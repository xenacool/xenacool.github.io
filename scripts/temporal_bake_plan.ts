import * as fs from 'fs';
import * as path from 'path';

type Catalog = {
  rigs: Record<string, { clips: Array<{ name: string; duration_ms: number }> }>;
  targets: Record<string, { rig: string; compatible: boolean }>;
};
type Manifest = {
  sampling: { mode: string; fps: number; include_terminal_frame: boolean };
  targets: string | Record<string, string | string[]>;
  clips: Record<string, { loop: boolean; rigs: Record<string, string> }>;
};

const root = process.cwd();
const MAX_RAW_MIB = Number(process.env.PYSTRAL_MAX_ANIMATION_RAW_MIB || 512);
const ENFORCE_BUDGET = process.env.PYSTRAL_ENFORCE_ANIMATION_BUDGET === '1';
const catalog = JSON.parse(fs.readFileSync(path.join(root, 'web/animation_catalog.json'), 'utf8')) as Catalog;
const manifest = JSON.parse(
  fs.readFileSync(path.join(root, 'assets/animation_bake_manifest.json'), 'utf8'),
) as Manifest;

function main() {
  if (manifest.sampling.mode !== 'fixed_fps' || manifest.sampling.fps <= 0) {
    throw new Error('temporal bake requires a positive fixed_fps sampling policy');
  }
  const compatibleTargets = Object.entries(catalog.targets).filter(([, target]) => target.compatible);
  const targetsFor = (requestName: string) => {
    const selection = typeof manifest.targets === 'string'
      ? manifest.targets
      : manifest.targets[requestName];
    if (selection === 'all-compatible') return compatibleTargets;
    if (Array.isArray(selection)) {
      return selection.map((name) => {
        const target = catalog.targets[name];
        if (!target?.compatible) throw new Error(`${requestName}: invalid target ${name}`);
        return [name, target] as const;
      });
    }
    throw new Error(`${requestName}: missing target selection`);
  };
  const requests = Object.entries(manifest.clips).map(([name, request]) => {
    const byRig = Object.entries(request.rigs).map(([rig, clipName]) => {
      const clip = catalog.rigs[rig]?.clips.find((candidate) => candidate.name === clipName);
      if (!clip) throw new Error(`${name}: ${clipName} is missing from ${rig} catalog`);
      const frames = Math.max(1, Math.ceil((clip.duration_ms / 1000) * manifest.sampling.fps));
      return { rig, clip: clipName, duration_ms: clip.duration_ms, frames };
    });
    return {
      name,
      loop: request.loop,
      target_count: targetsFor(name).length,
      targets_by_rig: Object.fromEntries(byRig.map((rig) => [
        rig.rig,
        targetsFor(name).filter(([, target]) => target.rig === rig.rig).length,
      ])),
      rigs: byRig,
    };
  });
  const totalFrames = requests.reduce((sum, request) => sum + request.rigs.reduce(
    (requestSum, rig) => requestSum + rig.frames * request.targets_by_rig[rig.rig],
    0,
  ), 0);
  const layerCount = 300;
  const pixelsPerLayer = 32 * 32;
  const rawBytes = totalFrames * layerCount * pixelsPerLayer * 4;
  const rawMib = rawBytes / (1024 * 1024);
  if (ENFORCE_BUDGET && rawMib > MAX_RAW_MIB) {
    throw new Error(`animation bake budget exceeded: ${rawMib.toFixed(2)} MiB > ${MAX_RAW_MIB} MiB`);
  }
  console.log(JSON.stringify({
    version: 1,
    target_count: compatibleTargets.length,
    layer_count: layerCount,
    requests,
    total_temporal_frames: totalFrames,
    total_layers: totalFrames * layerCount,
    raw_rgba_bytes: rawBytes,
    raw_rgba_mib: Number(rawMib.toFixed(2)),
    budget: { max_raw_mib: MAX_RAW_MIB, enforced: ENFORCE_BUDGET, within: rawMib <= MAX_RAW_MIB },
    note: 'PNG compression and atlas packing must be measured during the first generated batch.',
  }, null, 2));
}

main();
