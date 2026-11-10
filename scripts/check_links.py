#!/usr/bin/env python3
"""
Link checker for xenacool.github.io

Uses Playwright to check all links and images in the built mdBook docs
and README files against the live GitHub Pages site.

Usage:
    python3 scripts/check_links.py [--local] [--port 8000]

Options:
    --local    Check against local server instead of live site
    --port     Port for local server (default: 8000)
"""

import os
import re
import sys
import argparse
import subprocess
from pathlib import Path
from urllib.parse import urljoin, urlparse
from html.parser import HTMLParser

# Base URLs
LIVE_BASE = "https://xenacool.github.io"
BOOK_PATHS = {
    "bevy_composable_dragging": "/bevy_composable_dragging/",
    "bevy_gradmesh": "/bevy_gradmesh/",
}

class HTMLLinkExtractor(HTMLParser):
    """Extract all links and image sources from HTML."""
    
    def __init__(self):
        super().__init__()
        self.links = []
        self.images = []
        
    def handle_starttag(self, tag, attrs):
        attrs_dict = dict(attrs)
        if tag == 'a' and 'href' in attrs_dict:
            self.links.append(attrs_dict['href'])
        elif tag == 'img' and 'src' in attrs_dict:
            self.images.append(attrs_dict['src'])

def extract_links_from_html(html_file):
    """Extract all links and images from an HTML file."""
    extractor = HTMLLinkExtractor()
    try:
        with open(html_file, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        extractor.feed(content)
    except Exception as e:
        print(f"  Error reading {html_file}: {e}")
    return extractor.links, extractor.images

def extract_links_from_markdown(md_file):
    """Extract links and images from markdown files."""
    links = []
    images = []
    try:
        with open(md_file, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        # Markdown links: [text](url)
        for match in re.finditer(r'\[([^\]]+)\]\(([^)]+)\)', content):
            text, url = match.groups()
            if text.startswith('!'):
                images.append(url)
            else:
                links.append(url)
    except Exception as e:
        print(f"  Error reading {md_file}: {e}")
    return links, images

def resolve_url(base, link):
    """Resolve a relative link against a base URL."""
    if link.startswith('#'):
        return None  # Fragment-only, skip
    if link.startswith('mailto:') or link.startswith('tel:'):
        return None  # Protocol links, skip
    if link.startswith('javascript:'):
        return None  # JS links, skip
    return urljoin(base, link)

def collect_all_links(base_url, local_mode=False):
    """Collect all links from built books and READMEs."""
    all_links = []
    all_images = []
    
    # Check built book HTML files
    for book_name, book_path in BOOK_PATHS.items():
        book_dir = Path(book_name)
        if not book_dir.exists():
            print(f"  Warning: {book_name}/ not found. Run 'make build' first.")
            continue
        
        base = f"{base_url}{book_path}"
        
        # Find all HTML files
        for html_file in book_dir.rglob('*.html'):
            links, images = extract_links_from_html(html_file)
            for link in links:
                url = resolve_url(base, link)
                if url:
                    all_links.append((url, str(html_file)))
            for img in images:
                url = resolve_url(base, img)
                if url:
                    all_images.append((url, str(html_file)))
    
    # Check README files in submodules
    for submodule in ['libs/bevy_composable_dragging', 'libs/bevy_gradmesh']:
        readme = Path(submodule) / 'README.md'
        if readme.exists():
            links, images = extract_links_from_markdown(readme)
            base = f"{base_url}/"
            for link in links:
                url = resolve_url(base, link)
                if url:
                    all_links.append((url, str(readme)))
            for img in images:
                url = resolve_url(base, img)
                if url:
                    all_images.append((url, str(readme)))
    
    # Check root README
    root_readme = Path('README.md')
    if root_readme.exists():
        links, images = extract_links_from_markdown(root_readme)
        base = f"{base_url}/"
        for link in links:
            url = resolve_url(base, link)
            if url:
                all_links.append((url, str(root_readme)))
        for img in images:
            url = resolve_url(base, img)
            if url:
                all_images.append((url, str(root_readme)))
    
    return all_links, all_images

async def check_links_with_playwright(links, images):
    """Use Playwright to check if links and images load correctly."""
    try:
        from playwright.async_api import async_playwright
    except ImportError:
        print("Error: Playwright Python package not installed.")
        print("Install with: pip install playwright && playwright install chromium")
        sys.exit(1)
    
    broken_links = []
    broken_images = []
    total = len(links) + len(images)
    checked = 0
    
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context()
        
        # Check links
        for url, source in links:
            checked += 1
            try:
                response = await context.request.get(url, timeout=10000)
                if response.status >= 400:
                    broken_links.append((url, source, f"HTTP {response.status}"))
            except Exception as e:
                broken_links.append((url, source, str(e)))
            print(f"  [{checked}/{total}] Link: {url[:80]}...")
        
        # Check images
        for url, source in images:
            checked += 1
            try:
                response = await context.request.get(url, timeout=10000)
                if response.status >= 400:
                    broken_images.append((url, source, f"HTTP {response.status}"))
            except Exception as e:
                broken_images.append((url, source, str(e)))
            print(f"  [{checked}/{total}] Image: {url[:80]}...")
        
        await browser.close()
    
    return broken_links, broken_images

def check_links_simple(links, images):
    """Simple HTTP check without Playwright (fallback)."""
    import urllib.request
    import ssl
    
    broken_links = []
    broken_images = []
    total = len(links) + len(images)
    checked = 0
    
    # Create SSL context that doesn't verify (for local testing)
    ssl_context = ssl.create_default_context()
    
    # Check links
    for url, source in links:
        checked += 1
        try:
            req = urllib.request.Request(url, method='HEAD')
            req.add_header('User-Agent', 'Mozilla/5.0 (Link Checker)')
            response = urllib.request.urlopen(req, timeout=10, context=ssl_context)
            print(f"  [{checked}/{total}] OK {url[:80]}")
        except Exception as e:
            broken_links.append((url, source, str(e)))
            print(f"  [{checked}/{total}] FAIL {url[:80]} - {e}")
    
    # Check images
    for url, source in images:
        checked += 1
        try:
            req = urllib.request.Request(url, method='HEAD')
            req.add_header('User-Agent', 'Mozilla/5.0 (Link Checker)')
            response = urllib.request.urlopen(req, timeout=10, context=ssl_context)
            print(f"  [{checked}/{total}] OK {url[:80]}")
        except Exception as e:
            broken_images.append((url, source, str(e)))
            print(f"  [{checked}/{total}] FAIL {url[:80]} - {e}")
    
    return broken_links, broken_images

async def main():
    parser = argparse.ArgumentParser(description='Check links in mdBook docs')
    parser.add_argument('--local', action='store_true', help='Check against local server')
    parser.add_argument('--port', type=int, default=8000, help='Local server port')
    parser.add_argument('--playwright', action='store_true', help='Use Playwright (requires installation)')
    args = parser.parse_args()
    
    if args.local:
        base_url = f"http://localhost:{args.port}"
        print(f"Checking links against local server: {base_url}")
    else:
        base_url = LIVE_BASE
        print(f"Checking links against live site: {base_url}")
    
    print("\nCollecting links...")
    links, images = collect_all_links(base_url, args.local)
    
    # Deduplicate
    seen_links = set()
    unique_links = []
    for url, source in links:
        if url not in seen_links:
            seen_links.add(url)
            unique_links.append((url, source))
    links = unique_links
    
    seen_images = set()
    unique_images = []
    for url, source in images:
        if url not in seen_images:
            seen_images.add(url)
            unique_images.append((url, source))
    images = unique_images
    
    print(f"Found {len(links)} unique links and {len(images)} unique images")
    
    if not links and not images:
        print("No links or images found. Run 'make build' first.")
        return 0
    
    print("\nChecking links...")
    
    if args.playwright:
        broken_links, broken_images = await check_links_with_playwright(links, images)
    else:
        broken_links, broken_images = check_links_simple(links, images)
    
    # Report results
    print("\n" + "="*60)
    print("LINK CHECK RESULTS")
    print("="*60)
    
    if broken_links:
        print(f"\nBROKEN LINKS ({len(broken_links)}):")
        for url, source, error in broken_links:
            print(f"  {url}")
            print(f"    Source: {source}")
            print(f"    Error: {error}")
    else:
        print("\nAll links are working!")
    
    if broken_images:
        print(f"\nBROKEN IMAGES ({len(broken_images)}):")
        for url, source, error in broken_images:
            print(f"  {url}")
            print(f"    Source: {source}")
            print(f"    Error: {error}")
    else:
        print("All images are loading!")
    
    total_broken = len(broken_links) + len(broken_images)
    print(f"\nTotal: {len(links) + len(images)} checked, {total_broken} broken")
    
    return 1 if total_broken > 0 else 0

if __name__ == '__main__':
    import asyncio
    result = asyncio.run(main())
    sys.exit(result)
