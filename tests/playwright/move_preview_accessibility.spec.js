const { test, expect } = require('@playwright/test');

async function waitForHistoryToSettle(page) {
  await page.waitForFunction(() => {
    const slider = document.getElementById('history-slider');
    const menu = document.getElementById('action-menu');
    const controls = document.getElementById('action-controls');
    return slider
      && Number(slider.value) === Number(slider.max)
      && menu?.style.display === 'block'
      && controls?.style.display !== 'none'
      && menu.dataset.actionPending !== 'true';
  }, { timeout: 40000 });

}

test('move preview exposes accessible status and returns to the top-level menu', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  const confirm = page.getByRole('button', { name: /action/i });
  const back = page.getByRole('button', { name: 'Return' });

  await expect(menu).toBeVisible({ timeout: 40000 });
  await expect(page.getByRole('heading', { name: /unit \d+ action menu/i })).toBeVisible();
  await expect(status).toContainText('Focus a job and press Enter to open its abilities.');

  await confirm.click({ force: true });
  await expect(status).toContainText(/Move preview: selected/);
  await expect(status).toContainText(/reachable destinations/);

  await back.click({ force: true });
  await expect(status).toContainText('Focus a job and press Enter to open its abilities.');
});

test('ability descriptors open legal targets and restore focus through the menu path', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  await expect(menu).toBeVisible({ timeout: 40000 });

  const primaryJob = page.locator('[data-menu-key="job:primary"]');
  await primaryJob.click();
  const ability = page.locator('[data-menu-key^="ability:"]').first();
  await expect(ability).toBeFocused();

  await ability.click();
  await expect(status).toContainText(/Ability target:/);
  await expect(page.getByRole('button', { name: /Unit \d+ at/ }).first()).toBeFocused();

  await page.getByRole('button', { name: 'Return' }).click({ force: true });
  await expect(ability).toBeFocused();
});

test('committed abilities report target count and restore the originating ability focus', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  await expect(menu).toBeVisible({ timeout: 40000 });

  await page.locator('[data-menu-key="job:primary"]').click();
  const ability = page.locator('[data-menu-key^="ability:"]').first();
  await ability.click();
  const target = page.locator('[data-menu-key^="target:"]').first();
  await expect(target).toBeVisible();
  await target.click();
  await page.getByRole('button', { name: 'Action' }).click({ force: true });

  await expect(menu).toHaveAttribute('data-animation-pending', 'true', { timeout: 5000 });
  await expect(status).toContainText(/ability \(\d+ target(s)?\) committed/i);
  await waitForHistoryToSettle(page);
  await expect(menu).toHaveAttribute('data-animation-pending', 'false');
  await expect(ability).toBeFocused();
});

test('cell-area abilities expose cell centers and report affected targets', async ({ page }) => {
  // This intentionally exercises a larger simulation batch than the scalar
  // ability case, but must still finish in seconds rather than heartbeat
  // timeouts measured in minutes.
  test.setTimeout(45000);
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  await expect(menu).toBeVisible({ timeout: 40000 });

  // Primal Roar belongs to the Caveman. Wait through the scheduler until the
  // deterministic player unit that owns that ability is active.
  let reachedCaveman = false;
  for (let attempts = 0; attempts < 16; attempts += 1) {
    const heading = await page.getByRole('heading', { name: /unit \d+ action menu/i }).innerText();
    if (heading === 'Unit 1 action menu') {
      reachedCaveman = true;
      break;
    }
    await page.getByRole('button', { name: 'Wait' }).click();
    await waitForHistoryToSettle(page);
  }
  expect(reachedCaveman, 'bounded Wait loop never reached the Caveman boundary').toBe(true);
  await page.waitForFunction(() => {
    const heading = document.querySelector('#action-menu h2');
    if (heading?.textContent !== 'Unit 1 action menu') {
      window.__cavemanMenuStableCount = 0;
      return false;
    }
    window.__cavemanMenuStableCount = (window.__cavemanMenuStableCount || 0) + 1;
    return window.__cavemanMenuStableCount >= 3;
  }, { timeout: 5000, polling: 100 });

  await page.locator('[data-menu-key="job:primary"]').click();
  const areaAbility = page.getByRole('button', { name: /Primal Roar/ });
  await areaAbility.click();
  const cellTarget = page.getByRole('button', { name: /Cell q/ }).first();
  await expect(cellTarget).toBeVisible();
  await cellTarget.click();
  await page.getByRole('button', { name: 'Action' }).click({ force: true });

  await expect(menu).toHaveAttribute('data-animation-pending', 'true', { timeout: 5000 });
  await expect(status).toContainText(/ability \(\d+ target(s)?\) committed/i);
  await waitForHistoryToSettle(page);
  await expect(menu).toHaveAttribute('data-animation-pending', 'false');
  await expect(areaAbility).toBeFocused();
});

