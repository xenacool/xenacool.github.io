import { showcaseFromSearch, scriptUrl } from './showcases.js';

const STATUS_SELECTOR = '.showcase-frame__status';

async function startShowcase() {
  const name = showcaseFromSearch(window.location.search);
  const showcase = await import(scriptUrl(name));
  await showcase.default();
}

function handleResize() {
  const canvas = document.querySelector('canvas');
  if (!canvas) {
    return;
  }
  // Trigger a resize event on the canvas so Bevy's winit adapter picks it up.
  // This works because fit_canvas_to_parent is true and the camera uses AutoMin.
  const event = new Event('resize');
  canvas.dispatchEvent(event);
  window.dispatchEvent(event);
}

function closeDemo() {
  window.close();
  // If window.close() doesn't work (e.g., not opened by script), navigate back.
  if (!window.closed) {
    window.history.back();
  }
}

function setupCloseHandler() {
  const closeBtn = document.querySelector('.showcase-frame__close');
  if (closeBtn) {
    closeBtn.addEventListener('click', closeDemo);
  }

  // Click on the background (not the canvas) to close.
  document.body.addEventListener('click', (event) => {
    if (event.target === document.body || event.target.classList.contains('showcase-frame')) {
      closeDemo();
    }
  });
}

async function main() {
  const status = document.querySelector(STATUS_SELECTOR);
  try {
    await startShowcase();
    status.remove();
    setupCloseHandler();
    window.addEventListener('resize', handleResize);
    window.addEventListener('orientationchange', () => {
      // Orientation change fires before resize on some browsers; wait a tick.
      setTimeout(handleResize, 100);
    });
  } catch (error) {
    status.textContent = `Error: ${error.message}`;
    throw error;
  }
}

main();
