const { test, expect } = require('@playwright/test');
const fs = require('fs');

const replayFixturePath = process.env.PYSTRAL_REPLAY_FIXTURE
  || 'debug-fixtures/pystral-replay-2026-08-12T22-19-28.130Z.json';
const replayFixture = JSON.parse(fs.readFileSync(
  replayFixturePath,
  'utf8',
));

async function waitForPlayerBoundary(page) {
  try {
    await page.waitForFunction(() => {
      const slider = document.getElementById('history-slider');
      const menu = document.getElementById('action-menu');
      return slider
        && Number(slider.value) === Number(slider.max)
        && menu?.style.display === 'block'
        && menu.dataset.actionPending !== 'true';
    }, { timeout: 8000 });
  } catch (error) {
    const diagnostics = await page.evaluate(() => ({
      status: window.__pystralWorkerStatus,
      reason: document.getElementById('heartbeat-reason').textContent,
      traces: window.__pystralDebugTraces,
    }));
    throw new Error(`${error.message}\nboundary diagnostics: ${JSON.stringify(diagnostics)}`);
  }
}

async function waitForBridgeIdle(page, timeout = 500) {
  await page.waitForFunction(() => {
    const status = window.__pystralWorkerStatus || '';
    return Date.now() - window.__pystralHeartbeatReceivedAt < 1000
      && status.includes('simulation request None')
      && !status.includes('WaitingForAnimationAck');
  }, { timeout });
}

async function waitForAnimationAcquiescence(page, timeout = 500) {
  await page.waitForFunction(() => new Promise((resolve) => {
    const isQuiescent = () => {
      const slider = document.getElementById('history-slider');
      const menu = document.getElementById('action-menu');
      return slider
        && Number(slider.value) === Number(slider.max)
        && menu?.style.display === 'block'
        && menu.dataset.actionPending !== 'true'
        && menu.dataset.animationPending !== 'true'
        && menu.dataset.waitPending !== 'true';
    };
    requestAnimationFrame(() => requestAnimationFrame(() => resolve(isQuiescent())));
  }), { timeout });
}

async function sendReplayInputAfterQuiescence(page, input) {
  let lastError;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      await waitForBridgeIdle(page);
      await waitForAnimationAcquiescence(page);
      const before = await page.evaluate((actionInput) => ({
        accepted: window.__pystralDebugTraces.filter(
          (trace) => trace === `unified worker accepted action input ${actionInput}`,
        ).length,
        inputSeq: window.__pystralWorkerLatestInputSeq,
      }), input);
      await page.evaluate((actionInput) => {
        window.app.action_nav(actionInput);
      }, input);
      await page.waitForFunction(
        ({ actionInput, accepted }) => window.__pystralDebugTraces.filter(
          (trace) => trace === `unified worker accepted action input ${actionInput}`,
        ).length > accepted,
        { actionInput: input, accepted: before.accepted },
        { timeout: 2000 },
      );
      await page.waitForFunction(
        (previousInputSeq) => window.__pystralWorkerLatestInputSeq > previousInputSeq,
        before.inputSeq,
        { timeout: 2000 },
      );
      return;
    } catch (error) {
      lastError = error;
    }
  }
  const diagnostics = await page.evaluate(() => ({
    input: window.__replayLastInput,
    heartbeat: Date.now() - window.__pystralHeartbeatReceivedAt,
    status: window.__pystralWorkerStatus,
    menu: document.getElementById('action-menu')?.outerHTML.slice(0, 800),
    traces: window.__pystralDebugTraces,
  }));
  throw new Error(`${lastError?.message}\ninput diagnostics: ${JSON.stringify(diagnostics)}`);
}

