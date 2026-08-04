const assert = require('assert');
const { execFileSync } = require('child_process');
const test = require('node:test');

test('temporal bake preflight validates the exact manifest and reports cost', () => {
  const output = execFileSync(process.execPath, [
    '--experimental-strip-types',
    'scripts/temporal_bake_plan.ts',
  ], { encoding: 'utf8' });
  const plan = JSON.parse(output);

  assert.strictEqual(plan.target_count, 43);
  assert.strictEqual(plan.layer_count, 300);
  assert.strictEqual(plan.requests.length, 2);
  assert.strictEqual(plan.requests[0].target_count, 43);
  assert.strictEqual(plan.requests[1].target_count, 5);
  assert.ok(plan.total_temporal_frames > 0);
  assert.strictEqual(plan.total_layers, plan.total_temporal_frames * plan.layer_count);
  assert.ok(plan.raw_rgba_bytes > 0);
});
