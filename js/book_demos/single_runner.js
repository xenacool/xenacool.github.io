export function createSingleRunner() {
  let stopCurrent = null;

  function stop() {
    if (stopCurrent) {
      stopCurrent();
      stopCurrent = null;
    }
  }

  function run(widget, src) {
    stop();
    widget.showFrame(src);
    stopCurrent = () => widget.showPlaceholder();
  }

  return { run, stop };
}
