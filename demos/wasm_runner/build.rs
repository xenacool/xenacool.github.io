// build.rs — Auto-generates example wrapper modules at compile time.
// Replaces gen_examples.py. Runs automatically on `cargo build`.

use std::path::Path;
use std::fs;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let examples_dir = Path::new(&out_dir).join("examples");
    fs::create_dir_all(&examples_dir).unwrap();

    // Project root is ../../ from wasm_runner/
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let libs = root.join("libs");

    let gradmesh: &[&str] = &[
        "appearance", "arbitrary_shapes", "character_creator", "composed_shapes",
        "curves", "deformation", "gradient_2d", "immediate", "instanced",
        "mix_spaces", "pbr_effects", "polyline", "retained", "road",
        "sdf_2d", "sdf_dash_gallery", "shapes", "standalone", "styles",
        "sweep", "terrain",
    ];

    let dragging: &[&str] = &[
        "blend_square", "chain_of_cells", "clock", "gizmo_3d",
        "gradient_hanoi_3d", "gradient_kanban_2d", "hello",
        "lists_in_lists", "reorder_list", "ring_of_beads", "scheduler",
        "slider", "tactile_tessellations", "tessellations", "three_way", "toggle",
    ];

    let mut mod_lines = vec![
        "// Auto-generated at compile time by build.rs.\n".to_string(),
        "use bevy::prelude::*;\n".to_string(),
        "".to_string(),
    ];

    let mut dispatch_matches = Vec::new();

    for name in gradmesh {
        let example_path = libs.join(format!("bevy_gradmesh/examples/{}.rs", name));
        let wrapper = generate_wrapper("bevy_gradmesh", name, &example_path);
        let mod_name = format!("gm_{}", name);
        mod_lines.push(format!("pub mod {};\n", mod_name));
        dispatch_matches.push(format!("        \"{}\" => {}::run(app),", name, mod_name));
        fs::write(examples_dir.join(format!("{}.rs", mod_name)), wrapper).unwrap();
    }

    for name in dragging {
        let example_path = libs.join(format!("bevy_composable_dragging/examples/{}.rs", name));
        let wrapper = generate_wrapper("bevy_composable_dragging", name, &example_path);
        let mod_name = format!("cd_{}", name);
        mod_lines.push(format!("pub mod {};\n", mod_name));
        dispatch_matches.push(format!("        \"{}\" => {}::run(app),", name, mod_name));
        fs::write(examples_dir.join(format!("{}.rs", mod_name)), wrapper).unwrap();
    }

    // Dispatch function
    mod_lines.push("/// Dispatch to the named example\n".to_string());
    mod_lines.push("pub fn dispatch(name: &str, app: &mut App) {\n".to_string());
    mod_lines.push("    match name {\n".to_string());
    for m in &dispatch_matches {
        mod_lines.push(format!("{}\n", m));
    }
    mod_lines.push("        _ => {\n".to_string());
    mod_lines.push("            bevy::log::warn!(\"Unknown example: {}. Showing shapes.\", name);\n".to_string());
    mod_lines.push("            gm_shapes::run(app);\n".to_string());
    mod_lines.push("        }\n".to_string());
    mod_lines.push("    }\n".to_string());
    mod_lines.push("}\n".to_string());

    fs::write(examples_dir.join("mod.rs"), mod_lines.join("")).unwrap();

    // Tell Cargo to re-run build.rs if any example file changes
    for name in gradmesh {
        println!(
            "cargo:rerun-if-changed={}/bevy_gradmesh/examples/{}.rs",
            libs.display(),
            name
        );
    }
    for name in dragging {
        println!(
            "cargo:rerun-if-changed={}/bevy_composable_dragging/examples/{}.rs",
            libs.display(),
            name
        );
    }

    println!("cargo:warning=Generated {} example wrappers", gradmesh.len() + dragging.len());
}

