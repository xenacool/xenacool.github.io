const { test, expect } = require('@playwright/test');
const {
  loadWithFixture,
  protocolClock,
  chooseFacing,
  waitForAnimationBarrier,
  waitForPlayerBoundary,
} = require('../helpers');

test.beforeEach(async ({ page }) => {
  await loadWithFixture(page, 'player_boundary');
});

async function waitForHistoryToSettle(page, afterClock = null) {
  return waitForPlayerBoundary(page, {
    after: afterClock || { output: -1, input: -1, history: -1 },
  });
}

async function completeEndTurn(page) {
  const before = await protocolClock(page);
  await page.getByRole('button', { name: 'End Turn' }).click();
  await chooseFacing(page);
  await waitForHistoryToSettle(page, before);
}

test('move preview exposes accessible status and returns to the top-level menu', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  const confirm = page.getByRole('button', { name: 'Action', exact: true });
  const back = page.getByRole('button', { name: 'Return' });

  await expect(menu).toBeVisible({ timeout: 60000 });
  await expect(page.getByRole('heading', { name: /unit \d+ action menu/i })).toBeVisible();
  await expect(status).toContainText('Focus a job and press Enter to open its abilities.');

  await confirm.click({ force: true });
  await expect(status).toContainText(
    /Move preview: selected q -?\d+, r -?\d+, layer \d+; cost: \d+ AP; \d+ reachable destinations/,
  );

  await back.click({ force: true });
  await expect(status).toContainText('Focus a job and press Enter to open its abilities.');
});

