import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  checkShowcaseName,
  parseShowcaseNames,
  showcaseFromSearch,
  frameSource,
  scriptUrl,
} from './showcases.js';

test('checkShowcaseName accepts snake case names', () => {
  assert.equal(checkShowcaseName('kart_racer_editor'), 'kart_racer_editor');
  assert.equal(checkShowcaseName('physics_lab2'), 'physics_lab2');
});

test('checkShowcaseName rejects paths, empty and non-string names', () => {
  assert.throws(() => checkShowcaseName('../index'), /Invalid showcase name "..\/index"/);
  assert.throws(() => checkShowcaseName('Kart'), /Invalid showcase name/);
  assert.throws(() => checkShowcaseName(''), /Invalid showcase name/);
  assert.throws(() => checkShowcaseName(42), /Invalid showcase name/);
});

test('parseShowcaseNames returns the set of names', () => {
  const names = parseShowcaseNames(['cozy_castle', 'physics_lab']);
  assert.ok(names.has('cozy_castle'));
  assert.ok(names.has('physics_lab'));
  assert.equal(names.size, 2);
});

test('parseShowcaseNames rejects a manifest that is not a list of names', () => {
  assert.throws(() => parseShowcaseNames({ name: 'cozy_castle' }), /must hold a list/);
  assert.throws(() => parseShowcaseNames(['cozy castle']), /Invalid showcase name/);
});

test('showcaseFromSearch reads the showcase parameter', () => {
  assert.equal(showcaseFromSearch('?showcase=cozy_castle'), 'cozy_castle');
  assert.equal(showcaseFromSearch('?debug=1&showcase=physics_lab'), 'physics_lab');
});

test('showcaseFromSearch fails loudly on a missing or invalid showcase', () => {
  assert.throws(() => showcaseFromSearch(''), /No showcase in ""/);
  assert.throws(() => showcaseFromSearch('?showcase=a/b'), /Invalid showcase name "a\/b"/);
});

test('frameSource and scriptUrl point into the gallery directory', () => {
  assert.equal(frameSource('cozy_castle'), '/demos/gallery/frame.html?showcase=cozy_castle');
  assert.equal(scriptUrl('cozy_castle'), '/demos/gallery/cozy_castle/cozy_castle.js');
});
