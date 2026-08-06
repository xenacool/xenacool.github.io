use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::io::Cursor;
use glam::Vec3;
use pystral_core::domain::{Spritestack, SpritestackSlice};
use png;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AssetCollection {
    pub spritestacks: HashMap<String, Spritestack>,
}

impl AssetCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_arrow(&mut self, name: &str, color: [u8; 4], spacing: f32) {
        let size = 32;
        let mut slices = Vec::new();
        let pixel_count = (size * size) as usize;
        
        for i in 0..size {
            let mut color_data = vec![0u8; pixel_count * 4];
            let mut normal_data = vec![0u8; pixel_count * 4];
            
            for y in 0..size {
                for x in 0..size {
                    let idx = (y * size + x) as usize * 4;
                    
                    let fx = x as f32 / (size - 1) as f32;
                    let fy = y as f32 / (size - 1) as f32;
                    let fi = i as f32 / (size - 1) as f32;
                    
                    let cx = fx - 0.5;
                    let cy = fy - 0.5;
                    let ci = fi - 0.5;
                    
                    let mut in_shape = false;
                    let mut current_color = color;

                    // Shaft: along X axis
                    if (-0.4..=0.2).contains(&cx) && cy.abs() < 0.03 && ci.abs() < 0.03 {
                        in_shape = true;
                    }
                    
                    // Head: cone at the end
                    if cx > 0.2 && cx <= 0.5 {
                        let head_progress = (cx - 0.2) / 0.3;
                        let radius = (1.0 - head_progress) * 0.12;
                        if (cy*cy + ci*ci).sqrt() < radius {
                            in_shape = true;
                        }
                    }

                    // Tail (fletching)
                    if (-0.5..-0.3).contains(&cx) {
                        if ci.abs() < 0.01 && cy.abs() < 0.12 {
                             in_shape = true;
                             current_color = [200, 200, 200, 255];
                        }
                        if cy.abs() < 0.01 && ci.abs() < 0.12 {
                             in_shape = true;
                             current_color = [200, 200, 200, 255];
                        }
                    }

                    if in_shape {
                        color_data[idx..idx+4].copy_from_slice(&current_color);
                        let nx = if cx > 0.4 { 1.0 } else if cx < -0.4 { -1.0 } else { 0.0 };
                        let ny = ci.signum();
                        let nz = cy.signum();
                        let len = (nx*nx + ny*ny + nz*nz).sqrt();
                        let (nx, ny, nz) = if len > 0.0 { (nx/len, ny/len, nz/len) } else { (0.0, 1.0, 0.0) };
                        
                        normal_data[idx] = ((nx * 0.5 + 0.5) * 255.0) as u8;
                        normal_data[idx+1] = ((ny * 0.5 + 0.5) * 255.0) as u8;
                        normal_data[idx+2] = ((nz * 0.5 + 0.5) * 255.0) as u8;
                        normal_data[idx+3] = 255;
                    }
                }
            }
            
            slices.push(SpritestackSlice { color_data, normal_data });
        }
        
