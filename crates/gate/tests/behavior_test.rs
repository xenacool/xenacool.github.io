#![allow(clippy::panic, clippy::unwrap_used, clippy::never_loop)]

use futures::channel::mpsc;
use pystral_core::history::HistoryManager;
use pystral_gate::WorkerInput;
use pystral_gate::render::utils::{EntityExt, RenderResultExt};
use pystral_runtime::pg_rpg::generate_pg_rpg_log;

#[test]
fn test_history_behavior_at_boundaries() {
    let (atlas_json, spritesheet_rgba, width) = pystral_gate::load_test_assets();
    let mut history = HistoryManager::new();
    generate_pg_rpg_log(&mut history, &atlas_json, &spritesheet_rgba, width);
    let total_len = history.log.len();

    // Given history at the end
    history.jump_to(total_len);
    let state_at_end = history.current_state.clone();
    assert_eq!(history.current_index, total_len);

    // When attempting to jump beyond or redo at the end
    history.jump_to(total_len + 10);
    assert_eq!(history.current_index, total_len);
    assert_eq!(history.current_state, state_at_end);

    history.redo();
    assert_eq!(history.current_index, total_len);
    assert_eq!(history.current_state, state_at_end);

    // When jumping to index 0
    history.jump_to(0);
    assert_eq!(history.current_index, 0);
    assert!(history.current_state.entities.is_empty());
}

#[test]
fn test_print_log() {
    let (atlas_json, spritesheet_rgba, width) = pystral_gate::load_test_assets();
    let mut history = HistoryManager::new();
    generate_pg_rpg_log(&mut history, &atlas_json, &spritesheet_rgba, width);
    for (i, event) in history.log.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
}

#[test]
fn test_pg_rpg_log_rendering_behavior_strict() {
    let (tx, mut rx) = mpsc::unbounded::<WorkerInput>();

    let (atlas_json, spritesheet_rgba, width) = pystral_gate::load_test_assets();
    let mut history = HistoryManager::new();
    generate_pg_rpg_log(&mut history, &atlas_json, &spritesheet_rgba, width);

    // Simulate each step of the log and check for UI log issues
    for i in 0..=history.log.len() {
        history.jump_to(i);

        let state = &history.current_state;
        for entity in &state.entities {
            if entity.id == 0 {
                if entity.properties.contains_key("map") {
                    let _ = entity.get_hex_map().log_fallback(&tx);
                    let _ = entity.get_lighting().log_fallback(&tx);
                }
            } else if entity.kind == "camera" {
                if entity.properties.contains_key("angle") {
                    let _ = entity.get_float("angle", 0.0).log_fallback(&tx);
                    let _ = entity.get_float("distance", 0.0).log_fallback(&tx);
                    let _ = entity.get_float("height", 0.0).log_fallback(&tx);
                    let _ = entity.get_float("target_x", 0.0).log_fallback(&tx);
                    let _ = entity.get_float("target_y", 0.0).log_fallback(&tx);
                    let _ = entity.get_float("target_z", 0.0).log_fallback(&tx);
                }
            } else {
                if !entity.properties.contains_key("asset") {
                    continue;
                }

                let _ = entity.get_float("scale", 1.0).log_fallback(&tx);
                let _ = entity.get_float("z", 0.0).log_fallback(&tx);
                let _ = entity.get_float("rotation_z", 0.0).log_fallback(&tx);
                let _ = entity.get_float("cam_offset_x", 0.0).log_fallback(&tx);
                let _ = entity.get_float("cam_offset_y", 0.0).log_fallback(&tx);
                let _ = entity.get_float("cam_offset_z", 0.0).log_fallback(&tx);
                let _ = entity.get_material(&state.materials).log_fallback(&tx);

                let _ = entity.get_collision().log_fallback(&tx);
            }
        }

        // Check for errors in the channel
        let mut error_messages = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            match msg {
                WorkerInput::LogError(msg) => {
                    error_messages.push(msg);
                }
                _ => {}
            }
            if error_messages.len() >= 10 {
                break;
            }
        }
        if !error_messages.is_empty() {
            panic!(
                "UI Log errors at index {}:\n{}",
                i,
                error_messages.join("\n")
            );
        }
    }
}

