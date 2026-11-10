const MANIFEST_URL = '/demos/wasm/examples.json';

export function libraryFromPath(pathname) {
  const library = pathname.split('/')[1];
  if (!library) {
    throw new Error(`Cannot find a library name in path "${pathname}"`);
  }
  return library;
}

export function manifestKey(library, exampleName) {
  return `${library}/${exampleName}`;
}

export async function loadManifest() {
  const response = await fetch(MANIFEST_URL);
  if (!response.ok) {
    throw new Error(`${MANIFEST_URL} returned HTTP ${response.status}`);
  }
  const entries = await response.json();
  return new Set(entries.map((entry) => manifestKey(entry.lib, entry.name)));
}