test('ability descriptors open legal targets and restore focus through the menu path', async ({ page }) => {
  test.setTimeout(45000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  await expect(menu).toBeVisible({ timeout: 40000 });

  const primaryJob = page.locator('[data-menu-key="job:primary"]');
  await primaryJob.click();
  const ability = page.locator('[data-menu-key^="ability:"]').filter({ hasText: 'Club Smash' });

  await ability.click({ force: true });
  await expect(status).toContainText(/Ability target:/);
  await expect(page.getByRole('button', { name: /Unit \d+ at/ }).first()).toBeFocused();

  await page.getByRole('button', { name: 'Return' }).click({ force: true });
  await expect(ability).toBeFocused();
});

test('committed abilities report target count and restore the originating ability focus', async ({ page }) => {
  test.setTimeout(45000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  await expect(menu).toBeVisible({ timeout: 40000 });

  await page.locator('[data-menu-key="job:primary"]').click();
  const ability = page.locator('[data-menu-key^="ability:"]').filter({ hasText: 'Club Smash' });
  await ability.click({ force: true });
  const target = page.locator('[data-menu-key^="target:"]').first();
  await expect(target).toBeVisible();
  await target.click();
  const beforeCommit = await protocolClock(page);
  await page.getByRole('button', { name: 'Action', exact: true }).click({ force: true });

  await waitForAnimationBarrier(page);
  await expect(status).toContainText(/ability \(\d+ target(s)?\) committed/i);
  await waitForHistoryToSettle(page, beforeCommit);
  await expect(ability).toBeFocused();
});

test('cell-area abilities expose cell centers and report affected targets', async ({ page }) => {
  // This intentionally exercises a larger simulation batch than the scalar
  // ability case, but must still finish in seconds rather than heartbeat
  // timeouts measured in minutes.
  test.setTimeout(45000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  await expect(menu).toBeVisible({ timeout: 40000 });

  // Primal Roar belongs to the Caveman. Wait through the scheduler until the
  // deterministic player unit that owns that ability is active.
  let reachedCaveman = false;
  for (let attempts = 0; attempts < 16; attempts += 1) {
    const heading = await page.getByRole('heading', { name: /unit \d+ action menu/i }).innerText();
    if (heading.startsWith('Unit 1 action menu')) {
      reachedCaveman = true;
      break;
    }
    await completeEndTurn(page);
  }
  expect(reachedCaveman, 'bounded Wait loop never reached the Caveman boundary').toBe(true);
  await page.waitForFunction(() => {
    const heading = document.querySelector('#action-menu h2');
    if (!heading?.textContent?.startsWith('Unit 1 action menu')) {
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
  // Capture the predecessor before confirmation. The animation barrier may
  // observe the worker's complete response before the boundary helper starts;
  // using a post-response clock would make a successful action look stale.
  const beforeCommit = await protocolClock(page);
  await page.getByRole('button', { name: 'Action', exact: true }).click({ force: true });

  await waitForAnimationBarrier(page);
  await expect(status).toContainText(/ability \(\d+ target(s)?\) committed/i);
  await waitForHistoryToSettle(page, beforeCommit);
  await expect(areaAbility).toBeFocused();
});

test('move preview exposes gameplay layer navigation separately from camera navigation', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  await expect(page.getByRole('region', { name: /unit \d+ action menu/i })).toBeVisible({ timeout: 40000 });
  await page.getByRole('button', { name: 'Action', exact: true }).click({ force: true });

  await expect(page.locator('#action-layer-up')).toBeVisible();
  await expect(page.locator('#action-layer-down')).toBeVisible();
  await expect(page.locator('#nav-up')).toHaveAttribute('title', 'Camera Up');
  await expect(page.locator('#nav-down')).toHaveAttribute('title', 'Camera Down');
});

test('stale preview rejection refreshes to the source cell', async ({ page }) => {
  await page.goto('/game.html?test=1');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const status = page.getByRole('status');
  const confirm = page.getByRole('button', { name: 'Action', exact: true });
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
  const before = await protocolClock(page);
  await confirm.click({ force: true });
  await page.waitForFunction(({ baseline }) => {
    const output = Number(window.__pystralWorkerLatestSeq || 0);
    const input = Number(window.__pystralWorkerLatestInputSeq || 0);
    const status = document.getElementById('action-menu-status')?.textContent || '';
    return output > baseline.output && input > baseline.input && status.includes('Move rejected:');
  }, { baseline: before }, { timeout: 10000 });
  const after = await protocolClock(page);
  expect(after.history).toBe(before.history);
  await expect(status).toContainText(expectedSource);
});

test('committed movement waits for its animation barrier', async ({ page }) => {
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  const action = page.getByRole('button', { name: 'Action', exact: true });

  await expect(menu).toBeVisible({ timeout: 40000 });
  await action.click({ force: true });
  await expect(status).toContainText(/Move preview: selected/);
  const before = await protocolClock(page);
  await action.click({ force: true });

  await waitForAnimationBarrier(page, { timeout: 15000 });
  await expect(status).toContainText(/Move committed/);
  await waitForHistoryToSettle(page, before);
  await expect(status).toContainText(/Move preview: selected/);
});

test('End Turn opens facing after the implicit wait settles', async ({ page }) => {
  test.setTimeout(120000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const status = page.getByRole('status');
  const wait = page.getByRole('button', { name: 'End Turn' });

  await expect(menu).toBeVisible({ timeout: 60000 });
  await waitForHistoryToSettle(page);
  await expect(wait).toBeVisible();
  const before = await protocolClock(page);
  await wait.click();

  await expect(page.locator('#log-container')).toContainText('Action input: end-turn', { timeout: 10000 });
  await expect(page.locator('.action-face')).toHaveCount(6);
  await expect(page.getByRole('button', { name: 'Face N', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'End Turn' })).not.toBeVisible();
  await expect(menu).toBeVisible({ timeout: 10000 });
  await expect(page.getByRole('button', { name: 'Face N', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Face N', exact: true }).click();
  await expect(page.locator('#log-container')).toContainText('Action input: face:north', { timeout: 10000 });
  await waitForHistoryToSettle(page, before);
  await expect(wait).toBeVisible();
});

test('End Turn hands a human-controlled unit back through the same boundary', async ({ page }) => {
  test.setTimeout(120000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const wait = page.getByRole('button', { name: 'End Turn' });
  await expect(menu).toBeVisible({ timeout: 60000 });
  await waitForHistoryToSettle(page);

  const before = await protocolClock(page);
  await wait.click();
  await expect(page.getByRole('button', { name: 'Face S', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Face S', exact: true }).click();
  await waitForHistoryToSettle(page, before);
  const secondHeading = await page.getByRole('heading', { name: /unit \d+ action menu/i }).innerText();

  expect(secondHeading).toMatch(/Unit [12] action menu/);
  await expect(wait).toBeVisible();
});

test('consecutive End Turn actions continue through repeated player turns', async ({ page }) => {
  test.setTimeout(180000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  const menu = page.getByRole('region', { name: /unit \d+ action menu/i });
  const unitStatePanel = page.locator('#unit-state-panel');
  const wait = page.getByRole('button', { name: 'End Turn' });
  await expect(menu).toBeVisible({ timeout: 40000 });
  await expect(unitStatePanel).toBeVisible({ timeout: 10000 });
  await expect(unitStatePanel.locator('summary')).not.toHaveCount(0);
  for (let turn = 0; turn < 2; turn += 1) {
    await waitForHistoryToSettle(page);
    const before = await protocolClock(page);
    await expect(wait).toBeVisible();
    await wait.click();
    await expect(page.getByRole('button', { name: 'Face S', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Face S', exact: true }).click();
    const after = await waitForHistoryToSettle(page, before);
    expect(after.history).toBeGreaterThan(before.history);
  }
});

test('settled End Turn leaves the UI log free of errors', async ({ page }) => {
  test.setTimeout(120000);
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });

  await waitForHistoryToSettle(page);
  const before = await protocolClock(page);
  await page.getByRole('button', { name: 'End Turn' }).click();
  await expect(page.locator('#log-container')).toContainText('Action input: end-turn', {
    timeout: 10000,
  });

  await page.getByRole('button', { name: 'Face S', exact: true }).click();
  await waitForHistoryToSettle(page, before);
  await expect(page.locator('#log-container')).toContainText('Action input: end-turn');
  await expect(page.locator('#error-display')).toHaveText(/^(Error: 0|Info: \d+)$/);
  await expect(page.locator('#log-container')).not.toContainText('Runtime Error');
  await expect(page.locator('#log-container')).not.toContainText('Unexpected');
});
