export const GALLERY_DIR = '/demos/gallery';
export const MANIFEST_URL = `${GALLERY_DIR}/showcases.json`;
export const FRAME_URL = `${GALLERY_DIR}/frame.html`;

const SHOWCASE_NAME_PATTERN = /^[a-z0-9_]+$/;

export function checkShowcaseName(name) {
  if (typeof name !== 'string' || !SHOWCASE_NAME_PATTERN.test(name)) {
    throw new Error(`Invalid showcase name "${name}": use lowercase letters, digits and underscores`);
  }
  return name;
}

export function parseShowcaseNames(entries) {
  if (!Array.isArray(entries)) {
    throw new Error(`${MANIFEST_URL} must hold a list of showcase names`);
  }
  const names = new Set();
  for (const entry of entries) {
    names.add(checkShowcaseName(entry));
  }
  return names;
}

export function showcaseFromSearch(search) {
  const name = new URLSearchParams(search).get('showcase');
  if (name === null) {
    throw new Error(`No showcase in "${search}": expected ?showcase=NAME`);
  }
  return checkShowcaseName(name);
}

export function frameSource(name) {
  return `${FRAME_URL}?showcase=${checkShowcaseName(name)}`;
}

export function scriptUrl(name) {
  const checked = checkShowcaseName(name);
  return `${GALLERY_DIR}/${checked}/${checked}.js`;
}
