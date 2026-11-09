#!/usr/bin/env python3
"""Generate wasm_runner/src/examples.rs and per-example wrapper modules.

Each example gets a wrapper module that:
1. Provides `mod common` via #[path] pointing to the library's common module
2. Copies the example file with `mod common;` and `fn main()` stripped out

The dispatch function matches example names to their run() functions.
"""

import re
import sys
from pathlib import Path

# All gradmesh examples (single-file, no subdirectory)
gradmesh_examples = [
    "appearance", "arbitrary_shapes", "character_creator", "composed_shapes",
    "curves", "deformation", "gradient_2d", "immediate", "instanced",
    "mix_spaces", "pbr_effects", "polyline", "retained", "road",
    "sdf_2d", "sdf_dash_gallery", "shapes", "standalone", "styles",
    "sweep", "terrain",
]

# All composable_dragging examples (single-file)
dragging_examples = [
    "blend_square", "chain_of_cells", "clock", "gizmo_3d",
    "gradient_hanoi_3d", "gradient_kanban_2d", "hello",
    "lists_in_lists", "reorder_list", "ring_of_beads", "scheduler",
    "slider", "tactile_tessellations", "tessellations", "three_way", "toggle",
]

# Multi-file demos excluded — they reference internal sub-modules that can't be easily included
# They can be built as separate WASM files later
multi_file_examples = []


def strip_mod_common_and_main(text: str) -> str:
    """Remove `mod common;`, `#[path = ...] mod common;`, `fn main() { ... }`,
    `#[cfg(test)] mod tests { ... }`, and leading inner doc comments from example text."""
    lines = text.split('\n')
    result = []
    skip_main = False
    main_depth = 0
    skip_test = False
    test_depth = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        # Skip #[path = "..."] mod common; pattern
        if stripped.startswith("#[path = ") and i + 1 < len(lines):
            if lines[i + 1].strip() == "mod common;":
                i += 2
                continue

        # Skip mod common;
        if stripped == "mod common;":
            i += 1
            continue

        # Skip inner doc comments (//!) that are left dangling after mod common removal
        if stripped.startswith("//!") or stripped == "/*!":
            i += 1
            continue

        # Skip inner attributes like #![cfg_attr(...)] that belong to the crate root
        if stripped.startswith("#!["):
            # Handle multi-line inner attributes
            depth = 0
            j = i
            while j < len(lines):
                for ch in lines[j]:
                    if ch == '(':
                        depth += 1
                    elif ch == ')':
                        depth -= 1
                j += 1
                if depth <= 0:
                    break
            i = j
            continue

        # Skip fn main() { ... }
        if not skip_main and re.match(r'^\s*fn main\s*\(', line):
            skip_main = True
            main_depth = 0
            # Count braces on this line
            for ch in line:
                if ch == '{':
                    main_depth += 1
                elif ch == '}':
                    main_depth -= 1
            if main_depth <= 0:
                skip_main = False
            i += 1
            continue
        if skip_main:
            for ch in line:
                if ch == '{':
                    main_depth += 1
                elif ch == '}':
                    main_depth -= 1
            if main_depth <= 0:
                skip_main = False
            i += 1
            continue

        # Skip #[cfg(test)] mod tests { ... }
        if not skip_test and stripped == "#[cfg(test)]":
            if i + 1 < len(lines) and lines[i + 1].strip().startswith("mod tests"):
                skip_test = True
                test_depth = 0
                # Count braces on the mod line
                for ch in lines[i + 1]:
                    if ch == '{':
                        test_depth += 1
                    elif ch == '}':
                        test_depth -= 1
                i += 2
                if test_depth <= 0:
                    skip_test = False
                continue
        if skip_test:
            for ch in line:
                if ch == '{':
                    test_depth += 1
                elif ch == '}':
                    test_depth -= 1
            if test_depth <= 0:
                skip_test = False
            i += 1
            continue

        result.append(line)
        i += 1

    return '\n'.join(result)


