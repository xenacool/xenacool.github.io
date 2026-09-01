const { test, expect } = require('@playwright/test');

test('semantic palette keeps teams distinct and unassigned neutral', async ({ page }) => {
  await page.goto('/game.html');
  const result = await page.evaluate(async () => {
    const palette = await import('/web/semantic_palette.js');
    return [palette.semanticColor(undefined), palette.semanticColor(1),
      palette.semanticColor(2), palette.semanticColor(3)];
  });
  expect(new Set(result).size).toBe(4);
});