test('move preview exposes gameplay layer navigation separately from camera navigation', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  await expect(page.getByRole('region', { name: /unit \d+ action menu/i })).toBeVisible({ timeout: 40000 });
  await page.getByRole('button', { name: /action/i }).click({ force: true });

  await expect(page.locator('#action-layer-up')).toBeVisible();
  await expect(page.locator('#action-layer-down')).toBeVisible();
  await expect(page.locator('#nav-up')).toHaveAttribute('title', 'Camera Up');
  await expect(page.locator('#nav-down')).toHaveAttribute('title', 'Camera Down');
});

test('stale preview rejection refreshes to the source cell', async ({ page }) => {
  await page.goto('/?test=1');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const status = page.getByRole('status');
  const confirm = page.getByRole('button', { name: /action/i });
  const occupy = page.getByRole('button', { name: 'Occupy selected destination' });

  await expect(page.getByRole('region', { name: /unit \d+ action menu/i })).toBeVisible({ timeout: 40000 });
  const activeHeading = await page.getByRole('heading', { name: /unit \d+ action menu/i }).innerText();
  const expectedSource = activeHeading.includes('Unit 2')
    ? 'selected q 1, r -1, layer 0'
    : 'selected q 0, r 0, layer 0';
  await confirm.click({ force: true });
  await expect(status).toContainText(/Move preview: selected/);
  await expect(occupy).toBeVisible();

  await occupy.click({ force: true });
  await confirm.click({ force: true });
  await expect(status).toContainText(/Move rejected:/, { timeout: 10000 });
  await expect(status).toContainText(expectedSource);
});

test('committed movement waits for its animation barrier', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  const action = page.getByRole('button', { name: /action/i });

  await expect(menu).toBeVisible({ timeout: 40000 });
  await action.click({ force: true });
  await expect(status).toContainText(/Move preview: selected/);
  await action.click({ force: true });

  await expect(menu).toHaveAttribute('data-animation-pending', 'true', { timeout: 5000 });
  await expect(status).toContainText(/Move committed/);
  await waitForHistoryToSettle(page);
  await expect(menu).toHaveAttribute('data-animation-pending', 'false');
  await expect(status).toContainText(/Move preview: selected/);
});

test('Wait ends the player turn through the action protocol', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  const wait = page.getByRole('button', { name: 'Wait' });

  await expect(menu).toBeVisible({ timeout: 40000 });
  await waitForHistoryToSettle(page);
  await expect(wait).toBeVisible();
  await wait.click();

  await expect(page.locator('#log-container')).toContainText('Action input: wait', { timeout: 10000 });
  await expect(menu).toBeVisible({ timeout: 10000 });
  await waitForHistoryToSettle(page);
  await expect(status).toContainText(/Focus a job and press Enter to open its abilities\./);
});

test('Wait hands a human-controlled unit back through the same boundary', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const wait = page.getByRole('button', { name: 'Wait' });
  await expect(menu).toBeVisible({ timeout: 40000 });
  await waitForHistoryToSettle(page);

  await wait.click();
  await waitForHistoryToSettle(page);
  const secondHeading = await page.getByRole('heading', { name: /unit \d+ action menu/i }).innerText();

  expect(secondHeading).toMatch(/Unit [12] action menu/);
  await expect(wait).toBeVisible();
});

test('consecutive Wait actions continue through repeated player turns', async ({ page }) => {
  test.setTimeout(60000);
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const unitStatePanel = page.locator('#unit-state-panel');
  const wait = page.getByRole('button', { name: 'Wait' });
  const slider = page.locator('#history-slider');

  await expect(menu).toBeVisible({ timeout: 40000 });
  await expect(unitStatePanel).toBeVisible({ timeout: 10000 });
  await expect(unitStatePanel.locator('summary')).not.toHaveCount(0);
  for (let turn = 0; turn < 4; turn += 1) {
    await waitForHistoryToSettle(page);
    const before = Number(await slider.inputValue());
    await expect(wait).toBeVisible();
    await wait.click();
    await waitForHistoryToSettle(page);
    const after = Number(await slider.inputValue());
    expect(after).toBeGreaterThan(before);
  }
});

test('settled Wait leaves the UI log free of errors', async ({ page }) => {
  await page.goto('/');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  await waitForHistoryToSettle(page);
  await page.getByRole('button', { name: 'Wait' }).click();
  await expect(page.locator('#log-container')).toContainText('Action input: wait', {
    timeout: 10000,
  });

  await waitForHistoryToSettle(page);
  await expect(page.locator('#log-container')).toContainText('Action input: wait');
  await expect(page.locator('#error-display')).toHaveText(/^(Error: 0|Info: \d+)$/);
  await expect(page.locator('#log-container')).not.toContainText('Runtime Error');
  await expect(page.locator('#log-container')).not.toContainText('Unexpected');
});
