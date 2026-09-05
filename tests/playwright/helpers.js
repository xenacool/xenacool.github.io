// Shared Playwright helpers for pg_rpg browser tests.

/**
 * Route /game.html so the production scenario is replaced by a test-only
 * fixture. The fixture path is merged into the fetched manifest so its
 * @include graph resolves; production loads never fetch fixtures.
 */
async function loadWithFixture(page, fixture) {
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
  waitForGameReady,
  waitForPlayerBoundary,
};
