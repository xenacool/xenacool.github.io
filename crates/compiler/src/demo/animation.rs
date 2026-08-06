use pystral_core::animation::{Keyframe, LoopBehavior, PropertyTrack};
use pystral_core::log::PropertyValue;
use crate::ik::{IkSystem, IkRequest, Vec3 as IkVec3};
use crate::physics::{TrajectorySystem, TrajectoryRequest};
use glam::Vec3;
use std::collections::HashMap;

pub fn generate_ik_tracks(
    ik_system: &mut IkSystem,
    rig_id: &str,
    num_steps: usize,
    step_duration_ms: f32,
    target_fn: impl Fn(usize, f64) -> HashMap<String, IkVec3>,
) -> HashMap<String, Vec<Keyframe>> {
    let mut tracks: HashMap<String, Vec<Keyframe>> = HashMap::new();
    for i in 0..=num_steps {
        let t_ms = (i as f32) * step_duration_ms;
        let phase = (i as f64) * 2.0 * std::f64::consts::PI / (num_steps as f64);
        let targets = target_fn(i, phase);
        
        let request = IkRequest {
            rig_id: rig_id.to_string(),
            targets,
            initial_guesses: HashMap::new(),
        };

        if let Ok(response) = ik_system.solve(request) {
            for (joint, pos) in &response.joints {
                tracks.entry(format!("{}_x", joint)).or_default().push(Keyframe { time_ms: t_ms, value: PropertyValue::Float(pos.x) });
                tracks.entry(format!("{}_y", joint)).or_default().push(Keyframe { time_ms: t_ms, value: PropertyValue::Float(pos.y) });
                tracks.entry(format!("{}_z", joint)).or_default().push(Keyframe { time_ms: t_ms, value: PropertyValue::Float(pos.z) });
            }
        }
    }
    tracks
}

pub fn generate_arrow_tracks(
    trajectory_system: &TrajectorySystem,
    start: Vec3,
    target: Vec3,
    map: &pystral_core::domain::HexMap,
) -> Vec<PropertyTrack> {
    let request = TrajectoryRequest {
        start,
        target,
        initial_speed: 15.0,
        gravity: 9.81,
    };

    let mut tracks = Vec::new();
    match trajectory_system.solve(request, map) {
        Ok(response) => {
            let mut x_keyframes = Vec::new();
            let mut y_keyframes = Vec::new();
            let mut z_keyframes = Vec::new();
            let mut pitch_keyframes = Vec::new();
            let mut yaw_keyframes = Vec::new();
            let duration = 2000.0;
            let num_points = response.trajectory.len();

            for (i, pos) in response.trajectory.iter().enumerate() {
                let t = (i as f32) * duration / (num_points as f32 - 1.0);
                x_keyframes.push(Keyframe { time_ms: t, value: PropertyValue::Float(pos.x) });
                y_keyframes.push(Keyframe { time_ms: t, value: PropertyValue::Float(pos.z) }); // 3D Z is Y in our engine
                z_keyframes.push(Keyframe { time_ms: t, value: PropertyValue::Float(pos.y) }); // 3D Y is Z in our engine
                
                let pitch = response.rotations[i];
                pitch_keyframes.push(Keyframe { time_ms: t, value: PropertyValue::Float(-pitch) }); // Negative pitch to tilt up
                yaw_keyframes.push(Keyframe { time_ms: t, value: PropertyValue::Float(response.yaw) });
            }

            tracks.push(PropertyTrack { property: "world_x".into(), keyframes: x_keyframes, loop_behavior: LoopBehavior::Loop });
            tracks.push(PropertyTrack { property: "world_y".into(), keyframes: y_keyframes, loop_behavior: LoopBehavior::Loop });
            tracks.push(PropertyTrack { property: "z".into(), keyframes: z_keyframes, loop_behavior: LoopBehavior::Loop });
            tracks.push(PropertyTrack { property: "rotation_y".into(), keyframes: pitch_keyframes, loop_behavior: LoopBehavior::Loop });
            tracks.push(PropertyTrack { property: "rotation_z".into(), keyframes: yaw_keyframes, loop_behavior: LoopBehavior::Loop });
        }
        Err(_e) => {
            // Log to UI if we can't solve it
            // Wait, we don't have access to ui_log here easily without importing it
            // But we can just leave it empty and let the test fail with a better message
        }
    }
    tracks
}
