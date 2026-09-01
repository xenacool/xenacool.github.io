const { test, expect } = require('@playwright/test');

test('two-bone IK is deterministic and clamps unreachable targets', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => {
    const { solveTwoBoneIK } = await import('/web/two_bone_ik.js');
    return {
      reachable: solveTwoBoneIK([0, 0], [1, 1], 1, 1),
      stretched: solveTwoBoneIK([0, 0], [4, 0], 1, 1),
    };
  });
  expect(result.reachable.clamped).toBe(false);
  expect(result.reachable.end[0]).toBeCloseTo(1);
  expect(result.reachable.end[1]).toBeCloseTo(1);
  expect(result.stretched.clamped).toBe(true);
  expect(Math.hypot(...result.stretched.end)).toBeCloseTo(2);
});
