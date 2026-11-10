import { showcaseFromSearch, scriptUrl } from './showcases.js';

const STATUS_SELECTOR = '.showcase-frame__status';

async function startShowcase() {
  const name = showcaseFromSearch(window.location.search);
  const showcase = await import(scriptUrl(name));
  await showcase.default();
}

async function main() {
  const status = document.querySelector(STATUS_SELECTOR);
  try {
    await startShowcase();
    status.remove();
  } catch (error) {
    status.textContent = `Error: ${error.message}`;
    throw error;
  }
}

main();
