pub mod world;
pub mod animation;
pub mod entity;
pub mod assets;

use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue};
use pystral_core::domain::Material;
use pystral_core::animation::{InactiveFSMDefinition, AnimationState};
use hexx::Hex;
use std::collections::HashMap;
use glam::Vec3;

use crate::physics::TrajectorySystem;
use self::world::create_demo_world;
use self::animation::generate_arrow_tracks;
use self::entity::{setup_rocks, setup_spritestack_demo};
use crate::character::setup_character;

pub fn generate_demo_log(history: &mut HistoryManager) {
    self::assets::setup_spritestack_assets(history);
    setup_camera(history);
    setup_world(history);
    setup_character(history);
    setup_rocks(history);
    setup_spritestack_demo(history);
    setup_arrow(history);
    finalize_demo(history);
}

fn finalize_demo(history: &mut HistoryManager) {
    // Add 100 more events of camera orbiting to let the animations play out
    for i in 0..100 {
        let angle = (i as f32) * 0.05;
        // TODO I love the cinematic camera but I'm setting up viewing angles at the moment
        history.push_and_apply(Event::UpdateProperty {
            id: 2,
            property: "angle".to_string(),
            value: PropertyValue::Float(angle),
        });
    }
}

fn setup_camera(history: &mut HistoryManager) {
    history.push_and_apply(Event::SpawnEntity {
        id: 2,
        kind: "camera".to_string(),
        hex: Hex::ZERO,
    });

    let mut camera_states = HashMap::new();
    camera_states.insert("orbit".to_string(), AnimationState {
        name: "orbit".to_string(),
        tracks: vec![], // simplified
    });

    history.push_and_apply(Event::DefineFSM {
        name: "camera_fsm".to_string(),
        definition: InactiveFSMDefinition { states: camera_states },
    });

    history.push_and_apply(Event::UpdateProperty {
        id: 2,
        property: "fsm".to_string(),
        value: PropertyValue::String("camera_fsm".to_string()),
    });

    history.push_and_apply(Event::UpdateProperty { id: 2, property: "angle".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 2, property: "distance".to_string(), value: PropertyValue::Float(20.0) });
    history.push_and_apply(Event::UpdateProperty { id: 2, property: "height".to_string(), value: PropertyValue::Float(12.0) });
    history.push_and_apply(Event::UpdateProperty { id: 2, property: "target_x".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 2, property: "target_y".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 2, property: "target_z".to_string(), value: PropertyValue::Float(0.0) });
}

fn setup_world(history: &mut HistoryManager) {
    history.push_and_apply(Event::DefineMaterial {
        name: "rock".to_string(),
        material: Material { color: [0.5, 0.5, 0.5], roughness: 0.7, metalness: 0.2, emissive: 0.0 },
    });
    history.push_and_apply(Event::DefineMaterial {
        name: "grass".to_string(),
        material: Material { color: [0.2, 0.6, 0.2], roughness: 0.9, metalness: 0.0, emissive: 0.0 },
    });
    history.push_and_apply(Event::DefineMaterial {
        name: "dirt".to_string(),
        material: Material { color: [0.4, 0.3, 0.2], roughness: 1.0, metalness: 0.0, emissive: 0.0 },
    });

    let map = create_demo_world();
    history.push_and_apply(Event::SpawnEntity {
        id: 0,
        kind: "world".to_string(),
        hex: Hex::ZERO,
    });
    history.push_and_apply(Event::UpdateProperty {
        id: 0,
        property: "map".to_string(),
        value: PropertyValue::HexMap(map),
    });

    let initial_lighting = pystral_core::domain::LightingConfig {
        ambient_intensity: 0.4,
        lights: vec![pystral_core::domain::Light { direction: [1.0, -2.0, 1.0], color: [1.0, 1.0, 1.0], intensity: 1.0 }],
        ..Default::default()
    };
    history.push_and_apply(Event::UpdateProperty {
        id: 0,
        property: "lighting".to_string(),
        value: PropertyValue::Lighting(initial_lighting),
    });
}


fn setup_arrow(history: &mut HistoryManager) {
    let trajectory_system = TrajectorySystem::new();
    
    // Find character position
    let char_hex = history.current_state.entities.iter()
        .find(|e| e.id == 1)
        .map(|e| e.hex)
        .unwrap_or(Hex::new(0, 1));
    
    let layout = hexx::HexLayout::default();
    let char_pos = layout.hex_to_world_pos(char_hex);
    
    let start = Vec3::new(char_pos.x, 2.2, char_pos.y);
    let target = Vec3::new(-5.0, 2.0, 0.0); // Target on the other side of the wall
    
    // Get the map from the current state
    let map = history.current_state.entities.iter()
        .find(|e| e.id == 0)
        .and_then(|e| {
            if let Some(PropertyValue::HexMap(m)) = e.properties.get("map") {
                Some(m)
            } else {
                None
            }
        })
        .cloned()
        .unwrap_or_else(create_demo_world);

    let arrow_tracks = generate_arrow_tracks(&trajectory_system, start, target, &map);

    let mut states = HashMap::new();
    states.insert("flight".to_string(), AnimationState {
        name: "flight".to_string(),
        tracks: arrow_tracks,
    });

    history.push_and_apply(Event::DefineFSM {
        name: "arrow_fsm".to_string(),
        definition: InactiveFSMDefinition { states },
    });

    history.push_and_apply(Event::DefineMaterial {
        name: "arrow_mat".to_string(),
        material: Material { color: [0.8, 0.4, 0.2], roughness: 0.5, metalness: 0.5, emissive: 0.0 },
    });

    history.push_and_apply(Event::SpawnEntity { id: 3, kind: "arrow".to_string(), hex: Hex::ZERO });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "material".to_string(), value: PropertyValue::String("arrow_mat".to_string()) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "scale".to_string(), value: PropertyValue::Float(0.5) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "rotation_z".to_string(), value: PropertyValue::Float(0.0) });
    
    // Initialize properties used by sprite parts first to satisfy renderer at every step
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "x_offset".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "y_offset".to_string(), value: PropertyValue::Float(0.0) });

    history.push_and_apply(Event::UpdateProperty { id: 3, property: "z_offset".to_string(), value: PropertyValue::Float(0.0) });

    let arrow_color = [0.8, 0.4, 0.2, 1.0];
    history.push_and_apply(Event::UpdateProperty {
        id: 3,
        property: "sprite_parts".to_string(),
        value: PropertyValue::SpriteParts(vec![
            pystral_core::domain::SpritePart {
                x_prop: "x_offset".into(),
                y_prop: "y_offset".into(),
                z_prop: "z_offset".into(),
                rotation_prop: None,
                color: [arrow_color[0], arrow_color[1], arrow_color[2]],
                scale: 1.0,
                painter_commands: crate::character::make_arrow_commands(arrow_color),
                spritestack: None,
            }
        ]),
    });

    history.push_and_apply(Event::UpdateProperty { id: 3, property: "cam_offset_x".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "cam_offset_y".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "cam_offset_z".to_string(), value: PropertyValue::Float(0.0) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "world_x".to_string(), value: PropertyValue::Float(start.x) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "world_y".to_string(), value: PropertyValue::Float(start.z) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "z".to_string(), value: PropertyValue::Float(start.y) });
    history.push_and_apply(Event::UpdateProperty { id: 3, property: "fsm".to_string(), value: PropertyValue::String("arrow_fsm".to_string()) });
    history.push_and_apply(Event::SetAnimationState { id: 3, state: "flight".to_string() });
}
