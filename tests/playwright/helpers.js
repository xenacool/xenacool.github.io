// Shared Playwright helpers for pg_rpg browser tests.

/**
 * Route /game.html so the production scenario is replaced by a test-only
 * fixture. The fixture path is merged into the fetched manifest so its
 * @include graph resolves; production loads never fetch fixtures.
 */
async function loadWithFixture(page, fixture) {
  const fastFixtureMcts = fixture !== 'render_synthetic';
  await page.route('**/web/scripts/pg_rpg.rhai', async (route) => {
    const response = await route.fetch();
    const source = await response.text();
    route.fulfill({
      response,
      body: source.replace(
        '@include "scenarios/skirmish.rhai"',
        `@include "fixtures/${fixture}.rhai"`,
      ),
    });
  });
  if (fastFixtureMcts) {
    await page.route('**/web/scripts/scenarios/skirmish.rhai', async (route) => {
      const response = await route.fetch();
      const source = await response.text();
      route.fulfill({
        response,
        body: source.replace(
          'let sim = new_simulation(scenario, mcts);',
          'mcts.set_visits(6);\nmcts.set_depth(3);\nlet sim = new_simulation(scenario, mcts);',
        ),
      });
    });
  }
  await page.route('**/web/scripts/manifest.json', async (route) => {
    const response = await route.fetch();
    const manifest = JSON.parse(await response.text());
    if (!manifest.files.includes(`fixtures/${fixture}.rhai`)) {
      manifest.files.push(`fixtures/${fixture}.rhai`);
    }
    route.fulfill({ response, body: JSON.stringify(manifest) });
  });
}

async function protocolClock(page) {
  return page.evaluate(() => ({
    output: Number(window.__pystralWorkerLatestSeq || 0),
    input: Number(window.__pystralWorkerLatestInputSeq || 0),
    history: Number(document.getElementById('history-slider')?.max || 0),
  }));
}

async function sendAcceptedAction(page, input, { timeout = 8000 } = {}) {
  await page.evaluate(({ actionInput, actionTimeout }) => new Promise((resolve, reject) => {
    const acceptedBefore = window.__pystralAcceptedActionCounts?.[actionInput] || 0;
    const acceptedTrace = `unified worker accepted action input ${actionInput}`;
    const traceBefore = window.__pystralDebugTraces
      ?.filter((trace) => trace === acceptedTrace).length || 0;
    const accepted = () => (window.__pystralAcceptedActionCounts?.[actionInput] || 0) > acceptedBefore
      || (window.__pystralDebugTraces?.filter((trace) => trace === acceptedTrace).length || 0) > traceBefore;
    const cleanup = () => {
      window.removeEventListener('pystral-debug-trace', check);
      clearInterval(poll);
      clearTimeout(timer);
    };
    const check = () => {
      if (accepted()) {
        cleanup();
        resolve();
      }
    };
    const poll = setInterval(check, 25);
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`action input not accepted: ${actionInput}; status=${window.__pystralWorkerStatus}`));
    }, actionTimeout);
    window.addEventListener('pystral-debug-trace', check);
    window.app.action_nav(actionInput);
    check();
  }), { actionInput: input, actionTimeout: timeout });
}

async function waitForAnimationBarrier(page, { timeout = 15000 } = {}) {
  const menu = page.locator('#action-menu');
  await menu.waitFor({ state: 'visible', timeout });
  await menu.waitFor({ state: 'attached', timeout });
  const baseline = await protocolClock(page);
  await page.waitForFunction(({ baseline }) => {
    const currentMenu = document.getElementById('action-menu');
    const animation = currentMenu?.dataset.animationPending;
    const settled = animation !== 'true'
      && currentMenu?.dataset.actionPending !== 'true'
      && currentMenu?.dataset.waitPending !== 'true';
    if (!settled) return false;

    // Fast actions may complete between two DOM samples, so observing the
    // transient animation=true state is optional. The causal barrier is a
    // newer worker output/history clock, or the terminal completion state.
    const output = Number(window.__pystralWorkerLatestSeq || 0);
    const input = Number(window.__pystralWorkerLatestInputSeq || 0);
    const history = Number(document.getElementById('history-slider')?.max || 0);
    return currentMenu?.dataset.gameCompleted === 'true'
      || output > baseline.output
      || input > baseline.input
      || history > baseline.history;
  }, { baseline }, { timeout });
}

async function chooseFacing(page, direction = 'S', { timeout = 10000 } = {}) {
  const button = page.getByRole('button', { name: `Face ${direction}`, exact: true });
  await button.waitFor({ state: 'visible', timeout });
  await button.click();
}

async function waitForPlayerBoundary(page, { after = null, unitId = null, timeout = 90000 } = {}) {
  const baseline = after || await protocolClock(page);
  try {
    await page.waitForFunction(({ baseline, unitId }) => {
    const slider = document.getElementById('history-slider');
    const menu = document.getElementById('action-menu');
    const state = window.__pystralLastTransientState;
    const output = Number(window.__pystralWorkerLatestSeq || 0);
    const input = Number(window.__pystralWorkerLatestInputSeq || 0);
    const history = Number(slider?.max || 0);
    return slider
      && Number(slider.value) === history
      && output > baseline.output
      && input > baseline.input
      && history > baseline.history
      && menu?.style.display === 'block'
      && menu.dataset.actionPending !== 'true'
      && menu.dataset.animationPending !== 'true'
      && menu.dataset.waitPending !== 'true'
      && state?.input_enabled
      && !state.action_pending
      && !state.wait_pending
      && !state.game_completed
      && (unitId === null || state.active_unit_id === unitId);
    }, { baseline, unitId }, { timeout });
  } catch (error) {
    const observed = await page.evaluate(() => {
      const slider = document.getElementById('history-slider');
      const menu = document.getElementById('action-menu');
      const state = window.__pystralLastTransientState;
      return {
        output: Number(window.__pystralWorkerLatestSeq || 0),
        input: Number(window.__pystralWorkerLatestInputSeq || 0),
        history: Number(slider?.max || 0),
        sliderValue: Number(slider?.value || 0),
        status: window.__pystralWorkerStatus,
        menuDisplay: menu?.style.display,
        menuPending: menu && {
          action: menu.dataset.actionPending,
          animation: menu.dataset.animationPending,
          wait: menu.dataset.waitPending,
        },
        transient: state && {
          activeUnit: state.active_unit_id,
          inputEnabled: state.input_enabled,
          actionPending: state.action_pending,
          waitPending: state.wait_pending,
          gameCompleted: state.game_completed,
        },
      };
    });
    error.message += `; baseline=${JSON.stringify(baseline)} observed=${JSON.stringify(observed)}`;
    throw error;
  }
  return protocolClock(page);
}

async function waitForGameReady(page) {
  await page.waitForFunction(
    () => document.body.dataset.loadingState === 'ready'
      && document.body.dataset.historyReady === 'true'
      && window.app !== undefined,
    null,
    { timeout: 40000 },
  );
}

module.exports = {
  loadWithFixture,
  protocolClock,
  sendAcceptedAction,
  waitForAnimationBarrier,
  chooseFacing,
  waitForGameReady,
  waitForPlayerBoundary,
};
