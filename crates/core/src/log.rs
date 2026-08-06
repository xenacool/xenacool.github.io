use serde::{Serialize, Deserialize};
use hexx::Hex;
use glam::Vec3;
use crate::domain::{HexGrid, HexMap, Material, LightingConfig, Shape3D, Skeleton, SpritePart, Spritestack};
use crate::animation::InactiveFSMDefinition;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PropertyValue {
    Float(f32),
    String(String),
    Vec3(Vec3),
    Color([f32; 3]),
    HexGrid(HexGrid),
    HexMap(HexMap),
    Material(Material),
    Lighting(LightingConfig),
    Shape3D(Shape3D),
    Skeleton(Skeleton),
    SpriteParts(Vec<SpritePart>),
    Spritestack(Spritestack),
    AssetRef(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    SpawnEntity {
        id: u64,
        kind: String,
        hex: Hex,
    },
    DespawnEntity {
        id: u64,
    },
    MoveSprite {
        id: u64,
        destination: Hex,
        duration_ms: Option<u32>,
    },
    UpdateProperty {
        id: u64,
        property: String,
        value: PropertyValue,
    },
    SetAnimationState {
        id: u64,
        state: String,
    },
    DefineMaterial {
        name: String,
        material: Material,
    },
    DefineFSM {
        name: String,
        definition: InactiveFSMDefinition,
    },
    TweenProperty {
        id: u64,
        property: String,
        value: PropertyValue,
        duration_ms: u32,
    },
    DefineAssetCollection {
        name: String,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorldState {
    pub entities: Vec<EntityState>,
    pub materials: HashMap<String, Material>,
    pub fsms: HashMap<String, InactiveFSMDefinition>,
    pub asset_collections: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityState {
    pub id: u64,
    pub kind: String,
    pub hex: Hex,
    pub animation_state: String,
    pub properties: HashMap<String, PropertyValue>,
    pub fsm_name: Option<String>,
}

impl EntityState {
    pub fn new(id: u64, kind: String, hex: hexx::Hex) -> Self {
        let mut properties = HashMap::new();
        
        // Add kind-based defaults to avoid log errors during setup
        if kind == "world" {
            properties.insert("map".to_string(), PropertyValue::HexMap(crate::domain::HexMap::new()));
            properties.insert("lighting".to_string(), PropertyValue::Lighting(crate::domain::LightingConfig::default()));
        } else if kind == "sprite" || kind == "arrow" || kind == "rock" || kind == "skeleton_minion" || kind == "necromancer" || kind == "caveman" || kind == "mage" || kind == "character" {
            properties.insert("scale".to_string(), PropertyValue::Float(1.0));
            properties.insert("z".to_string(), PropertyValue::Float(0.0));
            properties.insert("rotation_x".to_string(), PropertyValue::Float(0.0));
            properties.insert("rotation_y".to_string(), PropertyValue::Float(0.0));
            properties.insert("rotation_z".to_string(), PropertyValue::Float(0.0));
            properties.insert("cam_offset_x".to_string(), PropertyValue::Float(0.0));
            properties.insert("cam_offset_y".to_string(), PropertyValue::Float(0.0));
            properties.insert("cam_offset_z".to_string(), PropertyValue::Float(0.0));
            properties.insert("x".to_string(), PropertyValue::Float(0.0));
            properties.insert("y".to_string(), PropertyValue::Float(0.0));
            
            // Joint defaults to avoid log errors during character setup
            let joints = vec![
                "chest", "neck", "head", "l_shoulder", "r_shoulder", "pelvis",
                "l_hip", "r_hip", "l_elbow", "l_hand", "r_elbow", "r_hand",
                "l_knee", "l_foot", "r_knee", "r_foot",
                "spine_1", "tail_1", "tail_2", "tail_3", "tail_4"
            ];
            for j in joints {
                properties.insert(format!("{j}_x"), PropertyValue::Float(0.0));
                properties.insert(format!("{j}_y"), PropertyValue::Float(0.0));
                properties.insert(format!("{j}_z"), PropertyValue::Float(0.0));
            }

            properties.insert("material".to_string(), PropertyValue::Material(crate::domain::Material {
                color: [1.0, 1.0, 1.0],
                roughness: 0.5,
                metalness: 0.0,
                emissive: 0.0,
            }));
            properties.insert("sprite_parts".to_string(), PropertyValue::SpriteParts(Vec::new()));
            properties.insert("spritestack".to_string(), PropertyValue::Spritestack(crate::domain::Spritestack {
                width: 0,
                height: 0,
                spacing: 0.1,
                aabb: Vec3::ZERO,
                slices: Vec::new(),
            }));
        } else if kind == "camera" {
            properties.insert("angle".to_string(), PropertyValue::Float(0.0));
            properties.insert("distance".to_string(), PropertyValue::Float(20.0));
            properties.insert("height".to_string(), PropertyValue::Float(12.0));
            properties.insert("target_x".to_string(), PropertyValue::Float(0.0));
            properties.insert("target_y".to_string(), PropertyValue::Float(0.0));
            properties.insert("target_z".to_string(), PropertyValue::Float(0.0));
        }

        Self {
            id,
            kind,
            hex,
            animation_state: "idle".to_string(),
            properties,
            fsm_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub event_index: usize,
    pub state: WorldState,
}