fn generate_wrapper(lib: &str, name: &str, example_path: &Path) -> String {
    let text = fs::read_to_string(example_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", example_path.display(), e));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let common_dir = root.join(format!("libs/{}/examples/common", lib));

    // Read all .rs files in common/ except mod.rs and inline them as submodules
    let entries: Vec<_> = fs::read_dir(&common_dir)
        .unwrap_or_else(|e| panic!("Failed to read common dir: {}", e))
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            let ext = path.extension().unwrap_or_default();
            ext == "rs" && path.file_name().unwrap_or_default() != "mod.rs"
        })
        .collect();

    let mut common_inner = String::new();
    for entry in &entries {
        let file_name = entry.file_name();
        let module_name = file_name
            .to_string_lossy()
            .trim_end_matches(".rs")
            .to_string();
        let content = fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", entry.path().display(), e));
        // Inline the file content as a submodule
        common_inner.push_str(&format!("pub mod {} {{\n{}\n}}\n", module_name, content));
    }

    // Read mod.rs and strip all mod declarations, inner doc comments, and inner attributes
    let mod_rs_path = common_dir.join("mod.rs");
    let mod_rs_text = fs::read_to_string(&mod_rs_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", mod_rs_path.display(), e));
    let mod_lines: Vec<&str> = mod_rs_text.lines().collect();
    let mut mod_rs_cleaned = Vec::new();
    let mut mi = 0;
    while mi < mod_lines.len() {
        let ml = mod_lines[mi];
        let ms = ml.trim();
        if ms.starts_with("//!") || ms.starts_with("/*!") || ms.starts_with("*/")
            || ms.starts_with("#![")
            || (ms.starts_with("pub mod ") && ms.ends_with(';'))
            || (ms.starts_with("mod ") && ms.ends_with(';'))
        {
            // Skip #[cfg(test)] if it precedes a mod declaration
            if ms == "#[cfg(test)]" {
                let mut peek = mi + 1;
                while peek < mod_lines.len() && mod_lines[peek].trim().is_empty() {
                    peek += 1;
                }
                if peek < mod_lines.len() {
                    let next = mod_lines[peek].trim();
                    if (next.starts_with("pub mod ") || next.starts_with("mod ")) && next.ends_with(';') {
                        mi += 1;
                        continue;
                    }
                }
            } else {
                mi += 1;
                continue;
            }
        }
        mod_rs_cleaned.push(ml.to_string());
        mi += 1;
    }
    let mod_rs_cleaned = mod_rs_cleaned.join("\n");

    let cleaned = strip_mod_common_and_main(&text);
    let cleaned = strip_add_plugins(&cleaned);
    format!(
        "// Auto-generated wrapper for {}/{}\n\
         mod common {{\n\
         {}\n\
         {}\n\
         }}\n\
         {}",
        lib, name, common_inner, mod_rs_cleaned, cleaned
    )
}

fn strip_mod_common_and_main(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut skip_main = false;
    let mut main_depth = 0;
    let mut skip_test = false;
    let mut test_depth = 0;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let stripped = line.trim();

        // Skip #[path = "..."] mod common; pattern
        if stripped.starts_with("#[path = ") && i + 1 < lines.len() {
            if lines[i + 1].trim() == "mod common;" {
                i += 2;
                continue;
            }
        }

        // Skip mod common;
        if stripped == "mod common;" {
            i += 1;
            continue;
        }

        // Skip inner doc comments (//!) left dangling after mod common removal
        if stripped.starts_with("//!") || stripped == "/*!" {
            i += 1;
            continue;
        }

        // Skip inner attributes like #![cfg_attr(...)]
        if stripped.starts_with("#![") {
            let mut depth = 0;
            let mut j = i;
            while j < lines.len() {
                for ch in lines[j].chars() {
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                    }
                }
                j += 1;
                if depth <= 0 {
                    break;
                }
            }
            i = j;
            continue;
        }

        // Skip fn main() { ... }
        if !skip_main && stripped.starts_with("fn main") {
            skip_main = true;
            main_depth = 0;
            for ch in line.chars() {
                if ch == '{' {
                    main_depth += 1;
                } else if ch == '}' {
                    main_depth -= 1;
                }
            }
            if main_depth <= 0 {
                skip_main = false;
            }
            i += 1;
            continue;
        }
        if skip_main {
            for ch in line.chars() {
                if ch == '{' {
                    main_depth += 1;
                } else if ch == '}' {
                    main_depth -= 1;
                }
            }
            if main_depth <= 0 {
                skip_main = false;
            }
            i += 1;
            continue;
        }

        // Skip #[cfg(test)] mod tests { ... }
        if !skip_test && stripped == "#[cfg(test)]" {
            if i + 1 < lines.len() && lines[i + 1].trim().starts_with("mod tests") {
                skip_test = true;
                test_depth = 0;
                for ch in lines[i + 1].chars() {
                    if ch == '{' {
                        test_depth += 1;
                    } else if ch == '}' {
                        test_depth -= 1;
                    }
                }
                i += 2;
                if test_depth <= 0 {
                    skip_test = false;
                }
                continue;
            }
        }
        if skip_test {
            for ch in line.chars() {
                if ch == '{' {
                    test_depth += 1;
                } else if ch == '}' {
                    test_depth -= 1;
                }
            }
            if test_depth <= 0 {
                skip_test = false;
            }
            i += 1;
            continue;
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n")
}

