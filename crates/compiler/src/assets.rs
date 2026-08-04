use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use pystral_core::domain::{Spritestack, SpritestackSlice};

#[derive(Serialize, Deserialize, Debug)]
pub struct AssetCollection {
    pub spritestacks: HashMap<String, Spritestack>,
}

impl AssetCollection {
    pub fn new() -> Self {
        Self {
            spritestacks: HashMap::new(),
        }
    }

    pub fn add_cube(&mut self, name: &str, size: u32, color: [u8; 4], spacing: f32) {
        let mut slices = Vec::new();
        let pixel_count = (size * size) as usize;
        
        for i in 0..size {
            let mut color_data = vec![0u8; pixel_count * 4];
            let mut normal_data = vec![0u8; pixel_count * 4];
            
            for y in 0..size {
                for x in 0..size {
                    let idx = (y * size + x) as usize * 4;
                    
                    // Simple cube: all pixels filled
                    color_data[idx..idx+4].copy_from_slice(&color);
                    
                    // Normal calculation (very basic)
                    // Ny is UP (slicer Y), Nx is right, Nz is depth
                    let mut nx = 0.0f32;
                    let mut ny = 0.0f32;
                    let mut nz = 0.0f32;
                    
                    if x == 0 { nx = -1.0; }
                    else if x == size - 1 { nx = 1.0; }
                    
                    if i == 0 { ny = -1.0; }
                    else if i == size - 1 { ny = 1.0; }
                    
                    if y == 0 { nz = -1.0; }
                    else if y == size - 1 { nz = 1.0; }
                    
                    // Normalize
                    let len = (nx*nx + ny*ny + nz*nz).sqrt();
                    if len > 0.0 {
                        nx /= len;
                        ny /= len;
                        nz /= len;
                    }
                    
                    normal_data[idx] = ((nx * 0.5 + 0.5) * 255.0) as u8;
                    normal_data[idx+1] = ((ny * 0.5 + 0.5) * 255.0) as u8;
                    normal_data[idx+2] = ((nz * 0.5 + 0.5) * 255.0) as u8;
                    normal_data[idx+3] = 255;
                }
            }
            
            slices.push(SpritestackSlice {
                color_data,
                normal_data,
            });
        }
        
        self.spritestacks.insert(name.to_string(), Spritestack {
            width: size,
            height: size,
            spacing,
            slices,
        });
    }

    pub fn to_binary(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize asset collection")
    }

    pub fn from_binary(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Failed to deserialize asset collection")
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
