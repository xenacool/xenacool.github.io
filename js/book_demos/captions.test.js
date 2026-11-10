import test from 'node:test';
import assert from 'node:assert/strict';
import { parseExampleName, findDemoCaptions } from './captions.js';

test('parseExampleName reads a plain caption', () => {
  const text = 'Spheres and boxes. Run cargo run --example shapes to reproduce it.';
  assert.equal(parseExampleName(text), 'shapes');
});

test('parseExampleName stops before the features flag', () => {
  const text = 'A slider. Run cargo run --example blend_square --features pbr.';
  assert.equal(parseExampleName(text), 'blend_square');
});

test('parseExampleName returns null for other text', () => {
  assert.equal(parseExampleName('Run cargo build --release.'), null);
  assert.equal(parseExampleName(''), null);
});

function fakeEmphasis(textContent, parent) {
  return { textContent, parentElement: parent };
}

test('findDemoCaptions returns the paragraph and name of each caption', () => {
  const first = {};
  const second = {};
  const root = {
    querySelectorAll: () => [
      fakeEmphasis('Run cargo run --example shapes.', first),
      fakeEmphasis('An ordinary emphasis.', {}),
      fakeEmphasis('Run cargo run --example shapes.', second),
    ],
  };
  const captions = findDemoCaptions(root);
  assert.equal(captions.length, 2);
  assert.equal(captions[0].element, first);
  assert.equal(captions[1].element, second);
  assert.equal(captions[1].exampleName, 'shapes');
});