fn strip_add_plugins(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut stripped_default = false;

    while i < lines.len() {
        let line = lines[i];
        let stripped = line.trim();

        // 1. Strip DefaultPlugins chain: "app" followed by ".add_plugins(" with "DefaultPlugins"
        if !stripped_default && stripped == "app" {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() && lines[j].contains(".add_plugins(") {
                let mut k = j;
                let mut found_default = false;
                while k < (j + 5).min(lines.len()) {
                    if lines[k].contains("DefaultPlugins") {
                        found_default = true;
                        break;
                    }
                    k += 1;
                }
                if found_default {
                    result.push(line.to_string()); // Keep the 'app' line
                    i += 1;
                    let mut depth = 0;
                    let mut started = false;
                    while i < lines.len() {
                        for ch in lines[i].chars() {
                            if ch == '(' {
                                depth += 1;
                                started = true;
                            } else if ch == ')' {
                                depth -= 1;
                            }
                        }
                        i += 1;
                        if started && depth == 0 {
                            let mut next_i = i;
                            while next_i < lines.len() && lines[next_i].trim().is_empty() {
                                next_i += 1;
                            }
                            if next_i < lines.len()
                                && (lines[next_i].trim().starts_with(".add_plugins")
                                    || lines[next_i].trim().starts_with(".add_systems")
                                    || lines[next_i].trim().starts_with(".add_event"))
                            {
                                stripped_default = true;
                                break;
                            } else if next_i < lines.len() && lines[next_i].contains(';') {
                                stripped_default = true;
                                i = next_i + 1;
                                break;
                            }
                        }
                    }
                    continue;
                }
            }
        }

        // 2. Strip AutoScreenshotPlugin
        if stripped.contains(".add_plugins(") {
            let has_auto = stripped.contains("AutoScreenshotPlugin")
                || (i + 1 < lines.len() && lines[i + 1].contains("AutoScreenshotPlugin"));
            let has_default_plugins = stripped.contains("default_plugins")
                || (i + 1 < lines.len() && lines[i + 1].contains("default_plugins"));

            if has_auto || has_default_plugins {
                let mut depth = 0;
                while i < lines.len() {
                    for ch in lines[i].chars() {
                        if ch == '(' {
                            depth += 1;
                        } else if ch == ')' {
                            depth -= 1;
                        }
                    }
                    i += 1;
                    if depth <= 0 {
                        let mut next_i = i;
                        while next_i < lines.len() && lines[next_i].trim().is_empty() {
                            next_i += 1;
                        }
                        if next_i < lines.len() && lines[next_i].trim().starts_with(".add_") {
                            break;
                        } else if next_i < lines.len() && lines[next_i].contains(';') {
                            i = next_i + 1;
                            break;
                        } else {
                            break;
                        }
                    }
                }
                continue;
            }
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n")
}
