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

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let web_dir = root_dir.join("web");

    if !web_dir.exists() {
        std::fs::create_dir_all(&web_dir).unwrap();
    }

    generate_favicon(&web_dir.join("favicon.ico"));

    let assets_dir = root_dir.join("assets").join("spritestacks");
    if !assets_dir.exists() {
        println!(
            "cargo:warning=Assets directory not found at {:?}",
            assets_dir
        );
        return;
    }

    let mut atlas = SpriteAtlas::default();

    // Collect all character layers
    let mut characters: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for entry in WalkDir::new(&assets_dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "png") {
            let parent = entry.path().parent().unwrap();
            let char_name = parent.file_name().unwrap().to_str().unwrap().to_string();
            characters
                .entry(char_name)
                .or_default()
                .push(entry.path().to_path_buf());
        }
    }

    if characters.is_empty() {
        println!(
            "cargo:warning=No character sprites found in {:?}",
            assets_dir
        );
        return;
    }

    // Sort layers for consistency
    for layers in characters.values_mut() {
        layers.sort_by_key(|p| {
            p.file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .replace("layer-", "")
                .parse::<u32>()
                .unwrap_or(0)
        });
    }

    let mut max_tile_w = 0u32;
    let mut max_tile_h = 0u32;
    let mut char_images = Vec::new();

    let mut sorted_char_names: Vec<_> = characters.keys().cloned().collect();
    sorted_char_names.sort();

    let mut total_tiles = 0;
    for name in sorted_char_names {
        let layers = &characters[&name];
        let mut char_layer_images = Vec::new();
        for layer_path in layers {
            let mut img = image::open(layer_path).unwrap().to_rgba8();
            spritestack_processing::process_slice(
                img.width() as usize,
                img.height() as usize,
                img.as_mut(),
                Default::default(),
            )
            .unwrap_or_else(|error| panic!("Failed to process {:?}: {}", layer_path, error));
            max_tile_w = max_tile_w.max(img.width());
            max_tile_h = max_tile_h.max(img.height());
            char_layer_images.push(img);
        }
        total_tiles += char_layer_images.len();
        char_images.push((name, char_layer_images));
    }

    if total_tiles == 0 {
        println!("cargo:warning=No tiles to pack");
        return;
    }

    // Grid packing: keep width within 4096 pixels
    let columns = if max_tile_w > 0 { 4096 / max_tile_w } else { 1 };
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

    let atlas_json = serde_json::to_string_pretty(&atlas).unwrap();
    std::fs::write(web_dir.join("atlas.json"), atlas_json).expect("Failed to write atlas.json");

    println!(
        "cargo:warning=Generated spritesheet and atlas in {:?}",
        web_dir
    );
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
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            let ax = dx.abs();
            let ay = dy.abs();
            let inside = ax <= 12 && ay <= 10 && ax + ay / 2 <= 15;
            if !inside {
                image.put_pixel(x, y, background);
                continue;
            }
            let border = ax >= 11 || ay >= 9 || ax + ay / 2 >= 14;
            let gate_open = ax <= 4 && ay >= 1 && ay <= 8;
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
    let unchanged = std::fs::read(path)
        .map(|existing| existing == encoded)
        .unwrap_or(false);
    if !unchanged {
        std::fs::write(path, encoded).expect("failed to write generated favicon");
    }
}
