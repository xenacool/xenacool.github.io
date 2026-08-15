use image::{DynamicImage, GenericImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[path = "src/spritestack_processing.rs"]
mod spritestack_processing;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SpriteRegion {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct SpriteAtlas {
    width: u32,
    height: u32,
    spritestacks: HashMap<String, Vec<SpriteRegion>>,
}

fn main() {
    // Cargo reruns this script when either the generator or its source assets change.
    // The Makefile also forces a rerun when generated web outputs are missing.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/spritestack_processing.rs");
    println!("cargo:rerun-if-changed=../../assets/spritestacks");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("Cargo sets manifest directory");
    let root_dir = Path::new(&manifest_dir)
        .parent()
        .expect("manifest directory has a parent")
        .parent()
        .expect("crate directory has a workspace parent");
    let web_dir = root_dir.join("web");

    if !web_dir.exists() {
        std::fs::create_dir_all(&web_dir).expect("create generated web directory");
    }

    generate_favicon(&web_dir.join("favicon.ico"));

    let assets_dir = root_dir.join("assets").join("spritestacks");
    if !assets_dir.exists() {
        println!(
            "cargo:warning=Assets directory not found at {}",
            assets_dir.display()
        );
        return;
    }

    let Some((char_images, max_tile_w, max_tile_h, total_tiles)) =
        collect_character_images(&assets_dir)
    else {
        println!(
            "cargo:warning=No character sprites found in {}",
            assets_dir.display()
        );
        return;
    };

    let mut atlas = SpriteAtlas::default();

    if total_tiles == 0 {
        println!("cargo:warning=No tiles to pack");
        return;
    }

    // Grid packing: keep width within 4096 pixels
    let columns = 4096u32.checked_div(max_tile_w).unwrap_or(1);
    let columns = columns.max(1).min(total_tiles as u32);
    let rows = (total_tiles as f32 / columns as f32).ceil() as u32;

    let atlas_width = columns * max_tile_w;
    let atlas_height = rows * max_tile_h;

    let mut spritesheet = RgbaImage::new(atlas_width, atlas_height);
    let mut current_tile = 0;

    atlas.width = atlas_width;
    atlas.height = atlas_height;

    for (name, layers) in char_images {
        let mut regions = Vec::new();
        for img in layers {
            let w = img.width();
            let h = img.height();

            let x = (current_tile % columns) * max_tile_w;
            let y = (current_tile / columns) * max_tile_h;

            spritesheet
                .copy_from(&img, x, y)
                .expect("Failed to copy image to spritesheet");
            regions.push(SpriteRegion { x, y, w, h });
            current_tile += 1;
        }

        atlas.spritestacks.insert(name, regions);
    }

    spritesheet
        .save(web_dir.join("spritesheet.png"))
        .expect("Failed to save spritesheet");

    let atlas_json = serde_json::to_string_pretty(&atlas).expect("atlas must serialize");
    std::fs::write(web_dir.join("atlas.json"), atlas_json).expect("Failed to write atlas.json");

    println!(
        "cargo:warning=Generated spritesheet and atlas in {}",
        web_dir.display()
    );
}

fn collect_character_images(
    assets_dir: &Path,
) -> Option<(Vec<(String, Vec<RgbaImage>)>, u32, u32, usize)> {
    let mut characters: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for entry in WalkDir::new(assets_dir) {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "png") {
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        let Some(char_name) = parent.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        characters
            .entry(char_name.to_owned())
            .or_default()
            .push(path.to_path_buf());
    }
    if characters.is_empty() {
        return None;
    }

    for layers in characters.values_mut() {
        layers.sort_by_key(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .replace("layer-", "")
                .parse::<u32>()
                .unwrap_or(0)
        });
    }

    let mut max_tile_w = 0u32;
    let mut max_tile_h = 0u32;
    let mut total_tiles = 0;
    let mut char_images = Vec::new();
    let mut names: Vec<_> = characters.keys().cloned().collect();
    names.sort();
    for name in names {
        let mut layers_out = Vec::new();
        for layer_path in &characters[&name] {
            let mut image = match image::open(layer_path) {
                Ok(image) => image.to_rgba8(),
                Err(error) => {
                    eprintln!("failed to open {}: {error}", layer_path.display());
                    std::process::exit(1);
                }
            };
            spritestack_processing::process_slice(
                image.width() as usize,
                image.height() as usize,
                image.as_mut(),
                spritestack_processing::SpritestackProcessConfig::default(),
            )
            .unwrap_or_else(|error| {
                eprintln!("failed to process {}: {error}", layer_path.display());
                std::process::exit(1);
            });
            max_tile_w = max_tile_w.max(image.width());
            max_tile_h = max_tile_h.max(image.height());
            layers_out.push(image);
        }
        total_tiles += layers_out.len();
        char_images.push((name, layers_out));
    }
    Some((char_images, max_tile_w, max_tile_h, total_tiles))
}

fn generate_favicon(path: &Path) {
    const SIZE: u32 = 32;
    let mut image: RgbaImage = ImageBuffer::from_pixel(SIZE, SIZE, Rgba([9, 14, 24, 255]));

    // A small deterministic hex gate: the silhouette matches the game's
    // layered hex board while remaining legible at browser favicon sizes.
    let background = Rgba([9, 14, 24, 255]);
    let edge = Rgba([61, 220, 190, 255]);
    let gate = Rgba([226, 244, 240, 255]);
    let opening = Rgba([28, 43, 58, 255]);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x.cast_signed() - 16;
            let dy = y.cast_signed() - 16;
            let ax = dx.abs();
            let ay = dy.abs();
            let inside = ax <= 12 && ay <= 10 && ax + ay / 2 <= 15;
            if !inside {
                image.put_pixel(x, y, background);
                continue;
            }
            let border = ax >= 11 || ay >= 9 || ax + ay / 2 >= 14;
            let gate_open = ax <= 4 && (1..=8).contains(&ay);
            image.put_pixel(
                x,
                y,
                if border {
                    edge
                } else if gate_open {
                    opening
                } else {
                    gate
                },
            );
        }
    }

    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut encoded, ImageFormat::Ico)
        .expect("failed to encode generated favicon");
    let encoded = encoded.into_inner();
    let unchanged = std::fs::read(path).is_ok_and(|existing| existing == encoded);
    if !unchanged {
        std::fs::write(path, encoded).expect("failed to write generated favicon");
    }
}
