#!/usr/bin/env python3
"""
Check that the visible text of main pages never mentions WASM.

Pages checked: the site index, and for each book the index and first pages
of both the plain and the interactive variant. Only the text a reader sees is
checked, so a link to the /wasm/ book variant is allowed while its label
reads "Read the book with live examples".

Usage:
    python3 scripts/check_main_pages.py
"""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BOOKS = ["bevy_gradmesh", "bevy_composable_dragging"]
BOOK_FIRST_PAGES = ["index.html", "introduction.html"]
FORBIDDEN_TERMS = ["wasm", "webassembly", "interactive", "gallery", "demo"]
HIDDEN_BLOCKS = re.compile(r"<(script|style)\b.*?</\1>", re.IGNORECASE | re.DOTALL)
TAGS = re.compile(r"<[^>]*>")


def main_pages():
    pages = [REPO_ROOT / "index.html"]
    for book in BOOKS:
        for variant_dir in [REPO_ROOT / book, REPO_ROOT / book / "wasm"]:
            for name in BOOK_FIRST_PAGES:
                pages.append(variant_dir / name)
    return pages


def visible_text(html):
    without_blocks = HIDDEN_BLOCKS.sub("", html)
    return TAGS.sub(" ", without_blocks)


def forbidden_terms_in(path):
    text = visible_text(path.read_text(encoding="utf-8")).lower()
    return [term for term in FORBIDDEN_TERMS if term in text]


def main():
    failures = []
    for page in main_pages():
        if not page.exists():
            failures.append(f"{page.relative_to(REPO_ROOT)}: page is missing, run make build-wasm")
            continue
        for term in forbidden_terms_in(page):
            failures.append(f'{page.relative_to(REPO_ROOT)}: mentions "{term}"')
    for failure in failures:
        print(failure)
    if failures:
        sys.exit(1)
    print(f"OK: {len(main_pages())} main pages are free of WASM mentions")


if __name__ == "__main__":
    main()
