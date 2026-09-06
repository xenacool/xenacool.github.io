// A1.0.1 render-baseline capture: records the oracle the Three.js runtime
// renderer (A1.0.3) must reproduce. Zero renderer change here — only a
// test-harness init script forces preserveDrawingBuffer and counts GL calls.
const { test, expect } = require('@playwright/test');
const { loadWithFixture, waitForPlayerBoundary } = require('./helpers');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const FIXTURES = ['render_synthetic', 'player_boundary'];
const OUT_ROOT = path.join(__dirname, '..', '..', '.junie', 'plans', 'visual-fixtures', 'render-baseline');

// This machine has no bundled Playwright browser; the system Chrome builds
// are stable enough for the baseline and keep other specs on their defaults.
test.use({
  viewport: { width: 1280, height: 720 },
  launchOptions: { channel: 'chrome' },
});

// Harness-only: force preserveDrawingBuffer so the canvas capture is stable,
// and count draw/texture calls for the capability record.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const origGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type, options) {
      if (typeof type === 'string' && type.startsWith('webgl')) {
        options = Object.assign({}, options, { preserveDrawingBuffer: true });
      }
      return origGetContext.call(this, type, options);
    };
    window.__glStats = { draws: 0, texs: 0 };
    const proto = window.WebGLRenderingContext.prototype;
    const origDrawElements = proto.drawElements;
    const origDrawArrays = proto.drawArrays;
    const origTexImage2D = proto.texImage2D;
    proto.drawElements = function (...args) {
      window.__glStats.draws += 1;
      return origDrawElements.apply(this, args);
    };
    proto.drawArrays = function (...args) {
      window.__glStats.draws += 1;
      return origDrawArrays.apply(this, args);
    };
    proto.texImage2D = function (...args) {
      window.__glStats.texs += 1;
      return origTexImage2D.apply(this, args);
    };
  });
});

async function waitForSettledPlayerBoundary(page) {
  await waitForPlayerBoundary(page, {
    after: { output: -1, input: -1, history: -1 },
    timeout: 90000,
  });
}

async function glStats(page) {
  return page.evaluate(() => {
    const canvas = document.getElementById('canvas');
    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
    if (!gl) throw new Error('active renderer did not expose WebGL2 or WebGL');
    const vp = gl.getParameter(gl.VIEWPORT);
    return {
      vendor: gl.getParameter(gl.VENDOR),
      renderer: gl.getParameter(gl.RENDERER),
      version: gl.getParameter(gl.VERSION),
      maxTextureSize: gl.getParameter(gl.MAX_TEXTURE_SIZE),
      maxTextureImageUnits: gl.getParameter(gl.MAX_TEXTURE_IMAGE_UNITS),
      maxViewportDims: Array.from(gl.getParameter(gl.MAX_VIEWPORT_DIMS)),
      viewport: [vp[0], vp[1]],
      drawingBuffer: [canvas.width, canvas.height],
      cssSize: [canvas.clientWidth, canvas.clientHeight],
      renderer: document.body.dataset.renderer || 'unknown',
      glError: gl.getError(),
      drawsTotal: window.__glStats.draws,
      texsTotal: window.__glStats.texs,
      uiLog: document.getElementById('log-container')?.textContent || '',
    };
  });
}

async function captureSettled(page, fixture) {
  if (fixture === null) {
    // Second run in the same page: routes persist, just reload.
  } else {
    await loadWithFixture(page, fixture);
  }
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 15000 });
  await waitForSettledPlayerBoundary(page);
  await page.waitForTimeout(500);
  const png = await page.locator('#canvas').screenshot();
  const stats = await glStats(page);
  const before = stats.drawsTotal;
  await page.waitForTimeout(1000);
  const after = await page.evaluate(() => window.__glStats.draws);
  stats.drawsPerSec = after - before;
  stats.viewport = { width: 1280, height: 720 };
  const logClean = !/error|panic|fail/i.test(stats.uiLog);
  return { png, stats, logClean };
}

const sha256 = (buf) => crypto.createHash('sha256').update(buf).digest('hex');

for (const fixture of FIXTURES) {
  test(`render baseline settles deterministically: ${fixture}`, async ({ page }) => {
    test.setTimeout(120000);
    const run1 = await captureSettled(page, fixture);
    expect(run1.stats.glError, `GL error after settled capture (${fixture})`).toBe(0);
    expect(run1.logClean, `UI log must be clean; got: ${run1.stats.uiLog}`).toBe(true);

    const run2 = await captureSettled(page, null);
    expect(run2.stats.glError, `GL error after second capture (${fixture})`).toBe(0);
    expect(run2.logClean, `UI log must stay clean on rerun (${fixture})`).toBe(true);

    const hash1 = sha256(run1.png);
    const hash2 = sha256(run2.png);
    expect(hash2, 'two consecutive captures of the same settled boundary must be byte-identical').toBe(hash1);

    const outDir = path.join(OUT_ROOT, fixture);
    fs.mkdirSync(outDir, { recursive: true });
    fs.writeFileSync(path.join(outDir, 'frame-1.png'), run1.png);
    fs.writeFileSync(path.join(outDir, 'frame-2.png'), run2.png);
    fs.writeFileSync(path.join(outDir, 'stats.json'), JSON.stringify({
      fixture,
      sha256: hash1,
      deterministic: true,
      glError: run1.stats.glError,
      uiLogClean: run1.logClean,
      capabilities: {
        vendor: run1.stats.vendor,
        renderer: run1.stats.renderer,
        version: run1.stats.version,
        maxTextureSize: run1.stats.maxTextureSize,
        maxTextureImageUnits: run1.stats.maxTextureImageUnits,
        maxViewportDims: run1.stats.maxViewportDims,
        drawingBuffer: run1.stats.drawingBuffer,
        cssSize: run1.stats.cssSize,
      },
      budget: {
        viewport: { width: 1280, height: 720 },
        drawsPerSec: run1.stats.drawsPerSec,
        texUploadsTotal: run1.stats.texsTotal,
      },
      seed: 42,
      three: null,
      capturedAt: new Date().toISOString(),
    }, null, 2));
  });
}

test('render baseline manifest', async () => {
  const manifest = {
    plan: 'A1.0.1 zero-renderer-change baseline lock',
    viewport: { width: 1280, height: 720 },
    seed: 42,
    three: null,
    fixtures: {},
  };
  for (const fixture of FIXTURES) {
    const statsPath = path.join(OUT_ROOT, fixture, 'stats.json');
    const stats = JSON.parse(fs.readFileSync(statsPath, 'utf8'));
    manifest.fixtures[fixture] = {
      sha256: stats.sha256,
      file: `${fixture}/frame-1.png`,
      glError: stats.glError,
      uiLogClean: stats.uiLogClean,
      capabilities: stats.capabilities,
      budget: stats.budget,
    };
  }
  fs.mkdirSync(OUT_ROOT, { recursive: true });
  fs.writeFileSync(path.join(OUT_ROOT, 'manifest.json'), JSON.stringify(manifest, null, 2));
  expect(manifest.fixtures[FIXTURES[0]].sha256, 'manifest must record a non-empty baseline hash').toMatch(/^[0-9a-f]{64}$/);
});
