use glam::Vec3;
use pystral_core::domain::{Spritestack, SpritestackSlice};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::spritestack_processing::{SpritestackProcessError, process_slice};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpriteRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SpriteAtlas {
    pub width: u32,
    pub height: u32,
    pub spritestacks: HashMap<String, Vec<SpriteRegion>>,
}

impl SpriteAtlas {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AssetCollection {
    pub spritestacks: HashMap<String, Spritestack>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    MissingSpritestack {
        name: String,
    },
    InvalidSpritesheetWidth {
        width: u32,
    },
    InvalidSpritesheetBuffer {
        expected: usize,
        actual: usize,
    },
    InvalidRegion {
        name: String,
        index: usize,
        reason: String,
    },
    InconsistentSliceDimensions {
        name: String,
        index: usize,
        expected: (u32, u32),
        actual: (u32, u32),
    },
    Processing(SpritestackProcessError),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSpritestack { name } => {
                write!(f, "spritestack {name} is missing from atlas")
            }
            Self::InvalidSpritesheetWidth { width } => {
                write!(f, "spritesheet width must be positive, got {width}")
            }
            Self::InvalidSpritesheetBuffer { expected, actual } => write!(
                f,
                "spritesheet buffer has {actual} bytes; expected {expected}"
            ),
            Self::InvalidRegion {
                name,
                index,
                reason,
            } => {
                write!(f, "invalid region {index} for {name}: {reason}")
            }
            Self::InconsistentSliceDimensions {
                name,
                index,
                expected,
                actual,
            } => write!(
                f,
                "slice {index} for {name} is {}x{}; expected {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::Processing(error) => write!(f, "spritestack processing failed: {error}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<SpritestackProcessError> for AssetError {
    fn from(error: SpritestackProcessError) -> Self {
        Self::Processing(error)
    }
}

impl AssetCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_binary(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Failed to serialize asset collection")
    }

    pub fn add_atlas_spritestack(
        &mut self,
        name: &str,
        spacing: f32,
        atlas: &SpriteAtlas,
        spritesheet_rgba: &[u8],
        spritesheet_width: u32,
    ) -> Result<(), AssetError> {
        let regions =
            atlas
                .spritestacks
                .get(name)
                .ok_or_else(|| AssetError::MissingSpritestack {
                    name: name.to_string(),
                })?;
        if regions.is_empty() {
            return Err(AssetError::InvalidRegion {
                name: name.to_string(),
                index: 0,
                reason: "atlas entry has no regions".to_string(),
            });
        }
        if spritesheet_width == 0 {
            return Err(AssetError::InvalidSpritesheetWidth {
                width: spritesheet_width,
            });
        }
        let expected_buffer = (atlas.width as usize)
            .checked_mul(atlas.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| AssetError::InvalidSpritesheetBuffer {
                expected: usize::MAX,
                actual: spritesheet_rgba.len(),
            })?;
        if spritesheet_width != atlas.width || spritesheet_rgba.len() != expected_buffer {
            return Err(AssetError::InvalidSpritesheetBuffer {
                expected: expected_buffer,
                actual: spritesheet_rgba.len(),
            });
        }

        let expected_dimensions = regions.first().map(|region| (region.w, region.h));
        for (index, region) in regions.iter().enumerate() {
            if region.w == 0 || region.h == 0 {
                return Err(AssetError::InvalidRegion {
                    name: name.to_string(),
                    index,
                    reason: "region dimensions must be positive".to_string(),
                });
            }
            if let Some(expected) = expected_dimensions {
                if (region.w, region.h) != expected {
                    return Err(AssetError::InconsistentSliceDimensions {
                        name: name.to_string(),
                        index,
                        expected,
                        actual: (region.w, region.h),
                    });
                }
            }
            let x_end = region.x.checked_add(region.w);
            let y_end = region.y.checked_add(region.h);
            if x_end.is_none_or(|end| end > atlas.width)
                || y_end.is_none_or(|end| end > atlas.height)
            {
                return Err(AssetError::InvalidRegion {
                    name: name.to_string(),
                    index,
                    reason: "region exceeds atlas dimensions".to_string(),
                });
            }
        }
        let mut slices = Vec::new();
        let mut width = 0;
        let mut height = 0;

        for region in regions {
            if width == 0 {
                width = region.w;
                height = region.h;
            }

            let mut color_data = Vec::with_capacity((region.w * region.h * 4) as usize);
            for y in 0..region.h {
                let row_start = ((region.y + y) as usize)
                    .checked_mul(spritesheet_width as usize)
                    .and_then(|offset| offset.checked_add(region.x as usize))
                    .and_then(|offset| offset.checked_mul(4))
                    .ok_or_else(|| AssetError::InvalidRegion {
                        name: name.to_string(),
                        index: slices.len(),
                        reason: "row offset overflow".to_string(),
                    })?;
                let row_end = row_start
                    .checked_add(region.w as usize * 4)
                    .ok_or_else(|| AssetError::InvalidRegion {
                        name: name.to_string(),
                        index: slices.len(),
                        reason: "row length overflow".to_string(),
                    })?;
                let row = spritesheet_rgba.get(row_start..row_end).ok_or_else(|| {
                    AssetError::InvalidRegion {
                        name: name.to_string(),
                        index: slices.len(),
                        reason: "region is outside the spritesheet buffer".to_string(),
                    }
                })?;
                color_data.extend_from_slice(row);
            }

            process_slice(
                region.w as usize,
                region.h as usize,
                &mut color_data,
                Default::default(),
            )
            .map_err(AssetError::from)?;

            let pixel_count = (region.w * region.h) as usize;
            let mut normal_data = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
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
        Ok(())
    }

    pub fn from_binary(data: &[u8]) -> Self {
        bincode::deserialize(data).expect("Failed to deserialize asset collection")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas_with_region(name: &str, region: SpriteRegion) -> SpriteAtlas {
        SpriteAtlas {
            width: 2,
            height: 1,
            spritestacks: HashMap::from([(name.to_string(), vec![region])]),
        }
    }

    #[test]
    fn malformed_atlas_name_does_not_mutate_collection() {
        let mut collection = AssetCollection::new();
        let atlas = SpriteAtlas::default();
        let error = collection
            .add_atlas_spritestack("Missing", 1.0, &atlas, &[], 0)
            .unwrap_err();

        assert!(matches!(error, AssetError::MissingSpritestack { .. }));
        assert!(collection.spritestacks.is_empty());
    }

    #[test]
    fn malformed_region_is_rejected_atomically() {
        let mut collection = AssetCollection::new();
        let atlas = atlas_with_region(
            "Caveman",
            SpriteRegion {
                x: 2,
                y: 0,
                w: 1,
                h: 1,
            },
        );
        let error = collection
            .add_atlas_spritestack("Caveman", 1.0, &atlas, &[1; 8], 2)
            .unwrap_err();

        assert!(matches!(error, AssetError::InvalidRegion { .. }));
        assert!(collection.spritestacks.is_empty());
    }

    #[test]
    fn valid_atlas_region_is_processed_and_inserted_after_validation() {
        let mut collection = AssetCollection::new();
        let atlas = atlas_with_region(
            "Caveman",
            SpriteRegion {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
        );
        collection
            .add_atlas_spritestack("Caveman", 1.0, &atlas, &[10, 20, 30, 128, 0, 0, 0, 0], 2)
            .unwrap();

        let stack = collection.spritestacks.get("Caveman").unwrap();
        assert_eq!(stack.slices.len(), 1);
        assert_eq!(stack.slices[0].color_data, vec![10, 20, 30, 255]);
        assert_eq!(stack.slices[0].normal_data.len(), 4);
    }
}
