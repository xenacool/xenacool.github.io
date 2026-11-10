import { isBookIndex, otherVariantPath, parseBookPath } from './book_variant_switch.js';

const MENU_SELECTOR = '.right-buttons';
const LABEL_TO_INTERACTIVE = 'Interactive version';
const LABEL_TO_PLAIN = 'Plain text version';

function createSwitchLink(isInteractive, href) {
  const link = document.createElement('a');
  link.className = 'book-variant-switch';
  link.href = href;
  link.textContent = isInteractive ? LABEL_TO_PLAIN : LABEL_TO_INTERACTIVE;
  return link;
}

function addSwitch() {
  const { pathname, search, hash } = window.location;
  if (isBookIndex(pathname)) {
    return;
  }
  const menu = document.querySelector(MENU_SELECTOR);
  if (!menu) {
    console.warn(`Variant switch skipped: no "${MENU_SELECTOR}" element on this page`);
    return;
  }
  const { isInteractive } = parseBookPath(pathname);
  const href = otherVariantPath(pathname) + search + hash;
  menu.prepend(createSwitchLink(isInteractive, href));
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', addSwitch);
} else {
  addSwitch();
}
