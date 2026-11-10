import test from 'node:test';
import assert from 'node:assert/strict';
import { otherVariantPath, isBookIndex, parseBookPath } from './book_variant_switch.js';

const BOOKS = ['bevy_gradmesh', 'bevy_composable_dragging'];

test('otherVariantPath maps a plain chapter to the interactive chapter', () => {
  for (const book of BOOKS) {
    assert.equal(otherVariantPath(`/${book}/shapes.html`), `/${book}/wasm/shapes.html`);
  }
});

test('otherVariantPath maps an interactive chapter to the plain chapter', () => {
  for (const book of BOOKS) {
    assert.equal(otherVariantPath(`/${book}/wasm/shapes.html`), `/${book}/shapes.html`);
  }
});

test('otherVariantPath round-trips', () => {
  const plain = '/bevy_gradmesh/curves.html';
  assert.equal(otherVariantPath(otherVariantPath(plain)), plain);
});

test('isBookIndex is true for index and first pages of both variants', () => {
  for (const book of BOOKS) {
    assert.ok(isBookIndex(`/${book}/`));
    assert.ok(isBookIndex(`/${book}`));
    assert.ok(isBookIndex(`/${book}/index.html`));
    assert.ok(isBookIndex(`/${book}/introduction.html`));
    assert.ok(isBookIndex(`/${book}/wasm/`));
    assert.ok(isBookIndex(`/${book}/wasm/index.html`));
    assert.ok(isBookIndex(`/${book}/wasm/introduction.html`));
  }
});

test('isBookIndex is false for chapters', () => {
  assert.ok(!isBookIndex('/bevy_gradmesh/shapes.html'));
  assert.ok(!isBookIndex('/bevy_gradmesh/wasm/shapes.html'));
});

test('parseBookPath rejects the site root', () => {
  assert.throws(() => parseBookPath('/'), /Cannot find a book name/);
});
