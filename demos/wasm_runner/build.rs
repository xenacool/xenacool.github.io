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
        "gradient_hanoi_3d", "gradient_kanban_2d",
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

    // Generate gallery manifest JSON for the web gallery page
    // This ensures the gallery always matches what compiles
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let manifest_path = root.join("demos/wasm/examples.json");
    
    let mut examples: Vec<(&str, &str, String, String)> = Vec::new();
    for name in gradmesh {
        examples.push((
            name,
            "bevy_gradmesh",
            format_title(name),
            format!("https://github.com/xenacool/bevy_gradmesh/tree/main/examples/{}.rs", name),
        ));
    }
    for name in dragging {
        examples.push((
            name,
            "bevy_composable_dragging",
            format_title(name),
            format!("https://github.com/xenacool/bevy_composable_dragging/tree/main/examples/{}.rs", name),
        ));
    }
    
    let json = build_json(&examples);
    fs::write(&manifest_path, json).unwrap_or_else(|e| {
        eprintln!("cargo:warning=Failed to write gallery manifest: {}", e);
    });

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

fn format_title(name: &str) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => {
                    let upper = first.to_ascii_uppercase();
                    format!("{}{}", upper, c.as_str().to_ascii_lowercase())
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_json(examples: &[(&str, &str, String, String)]) -> String {
    let mut out = String::from("[\n");
    for (i, (name, lib, title, source)) in examples.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str("  {\"name\":\"");
        out.push_str(name);
        out.push_str("\",\"lib\":\"");
        out.push_str(lib);
        out.push_str("\",\"title\":\"");
        out.push_str(title);
        out.push_str("\",\"source\":\"");
        out.push_str(source);
        out.push('"');
        out.push('}');
    }
    out.push_str("\n]");
    out
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

    while i < lines.len() {
        let line = lines[i];
        let stripped = line.trim();

        // 1. Handle new pattern: app.add_plugins(common::default_plugins("name"))
        //    followed by chained .add_plugins() calls.
        //    Replace the default_plugins call with just "app" so the chain continues.
        if stripped.starts_with("app.add_plugins(common::default_plugins(") {
            // Find the end of this add_plugins call (matching parens)
            let mut depth = 0;
            let mut end_line = i;
            for ch in lines[i].chars() {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                }
            }
            if depth <= 0 {
                // Single line: app.add_plugins(common::default_plugins("name"))
                // Check if it ends with ; (standalone) or chains to next line
                if stripped.ends_with(';') {
                    // Standalone statement - remove entirely
                    i += 1;
                    continue;
                } else {
                    // Chains to next line - replace with "app" to keep the chain
                    let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
                    result.push(format!("{}app", indent));
                    i += 1;
                    continue;
                }
            } else {
                // Multi-line: find the closing paren
                end_line = i;
                while end_line < lines.len() && depth > 0 {
                    end_line += 1;
                    for ch in lines[end_line].chars() {
                        if ch == '(' {
                            depth += 1;
                        } else if ch == ')' {
                            depth -= 1;
                        }
                    }
                }
                if depth <= 0 {
                    // Check what comes after
                    let mut next_i = end_line + 1;
                    while next_i < lines.len() && lines[next_i].trim().is_empty() {
                        next_i += 1;
                    }
                    if next_i < lines.len() && lines[next_i].trim().starts_with('.') {
                        // Chains to next line - replace with "app"
                        let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
                        result.push(format!("{}app", indent));
                    }
                    // else standalone - skip entirely
                    i = next_i;
                    continue;
                }
            }
        }

        // 2. Handle old pattern: "app" on its own line followed by .add_plugins(DefaultPlugins)
        if stripped == "app" {
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
                                break;
                            } else if next_i < lines.len() && lines[next_i].contains(';') {
                                i = next_i + 1;
                                break;
                            }
                        }
                    }
                    continue;
                }
            }
        }

        // 3. Strip AutoScreenshotPlugin calls (standalone or chained)
        if stripped.contains(".add_plugins(") && stripped.contains("AutoScreenshotPlugin") {
            // Skip this entire .add_plugins(AutoScreenshotPlugin...) block
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
                if started && depth <= 0 {
                    // Check if next line continues the chain
                    let mut next_i = i;
                    while next_i < lines.len() && lines[next_i].trim().is_empty() {
                        next_i += 1;
                    }
                    if next_i < lines.len() && lines[next_i].trim().starts_with('.') {
                        // Chain continues, keep going
                    } else {
                        // Statement ends here
                        if next_i < lines.len() && lines[next_i].contains(';') {
                            i = next_i + 1;
                        }
                    }
                    break;
                }
            }
            continue;
        }

        // 4. Strip AutoScreenshotPlugin on next line after .add_plugins(
        if stripped.contains(".add_plugins(") && !stripped.contains("AutoScreenshotPlugin") {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() && lines[j].contains("AutoScreenshotPlugin") {
                // Skip from current line through the closing paren and semicolon
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
                    if started && depth <= 0 {
                        let mut next_i = i;
                        while next_i < lines.len() && lines[next_i].trim().is_empty() {
                            next_i += 1;
                        }
                        if next_i < lines.len() && lines[next_i].trim().starts_with('.') {
                            // Chain continues
                        } else if next_i < lines.len() && lines[next_i].contains(';') {
                            i = next_i + 1;
                        }
                        break;
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
