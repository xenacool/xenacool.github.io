const VARIANT_SEGMENT = 'wasm';
const INDEX_PAGES = new Set(['', 'index.html', 'introduction.html']);

export function parseBookPath(pathname) {
  const [, book, ...rest] = pathname.split('/');
  if (!book) {
    throw new Error(`Cannot find a book name in path "${pathname}"`);
  }
  const isInteractive = rest[0] === VARIANT_SEGMENT;
  const pageSegments = isInteractive ? rest.slice(1) : rest;
  return { book, isInteractive, page: pageSegments.join('/') };
}

export function otherVariantPath(pathname) {
  const { book, isInteractive, page } = parseBookPath(pathname);
  if (isInteractive) {
    return `/${book}/${page}`;
  }
  return `/${book}/${VARIANT_SEGMENT}/${page}`;
}

export function isBookIndex(pathname) {
  return INDEX_PAGES.has(parseBookPath(pathname).page);
}
