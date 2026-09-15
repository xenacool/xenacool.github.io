const { test, expect } = require('@playwright/test');

test('Rhai editor runs an isolated authored case and exposes replay metadata', async ({ page }) => {
  await page.goto('/editor.html');
  await expect(page.getByRole('heading', { name: 'Rhai authoring' })).toBeVisible();
  const source = page.getByRole('textbox', { name: 'Rhai source' });
  await expect(source).toHaveValue(/@include/);
  await source.fill(`${await source.inputValue()}\nfn authored_case() { return #{ status: "passed", message: "edited" }; }`);
  await page.getByRole('button', { name: 'Test', exact: true }).click();
  const result = page.locator('#result');
  await expect(result).toContainText('"status": "passed"', { timeout: 15000 });
  await expect(result).toContainText('"entrypoint": "scripts/pg_rpg.rhai"');
  await expect(result).toContainText('"source_fingerprint"');
  await expect(result).toContainText('edited');
});

test('Rhai editor switches virtual files without replacing the active game route', async ({ page }) => {
  await page.goto('/editor.html');
  const source = page.getByRole('textbox', { name: 'Rhai source' });
  await expect(source).toHaveValue(/@include/);
  await page.getByRole('button', { name: 'common.rhai' }).click();
  await expect(source).toHaveValue(/fn sprite_init_properties/);
  await page.getByRole('link', { name: 'Play pg_rpg' }).click();
  await expect(page).toHaveURL(/\/game\.html$/);
  await expect(page.getByRole('combobox', { name: 'Workspace' })).toBeVisible();
  await page.getByRole('combobox', { name: 'Workspace' }).selectOption('builtin:pg_rpg');
  await page.getByRole('button', { name: 'Start selected workspace' }).click();
  await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
});
