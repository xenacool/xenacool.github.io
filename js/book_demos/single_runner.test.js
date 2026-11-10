import test from 'node:test';
import assert from 'node:assert/strict';
import { createSingleRunner } from './single_runner.js';

function fakeWidget(log, name) {
  return {
    showFrame: (src) => log.push(`${name} frame ${src}`),
    showPlaceholder: () => log.push(`${name} placeholder`),
  };
}

test('starting a second demo stops the first', () => {
  const log = [];
  const runner = createSingleRunner();
  runner.run(fakeWidget(log, 'a'), 'src-a');
  runner.run(fakeWidget(log, 'b'), 'src-b');
  assert.deepEqual(log, ['a frame src-a', 'a placeholder', 'b frame src-b']);
});

test('stop restores the running widget once', () => {
  const log = [];
  const runner = createSingleRunner();
  runner.run(fakeWidget(log, 'a'), 'src-a');
  runner.stop();
  runner.stop();
  assert.deepEqual(log, ['a frame src-a', 'a placeholder']);
});
