function createButton(label, onClick) {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'book-demo__button';
  button.textContent = label;
  button.addEventListener('click', onClick);
  return button;
}

function createFrame(src, title, onLoad) {
  const frame = document.createElement('iframe');
  frame.className = 'book-demo__frame';
  frame.title = title;
  frame.src = src;
  frame.allow = 'fullscreen';
  frame.addEventListener('load', onLoad);
  return frame;
}

function createElementWithClass(tagName, className) {
  const element = document.createElement(tagName);
  element.className = className;
  return element;
}

export function createDemoWidget({ title, onRun, onStop }) {
  const element = createElementWithClass('figure', 'book-demo');
  const stage = createElementWithClass('div', 'book-demo__stage');
  const bar = createElementWithClass('div', 'book-demo__bar');
  const status = createElementWithClass('p', 'book-demo__status');
  status.setAttribute('aria-live', 'polite');
  bar.append(status);
  element.append(stage, bar);

  function showPlaceholder() {
    stage.replaceChildren(createButton(`Run ${title}`, onRun));
    bar.replaceChildren(status);
    status.textContent = '';
  }

  function showFrame(src) {
    const onLoad = () => { status.textContent = `${title} is running.`; };
    stage.replaceChildren(createFrame(src, `${title} live demo`, onLoad));
    bar.replaceChildren(createButton('Stop', onStop), status);
    status.textContent = `Loading ${title}...`;
  }

  showPlaceholder();
  return { element, showFrame, showPlaceholder };
}
