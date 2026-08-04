const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const test = require('node:test');

test('runtime animation states have source-clip coverage on both rigs', () => {
  const script = path.join(process.cwd(), 'scripts/report_animation_mappings.ts');
  const output = execFileSync(process.execPath, ['--experimental-strip-types', script], {
    encoding: 'utf8',
  });
  const report = JSON.parse(output);

  assert.deepStrictEqual(report.missing, []);
  assert.ok(report.states.includes('idle'));
  assert.ok(report.states.includes('attack'));
  assert.ok(report.selection_required.length > 0);
  assert.strictEqual(report.bake_ready, false);
  assert.ok(fs.existsSync(path.join(process.cwd(), 'web/animation_catalog.json')));
});

test('initial temporal bake manifest selects exact clips from the catalog', () => {
  const catalog = JSON.parse(fs.readFileSync(
    path.join(process.cwd(), 'web/animation_catalog.json'),
    'utf8',
  ));
  const manifest = JSON.parse(fs.readFileSync(
    path.join(process.cwd(), 'assets/animation_bake_manifest.json'),
    'utf8',
  ));

  assert.strictEqual(manifest.sampling.mode, 'fixed_fps');
  assert.strictEqual(manifest.sampling.fps, 12);
  assert.strictEqual(manifest.targets.idle, 'all-compatible');
  assert.deepStrictEqual(manifest.targets.attack, [
    'Barbarian', 'Caveman', 'Mage', 'Barbarian_Large', 'FrostGolem',
  ]);
  for (const request of Object.values(manifest.clips)) {
    for (const [rig, clipName] of Object.entries(request.rigs)) {
      assert.ok(catalog.rigs[rig], `unknown rig ${rig}`);
      assert.ok(
        catalog.rigs[rig].clips.some((clip) => clip.name === clipName),
        `${clipName} is not present in ${rig} catalog`,
      );
    }
  }
});
