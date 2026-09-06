const { test, expect } = require('@playwright/test');
const {
  loadWithFixture,
  sendAcceptedAction,
  waitForAnimationBarrier,
  waitForPlayerBoundary,
} = require('../helpers');

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

const sendAccepted = sendAcceptedAction;

async function playFireball(page) {
  const menu = page.locator('#action-menu');
  const fireball = page.locator('[data-menu-key^="ability:"]').filter({ hasText: 'Fireball' });
  if (!(await fireball.isVisible().catch(() => false))) {
    const heading = await page.locator('#action-menu-heading').innerText();
    const jobInput = heading.startsWith('Unit 1 action menu')
      ? 'menu-job:secondary:0'
      : 'menu-job:primary';
    await sendAccepted(page, jobInput);
  }
  // The boundary snapshot and the click are separate worker messages.  If a
  // new simulation slice wins that race, the click is harmlessly rejected;
  // let the caller wait for the next player boundary and retry it.
  await expect(fireball).toBeVisible({ timeout: 5000 });
  const abilityKey = await fireball.getAttribute('data-menu-key');
  await sendAccepted(page, `menu-${abilityKey}`);

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
    if (!(await page.getByRole('button', { name: 'End Turn' }).isVisible().catch(() => false))) {
      await sendAccepted(page, 'return');
    }
    return { played: false, reason: await page.locator('#action-menu-status').innerText() };
  }
  const targetKey = await target.getAttribute('data-menu-key');
  await sendAccepted(page, `menu-target:${targetKey.split(':')[1]}`);
  await sendAccepted(page, 'confirm');
  await waitForAnimationBarrier(page);
  return { played: true };
}

test('deterministic pg_rpg Fireball reaches victory after one lethal cast', async ({ page }) => {
  test.setTimeout(90000);
  await loadWithFixture(page, 'casualty');
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
  await waitForPlayerBoundary(page, {
    after: { output: -1, input: -1, history: -1 },
    unitId: 1,
  });
  expect(await playFireball(page)).toEqual({ played: true });
  await expect(page.locator('#game-completed')).toHaveAttribute('data-outcome', 'Victory', {
    timeout: 15000,
  });
  await expect(page.locator('#action-menu-status')).toHaveText('Game completed.');
  await expect(page.locator('#game-completed')).toContainText('Victory');
  await expect(page.locator('#action-menu')).toHaveAttribute('data-game-completed', 'true');
  await expect(page.locator('#action-log')).toContainText('Victory');
});
