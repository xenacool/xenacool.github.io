#!/usr/bin/env python3
"""
Post-processor for mdBook HTML output.

Replaces static screenshot <img> tags with live <canvas> elements
and injects the Bevy WASM runner script and CSS.

Usage:
    python3 scripts/postprocess_book.py <book_dir> <lib_name>

Arguments:
    book_dir   Path to the built mdBook HTML directory
    lib_name   Library name (bevy_gradmesh or bevy_composable_dragging)
"""

import os
import re
import sys
import argparse
from pathlib import Path
from html.parser import HTMLParser
from typing import List, Tuple, Optional


class HTMLProcessor(HTMLParser):
    """Parse and modify HTML to replace screenshots with canvases."""

    def __init__(self):
        super().__init__()
        self.output = []
        self.in_head = False
        self.in_body = False
        self.injected_script = False
        self.injected_css = False
        self.injected_init_call = False
        self.lib_name = None
        self.depth = 2  # Default: book at repo root, e.g. bevy_gradmesh/

    def set_lib_name(self, lib_name: str):
        self.lib_name = lib_name

    def set_depth(self, depth: int):
        self.depth = depth

    def handle_starttag(self, tag: str, attrs: list):
        if tag == 'head':
            self.in_head = True
        elif tag == 'body':
            self.in_body = True

        # Process img tags that point to screenshots
        if tag == 'img':
            attrs_dict = dict(attrs)
            src = attrs_dict.get('src', '')

            # Check if this is a screenshot image
            if 'screenshot' in src.lower() or 'screenshots/' in src.lower():
                # Extract example name from filename
                example_name = self._extract_example_name(src)
                if example_name:
                    # Replace with canvas element
                    canvas_html = self._create_canvas_element(example_name)
                    self.output.append(canvas_html)
                    return

        # If we're at the end of head, inject our script and CSS
        if self.in_head and tag == 'link' and not self.injected_css:
            # Inject after existing links
            pass

        # Reconstruct the original tag
        attrs_str = ''.join(f' {k}="{v}"' if v is not None else f' {k}' for k, v in attrs)
        self.output.append(f'<{tag}{attrs_str}>')

    def handle_endtag(self, tag: str):
        if tag == 'head':
            self.in_head = False
            # Inject CSS and script before closing head
            if not self.injected_css:
                prefix = '../' * self.depth
                self.output.append(
                    f'    <link rel="stylesheet" href="{prefix}css/book_canvas.css">'
                )
                self.injected_css = True
        elif tag == 'body':
            self.in_body = False
            # Inject init call before closing body
            if not self.injected_init_call:
                prefix = '../' * self.depth
                self.output.append(
                    f'    <script src="{prefix}js/bevy_runner.js"></script>\n'
                    f'    <script>BevyRunner.observe();</script>'
                )
                self.injected_init_call = True
        elif tag == 'img':
            pass  # Already handled in starttag

        self.output.append(f'</{tag}>')

    def handle_data(self, data: str):
        self.output.append(data)

    def handle_comment(self, data: str):
        self.output.append(f'<!-- {data} -->')

    def handle_entityref(self, name: str):
        self.output.append(f'&{name};')

    def handle_charref(self, name: str):
        self.output.append(f'&#{name};')

    def _extract_example_name(self, src: str) -> Optional[str]:
        """Extract example name from screenshot filename.

        Examples:
            screenshots/screenshot_shapes.png -> shapes
            screenshots/screenshot_hello.webp -> hello
            img/screenshot_toggle.png -> toggle
        """
        # Get the filename
        filename = Path(src).name

        # Remove screenshot_ prefix
        if filename.startswith('screenshot_'):
            filename = filename[11:]  # Remove 'screenshot_'

        # Remove extension
        name = Path(filename).stem

        return name if name else None

    def _create_canvas_element(self, example_name: str) -> str:
        """Create a canvas element for a Bevy example."""
        return f'''    <div class="bevy-example-container">
        <canvas
            class="bevy-example"
            data-lib="{self.lib_name}"
            data-example="{example_name}"
            data-width="1280"
            data-height="720"
            width="640"
            height="360">
        </canvas>
    </div>'''

    def get_output(self) -> str:
        return ''.join(self.output)


