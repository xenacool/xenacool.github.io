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
        "lerp_2d", "mix_spaces", "pbr_effects", "polyline", "retained", "road",
        "sdf_2d", "sdf_dash_gallery", "shapes", "standalone", "styles",
        "sweep", "terrain",
    ];

    let dragging: &[&str] = &[
        "blend_square", "chain_of_cells", "clock", "constellation",
        "dag", "equal_constraint", "gizmo_3d", "graspable_math",
        "gradient_hanoi_3d", "gradient_kanban_2d", "gradmesh_showcase",
        "lists_in_lists", "reorder_list", "ring_of_beads", "scheduler",
        "slider", "stickiness", "tactile_tessellations", "tessellations",
        "three_way", "toggle",
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
        common_inner.push_str(&format!("pub mod {} {{\n{}\n}}\n", module_name, content));
    }

    // Read mod.rs and keep only function bodies, stripping mod declarations, doc comments,
    // and #[cfg(test)] attributes
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
            || ms == "#[cfg(test)]"
            || (ms.starts_with("pub mod ") && ms.ends_with(';'))
            || (ms.starts_with("mod ") && ms.ends_with(';'))
        {
            mi += 1;
            continue;
        }
        mod_rs_cleaned.push(ml.to_string());
        mi += 1;
    }
    let mod_rs_cleaned = mod_rs_cleaned.join("\n");

    // Strip `mod common;` and `fn main()` from the example; keep everything else including `pub fn run`
    let cleaned = strip_mod_common_and_main(&text);
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

/// Remove `mod common;`, `fn main()`, and dangling inner doc comments from example source
/// so the wrapper can inline common/ and call `pub fn run(app)` directly.
fn strip_mod_common_and_main(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let stripped = lines[i].trim();

        // Skip `#[path = "..."] mod common;` two-line pattern
        if stripped.starts_with("#[path = ") && i + 1 < lines.len() {
            if lines[i + 1].trim() == "mod common;" {
                i += 2;
                continue;
            }
        }

        // Skip `mod common;`
        if stripped == "mod common;" {
            i += 1;
            continue;
        }

        // Skip dangling inner doc comments left after mod common removal
        if stripped.starts_with("//!") || stripped == "/*!" || stripped == "*/" {
            i += 1;
            continue;
        }

        // Skip inner attributes like #![cfg_attr(...)] that become dangling
        if stripped.starts_with("#![") {
            let mut depth = 0;
            let mut j = i;
            while j < lines.len() {
                for ch in lines[j].chars() {
                    if ch == '(' { depth += 1; }
                    else if ch == ')' { depth -= 1; }
                }
                if depth <= 0 && j >= i {
                    i = j + 1;
                    break;
                }
                j += 1;
            }
            if depth > 0 {
                i = lines.len();
            }
            continue;
        }

        // Skip `fn main() { ... }` by tracking brace depth
        if stripped.starts_with("fn main()") {
            let mut depth = 0;
            let mut j = i;
            while j < lines.len() {
                for ch in lines[j].chars() {
                    if ch == '{' { depth += 1; }
                    else if ch == '}' { depth -= 1; }
                }
                if depth == 0 && j > i {
                    i = j + 1;
                    break;
                }
                j += 1;
            }
            if depth > 0 {
                i = lines.len();
            }
            continue;
        }

        // Skip `#[cfg(test)] mod tests { ... }`
        if stripped == "#[cfg(test)]" && i + 1 < lines.len() {
            if lines[i + 1].trim().starts_with("mod tests") {
                let mut depth = 0;
                let mut j = i;
                while j < lines.len() {
                    for ch in lines[j].chars() {
                        if ch == '{' { depth += 1; }
                        else if ch == '}' { depth -= 1; }
                    }
                    if depth == 0 && j > i {
                        i = j + 1;
                        break;
                    }
                    j += 1;
                }
                if depth > 0 {
                    i = lines.len();
                }
                continue;
            }
        }

        result.push(lines[i].to_string());
        i += 1;
    }

    result.join("\n")
}
