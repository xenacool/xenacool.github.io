import http.server
import socketserver
import sys
import os
import time
import threading


class MyHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        # Dev server: never let the browser hold onto a stale wasm or script.
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


def _watched_paths(root):
    """Absolute paths of everything the server may need to (re)load on:
    this source file plus every file under the served tree."""
    base = os.path.abspath(root)
    paths = [os.path.abspath(__file__)]
    if os.path.isdir(base):
        for dirpath, _dirs, filenames in os.walk(base):
            for name in filenames:
                path = os.path.join(dirpath, name)
                if os.path.exists(path):
                    paths.append(path)
    return paths


def _snapshot(paths):
    """Map path -> mtime_ns for the files we watch (absent files skipped)."""
    snap = {}
    for path in paths:
        try:
            snap[path] = os.stat(path).st_mtime_ns
        except OSError:
            pass
    return snap


def _watch_loop(stop, httpd, snapshot):
    """Poll watched mtimes; when any drifts from the snapshot, ask serve_forever
    to stop so the outer loop rebinds the server (a stdlib-only hot reload)."""
    while not stop.is_set():
        time.sleep(0.3)
        drift = any(
            os.path.exists(path) and snapshot.get(path) != os.stat(path).st_mtime_ns
            for path in snapshot
        )
        if drift:
            print("[hot reload] content changed; restarting server")
            stop.set()
            try:
                httpd.shutdown()
            except Exception:
                pass


def serve_with_hot_reload(port, root="."):
    """Serve `root` and automatically (re)bind the server whenever any watched
    file (server source or content) changes. Runs until interrupted (Ctrl-C).

    stdlib-only: poll mtimes in a watcher thread and call shutdown() to unblock
    serve_forever(); the outer loop then rebinds. Designed to be short and hold
    a single question (when did content change?) per function.
    """
    base = os.path.abspath(root)
    paths = _watched_paths(root)
    socketserver.TCPServer.allow_reuse_address = True
    while True:
        httpd = socketserver.TCPServer(("", port), MyHTTPRequestHandler)
        print(f"Serving at port {port} (hot-reload watching {base or '.'})")
        snapshot = _snapshot(paths)
        stop = threading.Event()
        threading.Thread(target=_watch_loop, args=(stop, httpd, snapshot), daemon=True).start()
        httpd.serve_forever()
        stop.set()
        httpd.server_close()


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    root = sys.argv[2] if len(sys.argv) > 2 else "web"
    serve_with_hot_reload(port, root)