def process_html_file(html_file: Path, lib_name: str, depth: int = 2) -> bool:
    """Process a single HTML file, replacing screenshots with canvases.

    Returns True if any changes were made.
    """
    with open(html_file, 'r', encoding='utf-8') as f:
        content = f.read()

    # Check if file contains any screenshot references
    if 'screenshot' not in content.lower():
        return False

    processor = HTMLProcessor()
    processor.set_lib_name(lib_name)
    processor.set_depth(depth)
    processor.feed(content)

    new_content = processor.get_output()

    if new_content != content:
        # Inject the version toggle link in the sidebar
        prefix = '../' * depth
        toggle_html = (
            '\n            <div class="book-version-toggle">\n'
            '                <a href="../index.html" class="version-link" title="Switch to static version">\n'
            '                    📷 Static version\n'
            '                </a>\n'
            '            </div>'
        )
        new_content = new_content.replace(
            '<nav id="mdbook-sidebar" class="sidebar" aria-label="Table of contents">',
            '<nav id="mdbook-sidebar" class="sidebar" aria-label="Table of contents">' + toggle_html,
            1
        )
        
        with open(html_file, 'w', encoding='utf-8') as f:
            f.write(new_content)
        return True

    return False


def process_book_directory(book_dir: Path, lib_name: str, depth: int = 2) -> int:
    """Process all HTML files in a book directory.

    Returns the number of files modified.
    """
    modified_count = 0

    for html_file in book_dir.rglob('*.html'):
        if process_html_file(html_file, lib_name, depth):
            modified_count += 1
            print(f"  Modified: {html_file.relative_to(book_dir.parent)}")

    return modified_count


def remove_screenshots_directory(book_dir: Path) -> int:
    """Remove the screenshots directory from the built book output.

    Returns the number of files removed.
    """
    screenshots_dir = book_dir / 'img' / 'screenshots'
    if not screenshots_dir.exists():
        return 0

    removed_count = 0
    for file_path in screenshots_dir.rglob('*'):
        if file_path.is_file():
            file_path.unlink()
            removed_count += 1

    # Remove empty directories
    for dir_path in sorted(screenshots_dir.rglob('*'), reverse=True):
        if dir_path.is_dir():
            try:
                dir_path.rmdir()
            except OSError:
                pass  # Directory not empty

    return removed_count


def main():
    parser = argparse.ArgumentParser(description='Post-process mdBook HTML for WASM examples')
    parser.add_argument('book_dir', help='Path to the built mdBook HTML directory')
    parser.add_argument('lib_name', help='Library name (bevy_gradmesh or bevy_composable_dragging)')
    parser.add_argument('--keep-screenshots', action='store_true',
                       help='Keep screenshot files instead of removing them')
    parser.add_argument('--depth', type=int, default=2,
                       help='Directory depth for relative CSS/JS paths (default: 2 for root-level books, 3 for /wasm/ subdirs)')
    parser.add_argument('--static-toggle', action='store_true',
                       help='Only inject the WASM version toggle link, do not replace screenshots')
    args = parser.parse_args()

    book_dir = Path(args.book_dir)
    if not book_dir.exists():
        print(f"Error: Book directory {book_dir} does not exist")
        sys.exit(1)

    print(f"Processing {book_dir} for {args.lib_name} (depth={args.depth})...")
    
    if args.static_toggle:
        # Only inject the toggle link, don't replace screenshots
        modified = _inject_toggle_only(book_dir, args.lib_name, args.depth)
        print(f"Modified {modified} HTML files (toggle only)")
    else:
        modified = process_book_directory(book_dir, args.lib_name, args.depth)
        print(f"Modified {modified} HTML files")

        if not args.keep_screenshots:
            removed = remove_screenshots_directory(book_dir)
            if removed:
                print(f"Removed {removed} screenshot files")

    print("Done")


def _inject_toggle_only(book_dir: Path, lib_name: str, depth: int = 2) -> int:
    """Inject only the version toggle link and CSS into the sidebar, without replacing screenshots.
    
    Returns the number of files modified.
    """
    modified_count = 0
    prefix = '../' * depth
    
    for html_file in book_dir.rglob('*.html'):
        with open(html_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        if 'book-version-toggle' in content:
            continue
        
        # Inject the toggle link after the sidebar nav opening tag
        toggle_html = (
            '\n            <div class="book-version-toggle">\n'
            '                <a href="wasm/index.html" class="version-link" title="Switch to interactive WASM version">\n'
            '                    ⚡ WASM version\n'
            '                </a>\n'
            '            </div>'
        )
        
        # Inject CSS link before </head>
        css_link = f'    <link rel="stylesheet" href="{prefix}css/book_canvas.css">'
        
        new_content = content.replace('</head>', f'{css_link}\n</head>', 1)
        new_content = new_content.replace(
            '<nav id="mdbook-sidebar" class="sidebar" aria-label="Table of contents">',
            '<nav id="mdbook-sidebar" class="sidebar" aria-label="Table of contents">' + toggle_html,
            1
        )
        
        if new_content != content:
            with open(html_file, 'w', encoding='utf-8') as f:
                f.write(new_content)
            modified_count += 1
            print(f"  Modified: {html_file.relative_to(book_dir.parent)}")
    
    return modified_count


if __name__ == '__main__':
    main()
