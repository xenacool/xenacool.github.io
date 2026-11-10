#!/usr/bin/env python3
"""Check that book version references match the current Cargo.toml versions.

Exits 0 if all versions align, exits 1 with a clear message if any are out of date.
"""

import re
import sys
from pathlib import Path

def cargo_version(toml_path: Path) -> str:
    """Read the package version from a Cargo.toml file."""
    text = toml_path.read_text()
    for line in text.splitlines():
        m = re.match(r'^version\s*=\s*"([^"]+)"', line.strip())
        if m:
            return m.group(1)
    raise ValueError(f"No version found in {toml_path}")

def book_versions(md_files: list[Path]) -> list[tuple[str, Path, int]]:
    """Find all version = \"...\" references in book markdown files.

    Returns a list of (version_string, file_path, line_number) for each match.
    """
    results = []
    for md in md_files:
        text = md.read_text()
        for lineno, line in enumerate(text.splitlines(), 1):
            # Match version = "X.Y" or version = "X" in TOML code blocks
            for m in re.finditer(r'version\s*=\s*"([^"]+)"', line):
                results.append((m.group(1), md, lineno))
    return results

def major_minor(version: str) -> str:
    """Extract major.minor from a version string like '0.3.2' -> '0.3'."""
    parts = version.split('.')
    return '.'.join(parts[:2])

def check(lib_name: str, submodule_dir: Path) -> list[str]:
    """Check one library. Returns a list of error messages (empty if OK)."""
    errors = []
    cargo_toml = submodule_dir / "Cargo.toml"
    book_src = submodule_dir / "book" / "src"

    if not cargo_toml.exists():
        errors.append(f"{lib_name}: Cargo.toml not found at {cargo_toml}")
        return errors
    if not book_src.exists():
        errors.append(f"{lib_name}: book/src not found at {book_src}")
        return errors

    expected = major_minor(cargo_version(cargo_toml))
    md_files = sorted(book_src.glob("*.md"))
    refs = book_versions(md_files)

    for ref_ver, file_path, lineno in refs:
        ref_major = major_minor(ref_ver)
        if ref_major != expected:
            rel = file_path.relative_to(Path.cwd())
            errors.append(
                f"{lib_name}: {rel}:{lineno} has version \"{ref_ver}\", "
                f"but Cargo.toml is {expected}.*"
            )

    return errors

def main():
    root = Path(__file__).resolve().parent.parent
    libs = [
        ("bevy_composable_dragging", root / "libs" / "bevy_composable_dragging"),
        ("bevy_gradmesh", root / "libs" / "bevy_gradmesh"),
    ]

    all_errors = []
    for name, path in libs:
        all_errors.extend(check(name, path))

    if all_errors:
        print("Version mismatch: the book examples won't install the latest release:")
        print()
        for err in all_errors:
            print(f"  x {err}")
        print()
        print("Fix the version in the book markdown, then rebuild.")
        sys.exit(1)
    else:
        print("All book versions match their Cargo.toml files.")
        sys.exit(0)

if __name__ == "__main__":
    main()
