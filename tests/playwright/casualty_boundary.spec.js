const { test, expect } = require('@playwright/test');

async function waitForPlayerBoundary(page) {
  await page.waitForFunction(() => {
    const slider = document.getElementById('history-slider');
    const menu = document.getElementById('action-menu');
    const status = window.__pystralWorkerStatus || '';
    return slider
      && Number(slider.value) === Number(slider.max)
      && menu?.style.display === 'block'
      && menu.dataset.actionPending !== 'true'
      && menu.dataset.animationPending !== 'true'
      && menu.dataset.waitPending !== 'true'
      && status.includes('AwaitingPlayerDecision')
      && status.includes('simulation request None');
  }, { timeout: 15000 });
}

async function sendAccepted(page, input) {
  await page.evaluate((actionInput) => new Promise((resolve, reject) => {
    const before = window.__pystralDebugTraces.filter(
      (trace) => trace === `unified worker accepted action input ${actionInput}`,
    ).length;
    const check = () => {
      const accepted = window.__pystralDebugTraces.filter(
        (trace) => trace === `unified worker accepted action input ${actionInput}`,
      ).length;
      if (accepted > before) {
        cleanup();
        resolve();
      }
    };
    const cleanup = () => {
      window.removeEventListener('pystral-debug-trace', check);
      clearInterval(poll);
      clearTimeout(timer);
    };
    const poll = setInterval(check, 50);
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`input was not accepted: ${actionInput}`));
    }, 5000);
    window.addEventListener('pystral-debug-trace', check);
    window.app.action_nav(actionInput);
    check();
  }), input);
}

async function castFireball(page) {
  const menu = page.locator('#action-menu');
  const ability = page.locator('[data-menu-key="ability:9"]');
  if (!(await ability.isVisible().catch(() => false))) {
    const heading = await page.locator('#action-menu-heading').innerText();
    await sendAccepted(
      page,
      heading === 'Unit 1 action menu' ? 'menu-job:secondary:0' : 'menu-job:primary',
    );
    await waitForPlayerBoundary(page);
  }
  await expect(ability).toBeVisible({ timeout: 5000 });
  await sendAccepted(page, 'menu-ability:9');
  const target = page.locator('[data-menu-key^="target:"]').first();
  await page.waitForFunction(() => {
    const menu = document.getElementById('action-menu');
    return menu?.querySelector('[data-menu-key^="target:"]')
      || menu?.textContent.includes('Insufficient action points')
      || menu?.textContent.includes('No legal targets');
  }, { timeout: 5000 });
  if (!(await target.isVisible().catch(() => false))) {
    await sendAccepted(page, 'return');
    return false;
  }
  await sendAccepted(page, `menu-target:${(await target.getAttribute('data-menu-key')).split(':')[1]}`);
  await sendAccepted(page, 'confirm');
  await expect(menu).toHaveAttribute('data-animation-pending', 'true', { timeout: 5000 });
  return true;
}

async function loadScenario(page, scenario) {
  await page.route('**/web/scripts/pg_rpg.rhai', async (route) => {
    const response = await route.fetch();
    const source = await response.text();
    route.fulfill({
      response,
      body: source.replace(
        '@include "scenarios/skirmish.rhai"',
        `@include "scenarios/${scenario}.rhai"`,
      ),
    });
  });
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });
}

async function expectCompletion(page, outcome) {
  try {
    await expect(page.locator('#game-completed'))
      .toHaveAttribute('data-outcome', outcome, { timeout: 15000 });
  } catch (error) {
    const diagnostic = await page.evaluate(() => ({
      transient: window.__pystralLastTransientState,
      rendered: document.getElementById('game-completed')?.outerHTML,
    }));
    throw new Error(`${error.message}\noutcome diagnostic: ${JSON.stringify(diagnostic)}`);
  }
  await expect(page.locator('#game-completed')).toContainText(outcome);
  await expect.poll(
    () => page.evaluate(() => window.__pystralGameCompletedResponseCount),
  ).toBe(1);

  const before = await page.evaluate(() => ({
    transient: JSON.stringify(window.__pystralLastTransientState),
    requests: window.__pystralDebugTraces.filter(
      (trace) => trace.startsWith('simulation bridge send request'),
    ).length,
  }));
  await page.evaluate(() => window.app.action_nav('confirm'));
  await expect.poll(
    () => page.evaluate(() => window.__pystralDebugTraces.some(
      (trace) => trace === 'ignored action input after completion confirm',
    )),
  ).toBe(true);
  const after = await page.evaluate(() => ({
    transient: JSON.stringify(window.__pystralLastTransientState),
    requests: window.__pystralDebugTraces.filter(
      (trace) => trace.startsWith('simulation bridge send request'),
    ).length,
    completions: window.__pystralGameCompletedResponseCount,
  }));
  expect(after.transient).toBe(before.transient);
  expect(after.requests).toBe(before.requests);
  expect(after.completions).toBe(1);
}

test('pg_rpg casualty boundary skips dead units and reaches victory', async ({ page }) => {
  test.setTimeout(90000);
  await loadScenario(page, 'casualty');
  await waitForPlayerBoundary(page);

  expect(await castFireball(page)).toBe(true);
  await page.waitForFunction(() => {
    const text = document.getElementById('unit-state-items')?.textContent || '';
    return /Unit [34] · team 2 · HP 0/.test(text);
  }, { timeout: 15000 });
  await expect(page.locator('#action-menu-heading')).not.toHaveText(/Unit [34] action menu/);
  try {
    await expectCompletion(page, 'Victory');
  } catch (error) {
    const diagnostics = await page.evaluate(() => ({
      status: window.__pystralWorkerStatus,
      reason: document.getElementById('heartbeat-reason')?.textContent,
      traces: window.__pystralDebugTraces,
    }));
    throw new Error(`${error.message}\ncasualty diagnostics: ${JSON.stringify(diagnostics)}`);
  }
});

test('pg_rpg reports Defeat when the player team starts dead', async ({ page }) => {
  test.setTimeout(30000);
  await loadScenario(page, 'defeat');
  await expectCompletion(page, 'Defeat');
});

test('pg_rpg reports Draw when both teams start dead', async ({ page }) => {
  test.setTimeout(30000);
  await loadScenario(page, 'draw');
  await expectCompletion(page, 'Draw');
});
