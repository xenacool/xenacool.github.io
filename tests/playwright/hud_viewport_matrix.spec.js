import { test, expect } from '@playwright/test';

const viewports = [
  { name: 'desktop-wide', width: 1920, height: 1080 },
  { name: 'desktop-standard', width: 1366, height: 768 },
  { name: 'tablet-landscape', width: 1024, height: 768 },
  { name: 'tablet-portrait', width: 768, height: 1024 },
  { name: 'tablet-tall', width: 820, height: 1180 },
  { name: 'phone-small', width: 375, height: 667 },
  { name: 'phone-standard', width: 390, height: 844 },
  { name: 'phone-tall', width: 393, height: 873 },
  { name: 'phone-large', width: 430, height: 932 },
];

const modes = ['play', 'actions', 'history', 'diagnostics'];

test.describe('HUD viewport matrix', () => {
  for (const viewport of viewports) {
    test(`${viewport.name} keeps every HUD mode bounded and the scene visible`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto('/game.html');
      await expect(page.locator('#hud-mode-bar')).toBeVisible();

      for (const mode of modes) {
        await page.locator(`#hud-mode-bar button[data-hud-mode="${mode}"]`).click();
        const result = await page.evaluate(() => {
          const ids = [
            'hud-mode-bar',
            'action-controls',
            'history-log-camera-controls',
            'action-stack',
            'action-log-panel',
            'history-viewer',
            'diagnostics-panel',
            'heartbeat-panel',
            'entity-viewer',
          ];
          const viewportWidth = window.innerWidth;
          const viewportHeight = window.innerHeight;
          const visible = (element) => {
            const style = getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return style.display !== 'none'
              && style.visibility !== 'hidden'
              && rect.width > 0
              && rect.height > 0;
          };
          const boxes = ids
            .map((id) => document.getElementById(id))
            .filter((element) => element && visible(element))
            .map((element) => {
              const rect = element.getBoundingClientRect();
              return {
                id: element.id,
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
                width: rect.width,
                height: rect.height,
              };
            });

          const outOfBounds = boxes.filter((box) =>
            box.left < -1
            || box.top < -1
            || box.right > viewportWidth + 1
            || box.bottom > viewportHeight + 1,
          );
          const overlaps = [];
          for (let i = 0; i < boxes.length; i += 1) {
            for (let j = i + 1; j < boxes.length; j += 1) {
              const a = boxes[i];
              const b = boxes[j];
              const width = Math.min(a.right, b.right) - Math.max(a.left, b.left);
              const height = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
              if (width > 2 && height > 2) overlaps.push(`${a.id}:${b.id}`);
            }
          }

          // Sample the central scene region. A panel may occupy a perimeter,
          // but it must not cover the entire playable viewport.
          const sceneSamples = [];
          for (const x of [0.25, 0.5, 0.75]) {
            for (const y of [0.30, 0.45, 0.60, 0.70]) {
              const element = document.elementFromPoint(viewportWidth * x, viewportHeight * y);
              sceneSamples.push(element === document.getElementById('canvas'));
            }
          }
          const sceneVisibleSamples = sceneSamples.filter(Boolean).length;
          const hudArea = boxes.reduce((area, box) => area + box.width * box.height, 0);

          return {
            boxes,
            outOfBounds,
            overlaps,
            sceneVisibleSamples,
            hudAreaRatio: hudArea / (viewportWidth * viewportHeight),
          };
        });

        expect(result.outOfBounds, `${viewport.name}/${mode}: ${JSON.stringify(result)}`).toEqual([]);
        expect(result.overlaps, `${viewport.name}/${mode}: ${JSON.stringify(result)}`).toEqual([]);
        expect(result.sceneVisibleSamples, `${viewport.name}/${mode}: ${JSON.stringify(result)}`).toBeGreaterThanOrEqual(4);
        expect(result.hudAreaRatio, `${viewport.name}/${mode}: ${JSON.stringify(result)}`).toBeLessThan(0.55);
      }
    });
  }
});
