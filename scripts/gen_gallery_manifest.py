#!/usr/bin/env python3
"""
Write demos/gallery/showcases.json: the names of the showcases that are built.

A showcase is built when demos/gallery/NAME/ holds NAME.js and NAME_bg.wasm.
The gallery page shows a Run button only for the names in this list, so a
showcase appears as soon as it has a build.

Usage:
    python3 scripts/gen_gallery_manifest.py
"""

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GALLERY_DIR = REPO_ROOT / "demos" / "gallery"
MANIFEST = GALLERY_DIR / "showcases.json"


def is_built(showcase_dir):
    name = showcase_dir.name
    script = showcase_dir / f"{name}.js"
    binary = showcase_dir / f"{name}_bg.wasm"
    if script.exists() != binary.exists():
        sys.exit(f"{showcase_dir.relative_to(REPO_ROOT)}: needs both {script.name} and {binary.name}")
    return script.exists()


def built_showcases():
    names = []
    for showcase_dir in sorted(GALLERY_DIR.iterdir()):
        if showcase_dir.is_dir() and is_built(showcase_dir):
            names.append(showcase_dir.name)
    return names


def main():
    if not GALLERY_DIR.is_dir():
        sys.exit(f"No gallery at {GALLERY_DIR.relative_to(REPO_ROOT)}, run make build-gallery")
    names = built_showcases()
    MANIFEST.write_text(json.dumps(names, indent=2) + "\n", encoding="utf-8")
    print(f"Gallery manifest: {len(names)} showcases in {MANIFEST.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