        self.spritestacks.insert(name.to_string(), Spritestack {
            width: size,
            height: size,
            spacing,
            aabb: Vec3::new(
                (size as f32 - 0.5) * spacing,
                (slices.len() as f32 - 1.0) * spacing,
                (size as f32 - 0.5) * spacing,
            ),
            slices,
        });
    }

    pub fn add_rock(&mut self, name: &str, size: u32, color: [u8; 4], spacing: f32) {
        self.add_sphere(name, size, color, spacing, 1.0);
    }

    pub fn add_sphere(&mut self, name: &str, size: u32, color: [u8; 4], spacing: f32, _roughness: f32) {
        let mut slices = Vec::new();
        let pixel_count = (size * size) as usize;
        
        for i in 0..size {
            let mut color_data = vec![0u8; pixel_count * 4];
            let mut normal_data = vec![0u8; pixel_count * 4];
            
            for y in 0..size {
                for x in 0..size {
                    let idx = (y * size + x) as usize * 4;
                    
                    let fx = x as f32 / (size - 1) as f32 - 0.5;
                    let fy = y as f32 / (size - 1) as f32 - 0.5;
                    let fi = i as f32 / (size - 1) as f32 - 0.5;
                    
                    let dist = (fx*fx + fy*fy + fi*fi).sqrt();
                    let limit = 0.45;
                    
                    if dist < limit {
                        color_data[idx..idx+4].copy_from_slice(&color);
                        
                        let nx = fx / dist;
                        let ny = fi / dist;
                        let nz = fy / dist;
                        
                        normal_data[idx] = ((nx * 0.5 + 0.5) * 255.0) as u8;
                        normal_data[idx+1] = ((ny * 0.5 + 0.5) * 255.0) as u8;
                        normal_data[idx+2] = ((nz * 0.5 + 0.5) * 255.0) as u8;
                        normal_data[idx+3] = 255;
                    }
                }
            }
            
            slices.push(SpritestackSlice { color_data, normal_data });
        }
        
        self.spritestacks.insert(name.to_string(), Spritestack {
            width: size,
            height: size,
            spacing,
            aabb: Vec3::new(
                (size as f32 - 0.5) * spacing,
                (slices.len() as f32 - 1.0) * spacing,
                (size as f32 - 0.5) * spacing,
            ),
            slices,
        });
    }

    pub fn to_binary(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize asset collection")
    }

    pub fn add_png_spritestack(&mut self, name: &str, spacing: f32, layers: Vec<&[u8]>) {
        let mut slices = Vec::new();
        let mut width = 0;
        let mut height = 0;

        for layer_data in layers {
            let decoder = png::Decoder::new(Cursor::new(layer_data));
            let mut reader = decoder.read_info().expect("Failed to read PNG info");
            let mut buf = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut buf).expect("Failed to read PNG frame");

            if width == 0 {
                width = info.width;
                height = info.height;
            }

            let color_data = match info.color_type {
                png::ColorType::Rgba => buf,
                png::ColorType::Rgb => {
                    let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
                    for chunk in buf.chunks_exact(3) {
                        rgba.push(chunk[0]);
                        rgba.push(chunk[1]);
                        rgba.push(chunk[2]);
                        rgba.push(255);
                    }
                    rgba
                }
                _ => panic!("Unsupported PNG color type: {:?}", info.color_type),
            };

            let pixel_count = (width * height) as usize;
            let mut normal_data = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                // Default normal: pointing up (0, 1, 0)
                // Packed format: [ (nx*0.5+0.5)*255, (ny*0.5+0.5)*255, (nz*0.5+0.5)*255, 255 ]
                // nx=0 -> 127
                // ny=1 -> 255
                // nz=0 -> 127
                normal_data[i * 4] = 127;
                normal_data[i * 4 + 1] = 255;
                normal_data[i * 4 + 2] = 127;
                normal_data[i * 4 + 3] = 255;
            }

            slices.push(SpritestackSlice {
                color_data,
                normal_data,
            });
        }

        let aabb = Vec3::new(
            (width as f32 - 0.5) * spacing,
            (slices.len() as f32 - 1.0) * spacing,
            (height as f32 - 0.5) * spacing,
        );

        self.spritestacks.insert(
            name.to_string(),
            Spritestack {
                width,
                height,
                spacing,
                aabb,
                slices,
            },
        );
    }

    pub fn from_binary(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Failed to deserialize asset collection")
    }

    pub fn add_skeleton_minion(&mut self) {
        let layers = crate::skeleton_minion_assets::LAYERS.to_vec();
        let num_layers = layers.len();
        
        // Normalize to what it was with 100 layers and 0.05 spacing
        // Width/Height: (pixel_size - 0.5) * 0.05
        // Depth (stack height): (100 - 1) * 0.05 = 4.95
        // For SkeletonMinion, let's assume a target bounding box.
        // If we want it to be independent of layers, we set the AABB directly.
        
        self.add_png_spritestack(
            "SkeletonMinion",
            0.05, // This spacing will be used to calculate aabb in add_png_spritestack, 
                  // but we want to override it or use a different spacing.
            layers,
        );

        // Override AABB for SkeletonMinion to be independent of layers
        if let Some(stack) = self.spritestacks.get_mut("SkeletonMinion") {
            let original_spacing = 0.05;
            let original_layers = 100.0;
            stack.aabb = Vec3::new(
                (stack.width as f32 - 0.5) * original_spacing,
                (original_layers - 1.0) * original_spacing,
                (stack.height as f32 - 0.5) * original_spacing,
            );
            // We also need to update the spacing so the renderer knows how to space the current number of layers
            stack.spacing = stack.aabb.y / (num_layers as f32 - 1.0);
        }
    }
}

#[macro_export]
macro_rules! include_assets {
    ($path:expr) => {
        {
            let data = include_bytes!($path);
            $crate::assets::AssetCollection::from_binary(data)
        }
    };
}
