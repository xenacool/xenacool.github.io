use crate::animation::InactiveFSMDefinition;
use crate::domain::{HexGrid, HexMap, LightingConfig, Material, Shape3D, Spritestack};
use glam::Vec3;
use hexx::Hex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    Spritestack(Spritestack),
    AssetRef(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TweenKind {
    SineInOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransitionConfig {
    pub duration_ms: u32,
    pub delta_time_ms: f32,
    pub tween: TweenKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableMove {
    pub hex: Hex,
    pub layer: i32,
    pub ap_cost: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableAbility {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableJobActions {
    pub name: String,
    pub abilities: Vec<AvailableAbility>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableActions {
    pub unit_id: u64,
    pub movement: Vec<AvailableMove>,
    pub primary_job: AvailableJobActions,
    pub secondary_jobs: Vec<AvailableJobActions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameOutcome {
    Victory { winning_team: u8 },
    Defeat { winning_team: u8 },
    Draw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    TurnStarted {
        unit_id: u64,
    },
    TurnCompleted {
        unit_id: u64,
    },
    UnitStateChanged {
        unit_id: u64,
        hex: Hex,
        layer: i32,
        health: i32,
        mana: i32,
        action_points: i32,
    },
    GameCompleted {
        winning_team: Option<u8>,
        outcome: GameOutcome,
        completed_rounds: u32,
    },
    SpawnEntity {
        id: u64,
        kind: String,
        hex: Hex,
        init_properties: Vec<String>,
    },
    DespawnEntity {
        id: u64,
    },
    MoveSprite {
        id: u64,
        destination: Hex,
        transition: Option<TransitionConfig>,
    },
    UpdateProperty {
        id: u64,
        property: String,
        value: PropertyValue,
    },
    ConfigureTransition {
        id: u64,
        config: TransitionConfig,
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
        transition: TransitionConfig,
    },
    DefineAssetCollection {
        name: String,
        data: Vec<u8>,
    },
    AvailableActions(AvailableActions),
    Log {
        msg: String,
    },
    SequenceNumber(u64),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorldState {
    pub entities: Vec<EntityState>,
    pub materials: HashMap<String, Material>,
    pub fsms: HashMap<String, InactiveFSMDefinition>,
    pub asset_collections: HashMap<String, Arc<Vec<u8>>>,
    pub transition_configs: HashMap<u64, TransitionConfig>,
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
    pub fn new(id: u64, kind: String, hex: hexx::Hex, init_properties: &[String]) -> Self {
        let mut properties = HashMap::new();

        for property in init_properties {
            if let Some(value) = default_property_value(&kind, property) {
                properties.insert(property.clone(), value);
            }
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

fn default_property_value(kind: &str, property: &str) -> Option<PropertyValue> {
    if kind == "world" {
        return match property {
            "map" => Some(PropertyValue::HexMap(crate::domain::HexMap::new())),
            "lighting" => Some(PropertyValue::Lighting(
                crate::domain::LightingConfig::default(),
            )),
            _ => None,
        };
    }
    if kind == "camera" {
        return match property {
            "angle" | "target_x" | "target_y" | "target_z" => Some(PropertyValue::Float(0.0)),
            "distance" => Some(PropertyValue::Float(20.0)),
            "height" => Some(PropertyValue::Float(12.0)),
            _ => None,
        };
    }
    if is_character_joint_property(property) {
        return Some(PropertyValue::Float(0.0));
    }
    match property {
        "scale" => Some(PropertyValue::Float(1.0)),
        "z" | "layer" | "rotation_x" | "rotation_y" | "rotation_z" | "cam_offset_x"
        | "cam_offset_y" | "cam_offset_z" | "x_offset" | "y_offset" | "z_offset" => {
            Some(PropertyValue::Float(0.0))
        }
        "material" => Some(PropertyValue::Material(crate::domain::Material {
            color: [1.0, 1.0, 1.0],
            roughness: 0.5,
            metalness: 0.0,
            emissive: 0.0,
        })),
        "spritestack" => Some(PropertyValue::Spritestack(crate::domain::Spritestack {
            width: 0,
            height: 0,
            spacing: 0.1,
            aabb: Vec3::ZERO,
            slices: Vec::new(),
        })),
        _ => None,
    }
}

fn is_character_joint_property(property: &str) -> bool {
    const JOINTS: &[&str] = &[
        "chest",
        "neck",
        "head",
        "l_shoulder",
        "r_shoulder",
        "pelvis",
        "l_hip",
        "r_hip",
        "l_elbow",
        "l_hand",
        "r_elbow",
        "r_hand",
        "l_knee",
        "l_foot",
        "r_knee",
        "r_foot",
        "spine_1",
        "tail_1",
        "tail_2",
        "tail_3",
        "tail_4",
    ];

    let Some(joint) = property
        .strip_suffix("_x")
        .or_else(|| property.strip_suffix("_y"))
        .or_else(|| property.strip_suffix("_z"))
    else {
        return false;
    };
    JOINTS.contains(&joint)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub event_index: usize,
    pub state: WorldState,
}

#[cfg(test)]
mod tests {
    use super::{EntityState, PropertyValue};
    use hexx::Hex;

    #[test]
    fn entity_spawn_does_not_materialize_unknown_axis_properties() {
        let entity = EntityState::new(
            1,
            "rock".to_string(),
            Hex::ZERO,
            &[
                "r_hip_z".to_string(),
                "foo_z".to_string(),
                "scale".to_string(),
            ],
        );

        assert_eq!(
            entity.properties.get("r_hip_z"),
            Some(&PropertyValue::Float(0.0))
        );
        assert!(!entity.properties.contains_key("foo_z"));
        assert_eq!(
            entity.properties.get("scale"),
            Some(&PropertyValue::Float(1.0))
        );
    }
}
