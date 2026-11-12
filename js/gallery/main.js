import { MANIFEST_URL, parseShowcaseNames, frameSource } from './showcases.js';

const SHOWCASE_SELECTOR = '[data-showcase]';
const STAGE_SELECTOR = '.showcase__stage';
const BUTTON_LABEL = 'Run';

async function loadBuiltShowcases() {
  const response = await fetch(MANIFEST_URL);
  if (!response.ok) {
    throw new Error(`${MANIFEST_URL} returned HTTP ${response.status}`);
  }
  return parseShowcaseNames(await response.json());
}

function addRunButton(article) {
  const name = article.dataset.showcase;
  const stage = article.querySelector(STAGE_SELECTOR);
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'showcase__run';
  button.textContent = BUTTON_LABEL + ' ' + name;
  button.addEventListener('click', () => {
    window.location.href = frameSource(name);
  });
  stage.replaceWith(button);
}

async function addShowcaseButtons() {
  let built;
  try {
    built = await loadBuiltShowcases();
  } catch (error) {
    console.warn(`Gallery showcases cannot run: ${error.message}`);
    return;
  }
  for (const article of document.querySelectorAll(SHOWCASE_SELECTOR)) {
    if (built.has(article.dataset.showcase)) {
      addRunButton(article);
    }
  }
}

addShowcaseButtons();
