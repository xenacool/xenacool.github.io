use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use image::{GenericImage, RgbaImage};
use serde::{Serialize, Deserialize};

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
    println!("cargo:rerun-if-changed=../../assets/spritestacks");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let web_dir = root_dir.join("web");
    
    if !web_dir.exists() {
        std::fs::create_dir_all(&web_dir).unwrap();
    }

    let assets_dir = root_dir.join("assets").join("spritestacks");
    if !assets_dir.exists() {
        println!("cargo:warning=Assets directory not found at {:?}", assets_dir);
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
            characters.entry(char_name).or_default().push(entry.path().to_path_buf());
        }
    }

    if characters.is_empty() {
        println!("cargo:warning=No character sprites found in {:?}", assets_dir);
        return;
    }

    // Sort layers for consistency
    for layers in characters.values_mut() {
        layers.sort_by_key(|p| {
            p.file_stem().unwrap().to_str().unwrap()
                .replace("layer-", "").parse::<u32>().unwrap_or(0)
        });
    }

    let mut total_width = 0u32;
    let mut total_height = 0u32;
    let mut char_images = Vec::new();

    let mut sorted_char_names: Vec<_> = characters.keys().cloned().collect();
    sorted_char_names.sort();

    for name in sorted_char_names {
        let layers = &characters[&name];
        let mut char_layer_images = Vec::new();
        for layer_path in layers {
            let img = image::open(layer_path).unwrap().to_rgba8();
            total_width = total_width.max(img.width());
            char_layer_images.push(img);
        }
        char_images.push((name, char_layer_images));
    }

    for (_, layers) in &char_images {
        for img in layers {
            total_height += img.height();
        }
    }

    let mut spritesheet = RgbaImage::new(total_width, total_height);
    let mut current_y = 0u32;

    atlas.width = total_width;
    atlas.height = total_height;

    for (name, layers) in char_images {
        let mut regions = Vec::new();
        for img in layers {
            let w = img.width();
            let h = img.height();
            spritesheet.copy_from(&img, 0, current_y).expect("Failed to copy image to spritesheet");
            regions.push(SpriteRegion { x: 0, y: current_y, w, h });
            current_y += h;
        }
        atlas.spritestacks.insert(name, regions);
    }

    spritesheet.save(web_dir.join("spritesheet.png")).expect("Failed to save spritesheet");
    
    let atlas_json = serde_json::to_string_pretty(&atlas).unwrap();
    std::fs::write(web_dir.join("atlas.json"), atlas_json).expect("Failed to write atlas.json");
    
    println!("cargo:warning=Generated spritesheet and atlas in {:?}", web_dir);
}
