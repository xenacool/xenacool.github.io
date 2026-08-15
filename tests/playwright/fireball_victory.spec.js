const { test, expect } = require('@playwright/test');

async function waitForSettledPlayerBoundary(page) {
  await page.evaluate(() => new Promise((resolve, reject) => {
    const check = () => {
      const slider = document.getElementById('history-slider');
      const menu = document.getElementById('action-menu');
      const status = window.__pystralWorkerStatus || '';
      const settled = Boolean(slider
        && Number(slider.value) === Number(slider.max)
        && menu?.style.display === 'block'
        && menu.dataset.gameCompleted !== 'true'
        && menu.dataset.actionPending !== 'true'
        && menu.dataset.animationPending !== 'true'
        && menu.dataset.waitPending !== 'true'
        && status.includes('AwaitingPlayerDecision')
        && status.includes('simulation request None')
        && !status.includes('WaitingForAnimationAck')
        && window.__pystralDebugTraces?.some(
          (trace) => trace.includes('unified worker published player transient'),
        ));
      if (settled) finish();
    };
    const finish = () => {
      cleanup();
      resolve();
    };
    const cleanup = () => {
      window.removeEventListener('pystral-heartbeat', check);
      window.removeEventListener('pystral-debug-trace', check);
      window.removeEventListener('pystral-menu-state', check);
      clearInterval(poll);
      clearTimeout(timer);
    };
    const poll = setInterval(check, 50);
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`settled player boundary timeout: ${window.__pystralWorkerStatus}`));
    }, 30000);
    window.addEventListener('pystral-heartbeat', check);
    window.addEventListener('pystral-debug-trace', check);
    window.addEventListener('pystral-menu-state', check);
    check();
  }));
}

async function diagnostics(page) {
  return page.evaluate(() => ({
    status: window.__pystralWorkerStatus,
    reason: document.getElementById('heartbeat-reason').textContent,
    historySlider: {
      value: document.getElementById('history-slider').value,
      max: document.getElementById('history-slider').max,
    },
    settledConditions: (() => {
      const slider = document.getElementById('history-slider');
      const menu = document.getElementById('action-menu');
      const status = window.__pystralWorkerStatus || '';
      return {
        atTail: Number(slider.value) === Number(slider.max),
        menuVisible: menu?.style.display === 'block',
        completed: menu?.dataset.gameCompleted,
        actionPending: menu?.dataset.actionPending,
        animationPending: menu?.dataset.animationPending,
        waitPending: menu?.dataset.waitPending,
        awaitingPlayer: status.includes('AwaitingPlayerDecision'),
        simulationIdle: status.includes('simulation request None'),
        hasTransientTrace: window.__pystralDebugTraces?.some(
          (trace) => trace.includes('unified worker published player transient'),
        ),
      };
    })(),
    menu: document.getElementById('action-menu').outerHTML.slice(0, 1200),
    traces: window.__pystralDebugTraces,
  }));
}

async function sendAccepted(page, input) {
  await page.evaluate((actionInput) => {
    const acceptedBefore = window.__pystralAcceptedActionCounts[actionInput] || 0;
    return new Promise((resolve, reject) => {
      const check = () => {
        if ((window.__pystralAcceptedActionCounts[actionInput] || 0) > acceptedBefore) {
          cleanup();
          resolve();
        }
      };
      const onTrace = (event) => {
        if (event.detail === `unified worker accepted action input ${actionInput}`) check();
      };
      const cleanup = () => {
        window.removeEventListener('pystral-debug-trace', onTrace);
        clearInterval(poll);
        clearTimeout(timer);
      };
      const poll = setInterval(check, 50);
      const timer = setTimeout(() => {
        cleanup();
        reject(new Error(`action input not accepted: ${actionInput}; status=${window.__pystralWorkerStatus}`));
      }, 5000);
      window.addEventListener('pystral-debug-trace', onTrace);
      window.app.action_nav(actionInput);
      check();
    });
  }, input);
}

async function playFireball(page) {
  const menu = page.locator('#action-menu');
  const fireball = page.locator('[data-menu-key="ability:9"]');
  if (!(await fireball.isVisible().catch(() => false))) {
    const heading = await page.locator('#action-menu-heading').innerText();
    const jobInput = heading === 'Unit 1 action menu' ? 'menu-job:secondary:0' : 'menu-job:primary';
    await sendAccepted(page, jobInput);
  }
  // The boundary snapshot and the click are separate worker messages.  If a
  // new simulation slice wins that race, the click is harmlessly rejected;
  // let the caller wait for the next player boundary and retry it.
  await expect(fireball).toBeVisible({ timeout: 5000 });
  await sendAccepted(page, 'menu-ability:9');

  const target = page.locator('[data-menu-key^="target:"]').first();
  await page.evaluate(() => new Promise((resolve, reject) => {
    const check = () => {
      const menu = document.getElementById('action-menu');
      if (menu?.querySelector('[data-menu-key^="target:"]')
        || menu?.textContent.includes('Insufficient action points')
        || menu?.textContent.includes('No legal targets')) {
        cleanup();
        resolve();
      }
    };
    const cleanup = () => {
      window.removeEventListener('pystral-heartbeat', check);
      window.removeEventListener('pystral-debug-trace', check);
      window.removeEventListener('pystral-menu-state', check);
      clearTimeout(timer);
    };
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`ability target response timeout: ${window.__pystralWorkerStatus}`));
    }, 5000);
    window.addEventListener('pystral-heartbeat', check);
    window.addEventListener('pystral-debug-trace', check);
    window.addEventListener('pystral-menu-state', check);
    check();
  }));
  if (!(await target.isVisible().catch(() => false))) {
    // Fireball is intentionally attempted on every player boundary.  A
    // failed attempt is expected after spending AP; return to the top-level
    // menu so the test can end the turn and let AP regenerate.
    await sendAccepted(page, 'return');
    if (!(await page.getByRole('button', { name: 'Wait' }).isVisible().catch(() => false))) {
      await sendAccepted(page, 'return');
    }
    return { played: false, reason: await page.locator('#action-menu-status').innerText() };
  }
  const targetKey = await target.getAttribute('data-menu-key');
  await sendAccepted(page, `menu-target:${targetKey.split(':')[1]}`);
  await sendAccepted(page, 'confirm');
  await expect(menu).toHaveAttribute('data-animation-pending', 'true', { timeout: 5000 });
  return { played: true };
}

test('deterministic pg_rpg Fireball reaches victory after one lethal cast', async ({ page }) => {
  test.setTimeout(30000);
  await page.route('**/web/scripts/pg_rpg.rhai', async (route) => {
    const response = await route.fetch();
    const source = await response.text();
    route.fulfill({
      response,
      body: source.replace(
        '@include "scenarios/skirmish.rhai"',
        '@include "scenarios/casualty.rhai"',
      ),
    });
  });
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
  await waitForSettledPlayerBoundary(page);
  expect(await playFireball(page)).toEqual({ played: true });
  await expect(page.locator('#game-completed')).toHaveAttribute('data-outcome', 'Victory', {
    timeout: 15000,
  });
  await expect(page.locator('#action-menu-status')).toHaveText('Game completed.');
  await expect(page.locator('#game-completed')).toContainText('Victory');
  await expect(page.locator('#action-menu')).toHaveAttribute('data-game-completed', 'true');
  await expect(page.locator('#action-log')).toContainText('Victory');
});