test.describe('Worker heartbeat diagnostics', () => {
  test('keeps worker liveness distinguishable from stale progress', async ({ page }) => {
    // Keep this regression intentionally short: the manual failure took
    // hundreds of seconds because the old heartbeat only moved on gameplay
    // input. Probe responses now keep the distinction observable immediately.
    // Full-suite browser contention can make WASM startup take longer than a
    // focused run, but this still reproduces liveness in tens of seconds
    // rather than the hundreds required by the original failure.
    test.setTimeout(30000);
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
    await page.check('#debug-checkbox');

    await page.waitForFunction(() => window.__pystralHeartbeatReceivedAt > 0, {
      timeout: 8000,
    });
    await page.waitForFunction(
      () => Date.now() - window.__pystralHeartbeatReceivedAt < 3000,
      { timeout: 8000 },
    );
    await expect(page.locator('#heartbeat-message')).toHaveText(/latest output seq: \d+ · \d+s ago/);
    await expect(page.locator('#heartbeat-reason')).toContainText(
      /Worker responsive.*(AwaitingPlayerDecision|WaitingForAnimationAck|Simulating|Idle|Completed)/,
    );
  });

  test('reports each simulation bridge startup checkpoint', async ({ page }) => {
    test.setTimeout(20000);
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
    await page.waitForFunction(
      () => window.__pystralDebugTraces.some(trace => trace.includes('simulation bridge send request seq')),
      { timeout: 8000 },
    );
    await page.waitForFunction(
      () => window.__pystralDebugTraces.some(trace => trace.includes('simulation worker heartbeat')),
      { timeout: 8000 },
    );
    await page.waitForFunction(
      () => window.__pystralDebugTraces.some(trace => trace.includes('simulation bridge received response')),
      { timeout: 8000 },
    );
    await page.waitForFunction(
      () => window.__pystralDebugTraces.some(trace => trace.includes('main thread applying transient')),
      { timeout: 8000 },
    );
  });

  test('copies the UI log and downloads an export-shaped history case', async ({ page }) => {
    test.setTimeout(20000);
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
    await page.waitForFunction(() => window.__pystralHistoryExportSource !== null, {
      timeout: 8000,
    });
    await page.waitForFunction(() => {
      const source = window.__pystralHistoryExportSource;
      if (!source) return false;
      try {
        return Array.isArray(JSON.parse(source).replay?.actionInputs);
      } catch (_) {
        return false;
      }
    }, { timeout: 8000 });

    await expect(page.locator('#log-copy')).toHaveValue(/UI log messages|Action input|Runtime/);
    await page.locator('#copy-ui-log').click();
    await expect(page.locator('#diagnostics-download-status')).toContainText(/copied|selected/);

    const downloadPromise = page.waitForEvent('download');
    await page.locator('#download-history').click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/^pystral-replay-.*\.json$/);
    const exportData = JSON.parse(fs.readFileSync(await download.path(), 'utf8'));
    expect(exportData.format).toBe('pystral-gate-replay-v1');
    expect(exportData.replay.entrypoint).toBe('web/scripts/demo.rhai');
    expect(exportData.replay.actionInputs).toEqual(expect.any(Array));
    expect(Object.keys(exportData).sort()).toEqual(['format', 'replay']);
    await expect(page.locator('#diagnostics-download-status')).toHaveText('Replay download started.');
  });

  test('does not continuously serialize history while debug is closed', async ({ page }) => {
    test.setTimeout(20000);
    await page.goto('/');
    await page.waitForFunction(() => window.__pystralHistoryExportUpdateCount > 0, {
      timeout: 8000,
    });
    await page.waitForFunction(() => {
      const slider = document.getElementById('history-slider');
      return slider && Number(slider.value) === Number(slider.max);
    }, { timeout: 8000 });

    const before = await page.evaluate(() => window.__pystralHistoryExportUpdateCount);
    const inputBefore = await page.evaluate(() => window.__pystralWorkerLatestInputSeq);
    await page.waitForTimeout(750);
    const after = await page.evaluate(() => window.__pystralHistoryExportUpdateCount);
    const inputAfter = await page.evaluate(() => window.__pystralWorkerLatestInputSeq);

    // Stable history may trigger one final append/export while this interval
    // starts, but it must not serialize once per animation frame.
    expect(after - before).toBeLessThanOrEqual(1);
    // The 500 ms heartbeat probe accounts for normal input growth. A
    // per-frame animation ACK flood would add dozens in this interval.
    expect(inputAfter - inputBefore).toBeLessThanOrEqual(4);
  });

  test('keeps answering probes while control input is flooded', async ({ page }) => {
    test.setTimeout(30000);
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
    await page.waitForFunction(() => window.__pystralHeartbeatReceivedAt > 0, {
      timeout: 8000,
    });

    const inputBefore = await page.evaluate(() => window.__pystralWorkerLatestInputSeq);
    await page.evaluate(() => {
      window.__probeFloodStartedAt = Date.now();
      window.__probeFloodInputBefore = window.__pystralWorkerLatestInputSeq;
      // ActionNav is deliberately harmless here. The volume models a noisy
      // renderer/debug client and must not prevent the worker from yielding
      // to its heartbeat traffic.
      for (let i = 0; i < 5000; i += 1) window.app.action_nav('left');
    });

    await page.waitForFunction(
      () => window.__pystralHeartbeatReceivedAt >= window.__probeFloodStartedAt
        && window.__pystralWorkerLatestInputSeq > window.__probeFloodInputBefore,
      { timeout: 5000 },
    );
    const result = await page.evaluate(() => ({
      elapsed: window.__pystralHeartbeatReceivedAt - window.__probeFloodStartedAt,
      inputAfter: window.__pystralWorkerLatestInputSeq,
    }));
    expect(result.inputAfter - inputBefore).toBeGreaterThan(1000);
    expect(result.elapsed).toBeLessThan(2500);
  });

  test('secondary ability followed by Wait does not starve in Simulating', async ({ page }) => {
    // Reduced from pystral-history-2026-08-12T20-16-54.118Z.json.
    test.setTimeout(45000);
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
    try {
    await page.waitForFunction(() => {
      const menu = document.getElementById('action-menu');
      return menu?.style.display === 'block'
        && menu.querySelector('h2')?.textContent.match(/^Unit \d+ action menu$/);
    }, { timeout: 10000 });
    } catch (error) {
      const diagnostics = await page.evaluate(() => ({
        status: window.__pystralWorkerStatus,
        reason: document.getElementById('heartbeat-reason').textContent,
        menu: document.getElementById('action-menu').outerHTML.slice(0, 500),
        traces: window.__pystralDebugTraces,
      }));
      throw new Error(`${error.message}\nstartup diagnostics: ${JSON.stringify(diagnostics)}`);
    }

    for (let attempts = 0; attempts < 16; attempts += 1) {
      const heading = await page.getByRole('heading', { name: /unit \d+ action menu/i }).innerText();
      if (heading === 'Unit 1 action menu') break;
      await page.getByRole('button', { name: 'Wait' }).click();
      await waitForPlayerBoundary(page);
    }
    await expect(page.getByRole('heading', { name: 'Unit 1 action menu' })).toBeVisible();

    await page.locator('[data-menu-key="job:secondary:0"]').click();
    await page.getByRole('button', { name: /Fireball/ }).click();
    await page.locator('[data-menu-key^="target:"]').first().click();
    await page.getByRole('button', { name: 'Action' }).click({ force: true });
    await waitForPlayerBoundary(page);

    await page.getByRole('button', { name: 'Wait' }).click();
    await waitForPlayerBoundary(page);
    await expect.poll(
      () => page.evaluate(() => window.__pystralWorkerStatus),
      { timeout: 5000 },
    ).not.toBe('Simulating');
  });

  test('replay fixture keeps heartbeat alive during secondary confirm', async ({ page }) => {
    test.setTimeout(60000);
    await page.goto('/');
    await page.waitForFunction(() => window.app !== undefined, { timeout: 8000 });
    await page.waitForFunction(() => window.__pystralHeartbeatReceivedAt > 0, {
      timeout: 8000,
    });

    const replayLimit = Number(process.env.PYSTRAL_REPLAY_LIMIT || 0);
    const inputs = replayLimit
      ? replayFixture.replay.actionInputs.slice(0, replayLimit)
      : replayFixture.replay.actionInputs;
    expect(inputs.length).toBeGreaterThan(0);

    for (const input of inputs) {
      await page.evaluate((actionInput) => {
        window.__replayLastInput = actionInput;
      }, input);
      await sendReplayInputAfterQuiescence(page, input);
      if (process.env.PYSTRAL_PRINT_REPLAY_TRACES) {
        console.log(`replay input applied: ${input}`);
      }
    }

    const heartbeatBefore = await page.evaluate(() => window.__pystralHeartbeatReceivedAt);
    await page.waitForTimeout(10000);
    const result = await page.evaluate((before) => ({
      heartbeatAge: Date.now() - window.__pystralHeartbeatReceivedAt,
      heartbeatAdvanced: window.__pystralHeartbeatReceivedAt > before,
      status: window.__pystralWorkerStatus,
      reason: document.getElementById('heartbeat-reason').textContent,
      traces: window.__pystralDebugTraces,
    }), heartbeatBefore);

    if (process.env.PYSTRAL_PRINT_REPLAY_TRACES) {
      console.log(`replay result: ${JSON.stringify(result)}`);
    }

    expect(result.heartbeatAdvanced, result.reason).toBe(true);
    expect(result.heartbeatAge, result.reason).toBeLessThan(2500);
    expect(
      result.status,
      `${result.reason}; traces=${JSON.stringify(result.traces)}`,
    ).not.toContain('Simulating');
  });
});