#[test]
fn test_arrow_trajectory_arc() {
    let (atlas_json, spritesheet_rgba, width) = pystral_gate::load_test_assets();
    let mut history = HistoryManager::new();
    generate_pg_rpg_log(&mut history, &atlas_json, &spritesheet_rgba, width);

    // Find the arrow FSM definition
    let arrow_fsm = history
        .current_state
        .fsms
        .get("arrow_fsm")
        .expect("Arrow FSM should be defined");
    let flight_state = arrow_fsm
        .states
        .get("flight")
        .expect("Flight state should exist");

    let z_track = flight_state.tracks.iter().find(|t| t.property == "z")
        .expect("Z track should exist. This might mean the trajectory solver failed to find a non-colliding path.");

    // Check that Z changes (it's a parabolic arc)
    let mut has_height_change = false;
    let mut max_z = 0.0f32;

    for kf in &z_track.keyframes {
        if let pystral_core::log::PropertyValue::Float(z) = kf.value {
            if z > 0.1 {
                has_height_change = true;
            }
            if z > max_z {
                max_z = z;
            }
        }
    }

    assert!(
        has_height_change,
        "Arrow should have some height in its flight arc"
    );
    assert!(
        max_z > 1.0,
        "Arrow peak should be significant (max_z: {})",
        max_z
    );
    assert!(
        max_z < 4.0,
        "Arrow flies above the window top (max_z: {})",
        max_z
    );

    // Verify rotation changes (ascend/descend)
    let rot_track = flight_state
        .tracks
        .iter()
        .find(|t| t.property == "rotation_z")
        .expect("Rotation track should exist");

    let first_rot = if let pystral_core::log::PropertyValue::Float(r) =
        rot_track.keyframes.first().unwrap().value
    {
        r
    } else {
        0.0
    };
    let last_rot = if let pystral_core::log::PropertyValue::Float(r) =
        rot_track.keyframes.last().unwrap().value
    {
        r
    } else {
        0.0
    };

    assert!(
        first_rot > 0.0,
        "Arrow should tilt UP at start (first_rot: {})",
        first_rot
    );
    assert!(
        last_rot < 0.0,
        "Arrow should tilt DOWN at end (last_rot: {})",
        last_rot
    );
}

#[test]
fn test_material_resolution_behavior() {
    let (atlas_json, spritesheet_rgba, width) = pystral_gate::load_test_assets();
    let mut history = HistoryManager::new();
    generate_pg_rpg_log(&mut history, &atlas_json, &spritesheet_rgba, width);

    // Jump to after arrow is spawned (it uses a named material)
    history.jump_to(history.log.len());

    let state = &history.current_state;
    let arrow = state
        .entities
        .iter()
        .find(|e| e.id == 4)
        .expect("Arrow should exist");
    let mat_res = arrow.get_material(&state.materials);

    assert!(
        mat_res.is_ok(),
        "Material arrow_mat should be resolved, but got: {:?}",
        mat_res.err().map(|e| e.message)
    );
}

#[test]
fn test_active_fsm_property_interpolation() {
    use pystral_core::animation::{
        ActiveFSM, AnimationState, InactiveFSMDefinition, Keyframe, LoopBehavior, PropertyTrack,
    };
    use pystral_core::log::PropertyValue;
    use std::collections::HashMap;

    let mut states = HashMap::new();
    states.insert(
        "idle".to_string(),
        AnimationState {
            name: "idle".to_string(),
            tracks: vec![PropertyTrack {
                property: "z".to_string(),
                keyframes: vec![
                    Keyframe {
                        time_ms: 0.0,
                        value: PropertyValue::Float(0.0),
                    },
                    Keyframe {
                        time_ms: 1000.0,
                        value: PropertyValue::Float(10.0),
                    },
                ],
                loop_behavior: LoopBehavior::None,
            }],
        },
    );

    let definition = InactiveFSMDefinition { states };
    let mut fsm = ActiveFSM::new(definition, "idle".to_string(), 0.0);

    fsm.update(0.0);
    assert_eq!(
        fsm.current_properties.get("z"),
        Some(&PropertyValue::Float(0.0))
    );

    fsm.update(500.0);
    assert_eq!(
        fsm.current_properties.get("z"),
        Some(&PropertyValue::Float(5.0))
    );

    fsm.update(1000.0);
    assert_eq!(
        fsm.current_properties.get("z"),
        Some(&PropertyValue::Float(10.0))
    );

    fsm.update(1500.0);
    assert_eq!(
        fsm.current_properties.get("z"),
        Some(&PropertyValue::Float(10.0))
    );
}
