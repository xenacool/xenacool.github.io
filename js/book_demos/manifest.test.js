import test from 'node:test';
import assert from 'node:assert/strict';
import { libraryFromPath, manifestKey, loadManifest } from './manifest.js';

test('libraryFromPath reads the first path segment', () => {
  assert.equal(libraryFromPath('/bevy_gradmesh/wasm/shapes.html'), 'bevy_gradmesh');
  assert.equal(libraryFromPath('/bevy_composable_dragging/blending.html'), 'bevy_composable_dragging');
});

test('libraryFromPath rejects the site root', () => {
  assert.throws(() => libraryFromPath('/'), /Cannot find a library name/);
});

test('manifestKey joins library and example', () => {
  assert.equal(manifestKey('bevy_gradmesh', 'shapes'), 'bevy_gradmesh/shapes');
});

test('loadManifest builds a set of keys', async () => {
  const entries = [{ lib: 'bevy_gradmesh', name: 'shapes' }];
  globalThis.fetch = async () => ({ ok: true, json: async () => entries });
  const manifest = await loadManifest();
  assert.ok(manifest.has('bevy_gradmesh/shapes'));
  assert.ok(!manifest.has('bevy_gradmesh/cozy_castle_3d'));
});

test('loadManifest fails loudly on an HTTP error', async () => {
  globalThis.fetch = async () => ({ ok: false, status: 404 });
  await assert.rejects(loadManifest(), /HTTP 404/);
});
