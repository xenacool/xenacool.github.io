use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{
    ColorType, DynamicImage, GenericImage, ImageBuffer, ImageEncoder, ImageFormat, Rgba, RgbaImage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_SPRITESTACK_RGB_BITS: u8 = 8;
const SPRITESTACK_ATLAS_PADDING: u32 = 1;

#[path = "src/spritestack_processing.rs"]
mod spritestack_processing;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SpriteRegion {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SpriteAnimation {
    frame_duration_ms: u32,
    #[serde(rename = "loop")]
    looped: bool,
    frames: Vec<Vec<u32>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct SpriteAtlas {
    width: u32,
    height: u32,
    spritestacks: HashMap<String, Vec<SpriteRegion>>,
    #[serde(default)]
    animations: HashMap<String, HashMap<String, SpriteAnimation>>,
}

fn main() {
    // Cargo reruns this script when either the generator or its source assets change.
    // The Makefile also forces a rerun when generated web outputs are missing.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/spritestack_processing.rs");
    println!("cargo:rerun-if-changed=../../assets/spritestacks");
    println!("cargo:rerun-if-env-changed=SPRITESTACK_RGB_BITS");

    let spritestack_rgb_bits = std::env::var("SPRITESTACK_RGB_BITS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(DEFAULT_SPRITESTACK_RGB_BITS);
    if !(1..=8).contains(&spritestack_rgb_bits) {
        panic!("SPRITESTACK_RGB_BITS must be between 1 and 8");
    }

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
        collect_character_images(&assets_dir, spritestack_rgb_bits)
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
    let packed_w = max_tile_w + SPRITESTACK_ATLAS_PADDING * 2;
    let packed_h = max_tile_h + SPRITESTACK_ATLAS_PADDING * 2;
    let columns = 4096u32.checked_div(packed_w).unwrap_or(1);
    let columns = columns.max(1).min(total_tiles as u32);
    let rows = (total_tiles as f32 / columns as f32).ceil() as u32;

    let atlas_width = columns * packed_w;
    let atlas_height = rows * packed_h;

    let mut spritesheet = RgbaImage::new(atlas_width, atlas_height);
    let mut current_tile = 0;

    atlas.width = atlas_width;
    atlas.height = atlas_height;

    for (name, layers) in char_images {
        let mut regions = Vec::new();
        for img in layers {
            let w = img.width();
            let h = img.height();

            let x = (current_tile % columns) * packed_w + SPRITESTACK_ATLAS_PADDING;
            let y = (current_tile / columns) * packed_h + SPRITESTACK_ATLAS_PADDING;

            spritesheet
                .copy_from(&img, x, y)
                .expect("Failed to copy image to spritesheet");
            // Extrude the edge pixels into the one-pixel gutter. This keeps
            // linear filtering and future outline passes from sampling the
            // neighboring slice while preserving antialiased color edges.
            for px in 0..w {
                let top = nearest_opaque_edge_pixel(&img, px, 0, true);
                let bottom = nearest_opaque_edge_pixel(&img, px, h - 1, true);
                spritesheet.put_pixel(x + px, y - 1, top);
                spritesheet.put_pixel(x + px, y + h, bottom);
            }
            for py in 0..h {
                let left = nearest_opaque_edge_pixel(&img, 0, py, false);
                let right = nearest_opaque_edge_pixel(&img, w - 1, py, false);
                spritesheet.put_pixel(x - 1, y + py, left);
                spritesheet.put_pixel(x + w, y + py, right);
            }
            regions.push(SpriteRegion { x, y, w, h });
            current_tile += 1;
        }

        atlas.spritestacks.insert(name, regions);
    }

    write_optimized_png(&web_dir.join("spritesheet.png"), &spritesheet);

    let atlas_json = serde_json::to_string_pretty(&atlas).expect("atlas must serialize");
    std::fs::write(web_dir.join("atlas.json"), atlas_json).expect("Failed to write atlas.json");

    println!(
        "cargo:warning=Generated spritesheet and atlas in {}",
        web_dir.display()
    );
}

fn nearest_opaque_edge_pixel(
    image: &RgbaImage,
    coordinate: u32,
    edge: u32,
    horizontal: bool,
) -> image::Rgba<u8> {
    let limit = if horizontal { image.height() } else { image.width() };
    for distance in 0..limit {
        let offset = if edge >= distance { edge - distance } else { edge + distance };
        if offset >= limit { continue; }
        let pixel = if horizontal {
            *image.get_pixel(coordinate, offset)
        } else {
            *image.get_pixel(offset, coordinate)
        };
        if pixel[3] > 0 { return pixel; }
    }
    *image.get_pixel(if horizontal { coordinate } else { edge }, if horizontal { edge } else { coordinate })
}

/// Keep the runtime format as RGBA8 while maximizing lossless PNG compression.
/// Indexed/palette PNG would reduce transfer size only when the complete atlas
/// fits a small palette and would complicate alpha/color validation. It is a
/// separate, measured optimization rather than an implicit visual change.
fn write_optimized_png(path: &Path, image: &RgbaImage) {
    let file = std::fs::File::create(path).expect("Failed to create spritesheet.png");
    let encoder = PngEncoder::new_with_quality(file, CompressionType::Best, FilterType::Adaptive);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8,
        )
        .expect("Failed to encode optimized spritesheet.png");
}

fn collect_character_images(
    assets_dir: &Path,
    spritestack_rgb_bits: u8,
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
            spritestack_processing::quantize_rgb_channels(image.as_mut(), spritestack_rgb_bits);
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
