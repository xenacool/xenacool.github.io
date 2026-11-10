import { MANIFEST_URL, parseShowcaseNames, frameSource } from './showcases.js';
import { createDemoWidget } from '../book_demos/demo_widget.js';
import { createSingleRunner } from '../book_demos/single_runner.js';

const SHOWCASE_SELECTOR = '[data-showcase]';
const STAGE_SELECTOR = '.showcase__stage';

async function loadBuiltShowcases() {
  const response = await fetch(MANIFEST_URL);
  if (!response.ok) {
    throw new Error(`${MANIFEST_URL} returned HTTP ${response.status}`);
  }
  return parseShowcaseNames(await response.json());
}

function addRunWidget(article, runner) {
  const name = article.dataset.showcase;
  const widget = createDemoWidget({
    title: name,
    onRun: () => runner.run(widget, frameSource(name)),
    onStop: () => runner.stop(),
  });
  article.querySelector(STAGE_SELECTOR).replaceWith(widget.element);
}

async function addShowcaseWidgets() {
  let built;
  try {
    built = await loadBuiltShowcases();
  } catch (error) {
    console.warn(`Gallery showcases cannot run: ${error.message}`);
    return;
  }
  const runner = createSingleRunner();
  for (const article of document.querySelectorAll(SHOWCASE_SELECTOR)) {
    if (built.has(article.dataset.showcase)) {
      addRunWidget(article, runner);
    }
  }
}

addShowcaseWidgets();