def strip_add_plugins(text: str) -> str:
    """Remove the `app.add_plugins(DefaultPlugins...` block and AutoScreenshotPlugin from pub fn run().
    
    The pattern is a chained expression like:
        app
            .add_plugins(
                DefaultPlugins
                    .set(WindowPlugin { ... })
                    .set(RenderPlugin { ... }),
            )
            .add_plugins(GradMeshPlugin)           <- KEEP
            .add_plugins(AutoScreenshotPlugin::new(..)) <- REMOVE (WASM incompatible)
            .add_systems(Startup, setup)            <- KEEP
        ;
    
    We remove DefaultPlugins (window conflict) and AutoScreenshotPlugin (file I/O in WASM).
    """
    lines = text.split('\n')
    result = []
    i = 0
    stripped_default = False
    
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        
        # 1. Strip DefaultPlugins chain: "app" followed by ".add_plugins(" with "DefaultPlugins"
        if not stripped_default and stripped == 'app':
            j = i + 1
            while j < len(lines) and lines[j].strip() == '':
                j += 1
            if j < len(lines) and '.add_plugins(' in lines[j]:
                k = j
                found_default = False
                while k < min(j + 5, len(lines)):
                    if 'DefaultPlugins' in lines[k]:
                        found_default = True
                        break
                    k += 1
                if found_default:
                    result.append(line)  # Keep the 'app' line
                    i += 1
                    depth = 0
                    started = False
                    while i < len(lines):
                        for ch in lines[i]:
                            if ch == '(':
                                depth += 1
                                started = True
                            elif ch == ')':
                                depth -= 1
                        i += 1
                        if started and depth == 0:
                            next_i = i
                            while next_i < len(lines) and lines[next_i].strip() == '':
                                next_i += 1
                            if next_i < len(lines) and (lines[next_i].strip().startswith('.add_plugins') or 
                                                         lines[next_i].strip().startswith('.add_systems') or
                                                         lines[next_i].strip().startswith('.add_event')):
                                stripped_default = True
                                break
                            elif next_i < len(lines) and ';' in lines[next_i]:
                                stripped_default = True
                                i = next_i + 1
                                break
                            else:
                                pass
                    continue
        
        # 2. Strip AutoScreenshotPlugin: ".add_plugins(" on same line or next line has "AutoScreenshotPlugin"
        if '.add_plugins(' in stripped:
            # Only match if AutoScreenshotPlugin is on the SAME line as .add_plugins(
            # or on the very next line (for multi-line patterns)
            if 'AutoScreenshotPlugin' in stripped or (i + 1 < len(lines) and 'AutoScreenshotPlugin' in lines[i + 1]):
                # Skip this chained .add_plugins(AutoScreenshotPlugin...) block
                depth = 0
                while i < len(lines):
                    for ch in lines[i]:
                        if ch == '(':
                            depth += 1
                        elif ch == ')':
                            depth -= 1
                    i += 1
                    if depth <= 0:
                        next_i = i
                        while next_i < len(lines) and lines[next_i].strip() == '':
                            next_i += 1
                        if next_i < len(lines) and lines[next_i].strip().startswith('.add_'):
                            break
                        elif next_i < len(lines) and ';' in lines[next_i]:
                            i = next_i + 1
                            break
                        else:
                            break
                continue
            # 3. Strip common::default_plugins() — it sets up window without canvas targeting
            elif 'default_plugins' in stripped or (i + 1 < len(lines) and 'default_plugins' in lines[i + 1]):
                depth = 0
                while i < len(lines):
                    for ch in lines[i]:
                        if ch == '(':
                            depth += 1
                        elif ch == ')':
                            depth -= 1
                    i += 1
                    if depth <= 0:
                        next_i = i
                        while next_i < len(lines) and lines[next_i].strip() == '':
                            next_i += 1
                        if next_i < len(lines) and lines[next_i].strip().startswith('.add_'):
                            break
                        elif next_i < len(lines) and ';' in lines[next_i]:
                            i = next_i + 1
                            break
                        else:
                            break
                continue
        
        result.append(line)
        i += 1
    
    return '\n'.join(result)


def generate_wrapper(lib: str, name: str, rel_path: str, example_text: str) -> str:
    """Generate a wrapper module for one example."""
    common_path = f"../../../../libs/{lib}/examples/common/mod.rs"
    cleaned = strip_mod_common_and_main(example_text)
    # Strip DefaultPlugins so main.rs controls the window with canvas targeting
    cleaned = strip_add_plugins(cleaned)
    return (
        f"// Auto-generated wrapper for {lib}/{name}\n"
        f"#[path = \"{common_path}\"]\n"
        f"mod common;\n"
        f"{cleaned}"
    )


def main():
    root = Path(__file__).resolve().parent
    examples_dir = root / "src" / "examples"
    examples_dir.mkdir(parents=True, exist_ok=True)

    # Generate per-example wrapper files
    all_examples = []

    project_root = root.parent.parent  # xenacool_github/
    libs = project_root / "libs"

    for name in gradmesh_examples:
        example_file = libs / "bevy_gradmesh" / "examples" / f"{name}.rs"
        text = example_file.read_text()
        wrapper = generate_wrapper("bevy_gradmesh", name, f"{name}.rs", text)
        (examples_dir / f"gm_{name}.rs").write_text(wrapper)
        all_examples.append(("gm_" + name, name, "bevy_gradmesh"))

    for name in dragging_examples:
        example_file = libs / "bevy_composable_dragging" / "examples" / f"{name}.rs"
        text = example_file.read_text()
        wrapper = generate_wrapper("bevy_composable_dragging", name, f"{name}.rs", text)
        (examples_dir / f"cd_{name}.rs").write_text(wrapper)
        all_examples.append(("cd_" + name, name, "bevy_composable_dragging"))

    for name, rel_path in multi_file_examples:
        example_file = libs / "bevy_composable_dragging" / "examples" / rel_path
        text = example_file.read_text()
        wrapper = generate_wrapper("bevy_composable_dragging", name, rel_path, text)
        (examples_dir / f"cd_{name}.rs").write_text(wrapper)
        all_examples.append(("cd_" + name, name, "bevy_composable_dragging"))

    # Generate examples.rs with module declarations and dispatch function
    lines = [
        "//! All examples as wrapper modules.",
        "//! Each wrapper provides `mod common` via #[path] before including the example file.",
        "use bevy::prelude::*;",
        "",
    ]

    # Module declarations
    for mod_name, _, lib in all_examples:
        lines.append(f"pub mod {mod_name};")

    lines.append("")
    lines.append("/// Dispatch to the named example")
    lines.append("pub fn dispatch(name: &str, app: &mut App) {")
    lines.append("    match name {")

    for mod_name, example_name, _ in all_examples:
        lines.append(f'        "{example_name}" => {mod_name}::run(app),')

    lines.append("        _ => {")
    lines.append('            bevy::log::warn!("Unknown example: {}. Showing shapes.", name);')
    lines.append("            gm_shapes::run(app);")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    (examples_dir / "mod.rs").write_text('\n'.join(lines))

    print(f"Generated {len(all_examples)} example wrappers in {examples_dir}")
    print(f"Generated dispatch function in {examples_dir / 'mod.rs'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
