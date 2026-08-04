use crate::render::state::{CameraTween, MovementTween, PropertyTween};
use pystral_core::animation::ActiveFSM;
use pystral_core::log::WorldState;
use std::collections::HashMap;

/// Rust owns authoritative presentation timing and poses; Three.js owns WebGL.
pub struct RenderContext {
    pub active_fsms: HashMap<u64, ActiveFSM>,
    pub movement_tweens: HashMap<u64, MovementTween>,
    pub property_tweens: HashMap<(u64, String), PropertyTween>,
    pub camera_tween: Option<CameraTween>,
    pub camera_pose: Option<(u64, [f32; 6])>,
    pub last_index: Option<usize>,
    pub tween_state: Option<WorldState>,
    pub active_camera_id: Option<u64>,
    pub camera_ids: Vec<u64>,
    pub next_seq: u64,
}

impl RenderContext {
    pub fn new() -> Self {
        Self {
            active_fsms: HashMap::new(),
            movement_tweens: HashMap::new(),
            property_tweens: HashMap::new(),
            camera_tween: None,
            camera_pose: None,
            last_index: None,
            tween_state: None,
            active_camera_id: None,
            camera_ids: Vec::new(),
            next_seq: 1,
        }
    }
}
