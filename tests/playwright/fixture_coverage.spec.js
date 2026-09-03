// Browserless coverage checks for the web/scripts fixture convention.
const { test, expect } = require('@playwright/test');
const fs = require('node:fs');
const path = require('node:path');

const scriptsDir = path.resolve(__dirname, '..', '..', 'web', 'scripts');
const fixturesDir = path.join(scriptsDir, 'fixtures');

function specSources() {
  return [__dirname, path.join(__dirname, 'trpg')]
    .flatMap((dir) => fs.readdirSync(dir)
      .filter((file) => file.endsWith('.spec.js'))
      .map((file) => fs.readFileSync(path.join(dir, file), 'utf8')))
    .join('\n');
}

test('every fixture is referenced by at least one spec', () => {
  const sources = specSources();
  for (const file of fs.readdirSync(fixturesDir)) {
    if (!file.endsWith('.rhai')) continue;
    const stem = file.replace(/\.rhai$/, '');
    expect(
      sources,
      `${file} is not referenced by any spec`,
    ).toMatch(new RegExp(`['"]${stem}['"]`));
  }
});

test('every fixture passed to a spec loader exists on disk', () => {
  const sources = specSources();
  const refs = [...sources.matchAll(/(?:loadWithFixture|loadScenario)\(page,\s*'([^']+)'/g)]
    .map((match) => match[1]);
  for (const stem of new Set(refs)) {
    expect(
      fs.existsSync(path.join(fixturesDir, `${stem}.rhai`)),
      `spec references missing fixture ${stem}.rhai`,
    ).toBe(true);
  }
});

test('production manifest contains no fixtures and every entry exists', () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(scriptsDir, 'manifest.json'), 'utf8'));
  for (const file of manifest.files) {
    expect(file, `fixture leaked into production manifest: ${file}`).not.toMatch(/^fixtures\//);
    expect(fs.existsSync(path.join(scriptsDir, file)), `missing manifest entry ${file}`).toBe(true);
  }
});
