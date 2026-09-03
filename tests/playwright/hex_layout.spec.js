const { test, expect } = require('@playwright/test');

test('pointy-top axial neighbors meet the rendered hex footprint exactly', async ({ page }) => {
  await page.goto('/game.html');
  const distances = await page.evaluate(async () => {
    const { hexCenter } = await import('/web/renderer.js');
    const center = hexCenter(0, 0, true, 1, 1);
    return [[1, 0], [0, 1], [1, -1]].map(([q, r]) => {
      const [x, z] = hexCenter(q, r, true, 1, 1);
      return Math.hypot(x - center[0], z - center[1]);
    });
  });
  expect(distances[0]).toBeCloseTo(Math.sqrt(3));
  expect(distances[1]).toBeCloseTo(Math.sqrt(3));
  expect(distances[2]).toBeCloseTo(Math.sqrt(3));
});

test('pointy-top hex footprint has no intentional overlap or gap', async ({ page }) => {
  await page.goto('/game.html');
  const radius = 1;
  const result = await page.evaluate(async () => {
    const { hexCenter } = await import('/web/renderer.js');
    const radius = 1;
    const center = hexCenter(0, 0, true, radius, radius);
    const neighbors = [[1, 0], [0, 1], [1, -1], [-1, 0], [0, -1], [-1, 1]];
    return neighbors.map(([q, r]) => {
      const [x, z] = hexCenter(q, r, true, radius, radius);
      return Math.hypot(x - center[0], z - center[1]);
    });
  });
  // A unit pointy hex has circumradius 1 and apothem sqrt(3)/2;
  // adjacent centers are sqrt(3) apart, so shared edges touch exactly.
  for (const distance of result) {
    const edgeSeparation = distance - Math.sqrt(3) * radius;
    expect(edgeSeparation).toBeCloseTo(0);
  }
});

test('pointy-top renderer uses the zero-angle vertex convention', async ({ page }) => {
  await page.goto('/game.html');
  const source = await page.evaluate(async () => (await fetch('/web/renderer.js')).text());
  expect(source).toContain('pointy ? 0 : Math.PI / 6');
});
