import { isBookIndex } from '../book_variant_switch.js';

const GALLERY_URL = '/demos/gallery.html';
const MENU_SELECTOR = '.right-buttons';
const GALLERY_LABEL = 'Demo gallery';

function createGalleryLink() {
  const link = document.createElement('a');
  link.className = 'book-variant-switch';
  link.href = GALLERY_URL;
  link.textContent = GALLERY_LABEL;
  return link;
}

export function addGalleryLink() {
  if (isBookIndex(window.location.pathname)) {
    return;
  }
  const menu = document.querySelector(MENU_SELECTOR);
  if (!menu) {
    console.warn(`Gallery link skipped: no "${MENU_SELECTOR}" element on this page`);
    return;
  }
  menu.prepend(createGalleryLink());
}
