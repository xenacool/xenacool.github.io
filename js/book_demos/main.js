import { findDemoCaptions } from './captions.js';
import { loadManifest, libraryFromPath, manifestKey } from './manifest.js';
import { createDemoWidget } from './demo_widget.js';
import { createSingleRunner } from './single_runner.js';
import { addGalleryLink } from './gallery_link.js';

const DEMO_FRAME_URL = '/demos/wasm/demo-frame.html';

export function frameSource(exampleName) {
  return `${DEMO_FRAME_URL}?example=${encodeURIComponent(exampleName)}`;
}

function insertWidget(caption, runner) {
  const { element, exampleName } = caption;
  const widget = createDemoWidget({
    title: exampleName,
    onRun: () => runner.run(widget, frameSource(exampleName)),
    onStop: () => runner.stop(),
  });
  element.before(widget.element);
}

async function addDemos() {
  addGalleryLink();
  let manifest;
  try {
    manifest = await loadManifest();
  } catch (error) {
    console.warn(`Interactive demos disabled: ${error.message}`);
    return;
  }
  const library = libraryFromPath(window.location.pathname);
  const runner = createSingleRunner();
  for (const caption of findDemoCaptions(document)) {
    if (manifest.has(manifestKey(library, caption.exampleName))) {
      insertWidget(caption, runner);
    }
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', addDemos);
} else {
  addDemos();
}
