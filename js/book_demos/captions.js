const EXAMPLE_PATTERN = /cargo run --example ([a-z0-9_]+)/;
const CAPTION_SELECTOR = 'p > em';

export function parseExampleName(captionText) {
  const match = EXAMPLE_PATTERN.exec(captionText);
  return match ? match[1] : null;
}

export function findDemoCaptions(root) {
  const captions = [];
  for (const emphasis of root.querySelectorAll(CAPTION_SELECTOR)) {
    const exampleName = parseExampleName(emphasis.textContent);
    if (exampleName) {
      captions.push({ element: emphasis.parentElement, exampleName });
    }
  }
  return captions;
}
