const fs = require('fs');
const path = require('path');
const { test, expect } = require('@playwright/test');

test('exported animation catalog has retargetable Medium and Large coverage', () => {
  const catalog = JSON.parse(fs.readFileSync(
    path.join(process.cwd(), 'web/animation_catalog.json'),
    'utf8',
  ));

  expect(catalog.version).toBe(1);
  expect(Object.keys(catalog.rigs)).toEqual(expect.arrayContaining(['Medium', 'Large']));
  for (const [rigName, rig] of Object.entries(catalog.rigs)) {
    expect(rig.skeleton).toBe(`Rig_${rigName}`);
    expect(rig.clips.length).toBeGreaterThan(0);
    expect(rig.animated_nodes.length).toBeGreaterThan(0);
    for (const clip of rig.clips) {
      expect(clip.name).not.toBe('');
      expect(clip.duration_ms).toBeGreaterThanOrEqual(0);
      expect(clip.track_count).toBeGreaterThan(0);
    }
  }
  expect(Object.keys(catalog.targets).length).toBeGreaterThan(0);
  for (const target of Object.values(catalog.targets)) {
    expect(target.compatible).toBe(true);
    expect(target.missing_animated_nodes).toEqual([]);
  }
});
