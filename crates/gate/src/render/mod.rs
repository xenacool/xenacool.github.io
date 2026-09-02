mod context;
pub mod loop_handler;
mod state;
pub mod utils;
pub use crate::render::state::PlaybackState;

use pystral_core::history::HistoryManager;
use pystral_core::log::WorldState;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{WebGlProgram, WebGlRenderingContext as GL, WebGlShader};

use crate::render::context::RenderContext;
use crate::render::loop_handler::LoopHandler;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    pub fn update_ui_slider(index: u32);

    #[wasm_bindgen(js_namespace = window)]
    pub fn set_ui_slider_max(max: u32);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_nav_buttons(up: bool, down: bool, left: bool, right: bool);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_action_buttons(
        visible: bool,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        layer_up: bool,
        layer_down: bool,
        confirm: bool,
        ret: bool,
        wait: bool,
        face: bool,
    );

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_entity_viewer(json: &str);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_history_log(json: &str);

    #[wasm_bindgen(js_namespace = window)]
    pub fn update_action_log(json: &str);

    #[wasm_bindgen(js_namespace = window)]
    pub fn publish_render_frame(json: &str);

}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderFrame {
    pub version: u8,
    pub tick: u64,
    pub entities: Vec<RenderEntityFrame>,
    pub cameras: Vec<RenderCameraFrame>,
    pub map: Option<RenderMapFrame>,
    pub materials: BTreeMap<String, RenderMaterialFrame>,
    pub camera_pose: Option<RenderCameraPoseFrame>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderCameraPoseFrame {
    /// Column-major matrices matching glam and WebGL uniform conventions.
    pub view: [f32; 16],
    pub projection: [f32; 16],
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderMapFrame {
    pub orientation: String,
    pub hex_size: [f32; 2],
    pub tiles: Vec<RenderTileFrame>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderTileFrame {
    pub q: i32,
    pub r: i32,
    pub layer: i32,
    pub bottom: f32,
    pub height: f32,
    pub material: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderMaterialFrame {
    pub color: [f32; 3],
    pub roughness: f32,
    pub metalness: f32,
    pub emissive: f32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderIndicatorFrame {
    pub kind: String,
    pub color: [f32; 3],
    pub state: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderEntityFrame {
    pub id: u64,
    pub kind: String,
    pub team_id: Option<u8>,
    pub q: i32,
    pub r: i32,
    pub layer: i32,
    pub animation_state: String,
    pub animation_time_ms: f32,
    pub animation_frame: Option<u32>,
    pub facing: String,
    pub indicator: RenderIndicatorFrame,
    pub render_order: u64,
    pub asset: Option<String>,
    pub scale: f32,
    pub z: f32,
    pub rotation_z: f32,
    pub rotation_y: f32,
    pub camera_offset: [f32; 3],
    pub slice_indices: Vec<u32>,
    pub selected_slice_index: Option<u32>,
    pub stack_dimensions: [f32; 3],
    pub stack_spacing: f32,
    pub unit_height: f32,
    pub world_position: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderCameraFrame {
    pub id: u64,
    pub angle: f32,
    pub distance: f32,
    pub height: f32,
    pub target: [f32; 3],
}

impl RenderFrame {
    pub fn from_world_state(state: &WorldState, tick: u64) -> Self {
        Self::from_world_state_with_camera(state, tick, None)
    }

    pub fn from_world_state_with_camera(
        state: &WorldState,
        tick: u64,
        camera_pose: Option<RenderCameraPoseFrame>,
    ) -> Self {
        Self::from_world_state_with_camera_and_positions(state, tick, camera_pose, None)
    }

    pub fn from_world_state_with_camera_and_positions(
        state: &WorldState,
        tick: u64,
        camera_pose: Option<RenderCameraPoseFrame>,
        positions: Option<&HashMap<u64, [f32; 3]>>,
    ) -> Self {
        Self::from_world_state_with_camera_positions_and_animation(
            state,
            tick,
            camera_pose,
            positions,
            None,
        )
    }

    pub fn from_world_state_with_camera_positions_and_animation(
        state: &WorldState,
        tick: u64,
        camera_pose: Option<RenderCameraPoseFrame>,
        positions: Option<&HashMap<u64, [f32; 3]>>,
        animation_times: Option<&HashMap<u64, f32>>,
    ) -> Self {
        let (asset_metadata, asset_animations) = state
            .asset_collections
            .get("primitives")
            .map(|data| {
                let collection = pystral_compiler::assets::AssetCollection::from_binary(data);
                let metadata = collection
                    .spritestacks
                    .into_iter()
                    .map(|(name, stack)| {
                        (
                            name,
                            (stack.slices.len(), stack.aabb.to_array(), stack.spacing),
                        )
                    })
                    .collect::<HashMap<_, _>>();
                (metadata, collection.animations)
            })
            .unwrap_or_default();
        let mut entities = state
            .entities
            .iter()
            .filter(|entity| !matches!(entity.kind.as_str(), "world" | "camera"))
            .map(|entity| {
                let asset = entity_asset_name(entity);
                let animation = asset
                    .and_then(|name| asset_animations.get(name))
                    .and_then(|animations| animations.get(&entity.animation_state));
                let animation_frame = animation.and_then(|clip| {
                    resolve_animation_frame(
                        clip,
                        animation_times
                            .and_then(|times| times.get(&entity.id).copied())
                            .unwrap_or(0.0),
                    )
                });
                let layer = match entity.properties.get("layer") {
                    Some(pystral_core::log::PropertyValue::Float(value)) => *value as i32,
                    _ => 0,
                };
                RenderEntityFrame {
                    id: entity.id,
                    kind: entity.kind.clone(),
                    team_id: property_u8(entity, "team_id"),
                    q: entity.hex.x,
                    r: entity.hex.y,
                    layer,
                    animation_state: entity.animation_state.clone(),
                    animation_time_ms: animation_times
                        .and_then(|times| times.get(&entity.id).copied())
                        .unwrap_or(0.0),
                    animation_frame: animation_frame.or_else(|| {
                        asset
                            .and_then(|name| asset_metadata.get(name).map(|meta| meta.0))
                            .and_then(|count| resolved_slice_index(entity, count))
                    }),
                    render_order: entity.id,
                    asset: match entity.properties.get("asset") {
                        Some(pystral_core::log::PropertyValue::String(value))
                        | Some(pystral_core::log::PropertyValue::AssetRef(value)) => {
                            Some(value.clone())
                        }
                        _ => None,
                    },
                    scale: property_float(entity, "scale", 1.0),
                    z: property_float(entity, "z", 0.0),
                    rotation_z: property_float(entity, "rotation_z", 0.0),
                    rotation_y: entity_rotation_y(entity),
                    camera_offset: [
                        property_float(entity, "cam_offset_x", 0.0),
                        property_float(entity, "cam_offset_y", 0.0),
                        property_float(entity, "cam_offset_z", 0.0),
                    ],
                    slice_indices: animation_frame
                        .and_then(|frame| animation.map(|clip| clip.frames[frame as usize].clone()))
                        .or_else(|| {
                            asset
                                .and_then(|name| asset_metadata.get(name).map(|meta| meta.0))
                                .map(|count| (0..count as u32).collect())
                        })
                        .unwrap_or_default(),
                    facing: entity_facing(entity),
                    indicator: RenderIndicatorFrame { kind: "facing".to_string(), color: [0.95, 0.72, 0.22], state: "committed".to_string(), direction: entity_facing(entity) },
                    selected_slice_index: entity_asset_name(entity)
                        .and_then(|asset| asset_metadata.get(asset).map(|meta| meta.0))
                        .and_then(|count| resolved_slice_index(entity, count)),
                    stack_dimensions: asset
                        .and_then(|name| asset_metadata.get(name).map(|meta| meta.1))
                        .unwrap_or([1.0, 0.0, 1.0]),
                    stack_spacing: asset
                        .and_then(|name| asset_metadata.get(name).map(|meta| meta.2))
                        .unwrap_or(0.0),
                    unit_height: pystral_games::collision::CollisionGeometry::default()
                        .unit_height(),
                    world_position: positions.and_then(|map| map.get(&entity.id).copied()),
                }
            })
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| (entity.render_order, entity.id));
        let mut cameras = state
            .entities
            .iter()
            .filter(|entity| entity.kind == "camera")
            .map(|entity| RenderCameraFrame {
                id: entity.id,
                angle: property_float(entity, "angle", 0.0),
                distance: property_float(entity, "distance", 20.0),
                height: property_float(entity, "height", 12.0),
                target: [
                    property_float(entity, "target_x", 0.0),
                    property_float(entity, "target_y", 0.0),
                    property_float(entity, "target_z", 0.0),
                ],
            })
            .collect::<Vec<_>>();
        cameras.sort_by_key(|camera| camera.id);
        let map = (tick == 0)
            .then(|| {
                state
                    .entities
                    .iter()
                    .find(|entity| entity.kind == "world")
                    .and_then(|entity| match entity.properties.get("map") {
                        Some(pystral_core::log::PropertyValue::HexMap(map)) => {
                            Some(RenderMapFrame {
                                orientation: format!("{:?}", map.orientation),
                                hex_size: map.hex_size.to_array(),
                                tiles: map
                                    .tiles
                                    .iter()
                                    .map(|tile| RenderTileFrame {
                                        q: tile.hex.x,
                                        r: tile.hex.y,
                                        layer: tile.layer,
                                        bottom: tile.bottom,
                                        height: tile.height,
                                        material: tile.material.clone(),
                                    })
                                    .collect(),
                            })
                        }
                        _ => None,
                    })
            })
            .flatten();
        let materials = if tick == 0 {
            state
                .materials
                .iter()
                .map(|(name, material)| {
                    (
                        name.clone(),
                        RenderMaterialFrame {
                            color: material.color,
                            roughness: material.roughness,
                            metalness: material.metalness,
                            emissive: material.emissive,
                        },
                    )
                })
                .collect()
        } else {
            BTreeMap::new()
        };
        Self {
            version: 1,
            tick,
            entities,
            cameras,
            map,
            materials,
            camera_pose,
        }
    }
}

fn property_float(entity: &pystral_core::log::EntityState, name: &str, fallback: f32) -> f32 {
    match entity.properties.get(name) {
        Some(pystral_core::log::PropertyValue::Float(value)) if value.is_finite() => *value,
        _ => fallback,
    }
}

fn property_u8(entity: &pystral_core::log::EntityState, name: &str) -> Option<u8> {
    match entity.properties.get(name) {
        Some(pystral_core::log::PropertyValue::Float(value)) if *value >= 0.0 => Some(*value as u8),
        _ => None,
    }
}

fn entity_asset_name(entity: &pystral_core::log::EntityState) -> Option<&str> {
    match entity.properties.get("asset") {
        Some(pystral_core::log::PropertyValue::String(asset))
        | Some(pystral_core::log::PropertyValue::AssetRef(asset)) => Some(asset),
        _ => None,
    }
}

fn entity_rotation_y(entity: &pystral_core::log::EntityState) -> f32 {
    property_float(entity, "rotation_y", 0.0)
}

fn entity_facing(entity: &pystral_core::log::EntityState) -> String {
    match entity.properties.get("facing") {
        Some(pystral_core::log::PropertyValue::String(facing)) => facing.clone(),
        _ => "south".to_string(),
    }
}

fn resolved_slice_index(entity: &pystral_core::log::EntityState, count: usize) -> Option<u32> {
    let value = match entity.properties.get("slice_index") {
        Some(pystral_core::log::PropertyValue::Float(value)) if value.is_finite() => *value,
        _ => return None,
    };
    if count == 0 {
        return None;
    }
    Some(value.round().clamp(0.0, (count - 1) as f32) as u32)
}

pub fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or("Unable to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl.get_shader_info_log(&shader).unwrap_or_default())
    }
}

pub fn link_program(
    gl: &GL,
    vert: &WebGlShader,
    frag: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl.create_program().ok_or("Unable to create program")?;
    gl.attach_shader(&program, vert);
    gl.attach_shader(&program, frag);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl.get_program_info_log(&program).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderCameraFrame, RenderEntityFrame, RenderFrame, RenderIndicatorFrame};
    use hexx::Hex;
    use pystral_compiler::assets::{AssetCollection, SpriteAnimation};
    use pystral_core::domain::{Spritestack, SpritestackSlice};
    use pystral_core::log::{EntityState, PropertyValue, WorldState};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn render_frame_is_presentation_only_and_deterministically_ordered() {
        let mut state = WorldState::default();
        state.entities = vec![
            EntityState::new(4, "character".to_string(), Hex::new(2, 1), &[]),
            EntityState::new(1, "world".to_string(), Hex::ZERO, &[]),
            EntityState::new(3, "camera".to_string(), Hex::ZERO, &[]),
            EntityState::new(2, "character".to_string(), Hex::new(0, 3), &[]),
        ];

        let frame = RenderFrame::from_world_state(&state, 7);

        assert_eq!(frame.version, 1);
        assert_eq!(frame.tick, 7);
        assert_eq!(
            frame.entities,
            vec![
                RenderEntityFrame {
                    id: 2,
                    kind: "character".to_string(),
                    team_id: None,
                    q: 0,
                    r: 3,
                    layer: 0,
                    animation_state: "idle".to_string(),
                    animation_time_ms: 0.0,
                    animation_frame: None,
                    facing: "south".to_string(),
                    indicator: RenderIndicatorFrame { kind: "facing".to_string(), color: [0.95, 0.72, 0.22], state: "committed".to_string(), direction: "south".to_string() },
                    render_order: 2,
                    asset: None,
                    scale: 1.0,
                    z: 0.0,
                    rotation_z: 0.0,
                    rotation_y: 0.0,
                    camera_offset: [0.0; 3],
                    slice_indices: vec![],
                    selected_slice_index: None,
                    stack_dimensions: [1.0, 0.0, 1.0],
                    stack_spacing: 0.0,
                    unit_height: pystral_games::collision::CollisionGeometry::default()
                        .unit_height(),
                    world_position: None,
                },
                RenderEntityFrame {
                    id: 4,
                    kind: "character".to_string(),
                    team_id: None,
                    q: 2,
                    r: 1,
                    layer: 0,
                    animation_state: "idle".to_string(),
                    animation_time_ms: 0.0,
                    animation_frame: None,
                    facing: "south".to_string(),
                    indicator: RenderIndicatorFrame { kind: "facing".to_string(), color: [0.95, 0.72, 0.22], state: "committed".to_string(), direction: "south".to_string() },
                    render_order: 4,
                    asset: None,
                    scale: 1.0,
                    z: 0.0,
                    rotation_z: 0.0,
                    rotation_y: 0.0,
                    camera_offset: [0.0; 3],
                    slice_indices: vec![],
                    selected_slice_index: None,
                    stack_dimensions: [1.0, 0.0, 1.0],
                    stack_spacing: 0.0,
                    unit_height: pystral_games::collision::CollisionGeometry::default()
                        .unit_height(),
                    world_position: None,
                },
            ]
        );
        assert_eq!(
            frame.cameras,
            vec![RenderCameraFrame {
                id: 3,
                angle: 0.0,
                distance: 20.0,
                height: 12.0,
                target: [0.0, 0.0, 0.0],
            }]
        );
    }

    #[test]
    fn render_frame_keeps_rust_resolved_entity_anchor() {
        let mut state = WorldState::default();
        state.entities.push(EntityState::new(
            7,
            "character".to_string(),
            Hex::new(1, 2),
            &[],
        ));
        let mut positions = HashMap::new();
        positions.insert(7, [3.5, 1.25, -2.0]);
        let mut animation_times = HashMap::new();
        animation_times.insert(7, 125.0);

        let frame = RenderFrame::from_world_state_with_camera_positions_and_animation(
            &state,
            0,
            None,
            Some(&positions),
            Some(&animation_times),
        );

        assert_eq!(frame.entities[0].world_position, Some([3.5, 1.25, -2.0]));
        assert_eq!(frame.entities[0].animation_time_ms, 125.0);
        assert_eq!(frame.entities[0].animation_frame, None);
    }

    #[test]
    fn render_frame_clamps_authored_slice_index_to_asset_stack() {
        let mut state = WorldState::default();
        let mut collection = AssetCollection::new();
        collection.spritestacks.insert(
            "hero".to_string(),
            Spritestack {
                width: 1,
                height: 1,
                spacing: 0.1,
                aabb: glam::Vec3::ONE,
                slices: vec![
                    SpritestackSlice {
                        color_data: vec![255, 255, 255, 255],
                        normal_data: vec![128, 128, 255, 255],
                    },
                    SpritestackSlice {
                        color_data: vec![255, 0, 0, 255],
                        normal_data: vec![128, 128, 255, 255],
                    },
                ],
            },
        );
        state
            .asset_collections
            .insert("primitives".to_string(), Arc::new(collection.to_binary()));
        let mut entity = EntityState::new(1, "character".to_string(), Hex::ZERO, &[]);
        entity.properties.insert(
            "asset".to_string(),
            PropertyValue::AssetRef("hero".to_string()),
        );
        entity
            .properties
            .insert("slice_index".to_string(), PropertyValue::Float(99.0));
        state.entities.push(entity);

        let frame = RenderFrame::from_world_state(&state, 0);
        assert_eq!(frame.entities[0].slice_indices, vec![0, 1]);
        assert_eq!(frame.entities[0].selected_slice_index, Some(1));
        assert_eq!(frame.entities[0].animation_frame, Some(1));
    }

    #[test]
    fn render_frame_resolves_animation_frame_and_slices_atomically() {
        let mut state = WorldState::default();
        let mut collection = AssetCollection::new();
        collection.spritestacks.insert(
            "hero".to_string(),
            Spritestack {
                width: 1,
                height: 1,
                spacing: 0.1,
                aabb: glam::Vec3::ONE,
                slices: vec![
                    SpritestackSlice {
                        color_data: vec![255, 255, 255, 255],
                        normal_data: vec![128, 128, 255, 255],
                    },
                    SpritestackSlice {
                        color_data: vec![255, 0, 0, 255],
                        normal_data: vec![128, 128, 255, 255],
                    },
                ],
            },
        );
        collection.animations.insert(
            "hero".to_string(),
            HashMap::from([(
                "idle".to_string(),
                SpriteAnimation {
                    frame_duration_ms: 100,
                    looped: false,
                    frames: vec![vec![1, 0], vec![0, 1]],
                },
            )]),
        );
        state
            .asset_collections
            .insert("primitives".to_string(), Arc::new(collection.to_binary()));
        let mut entity = EntityState::new(1, "character".to_string(), Hex::ZERO, &[]);
        entity.properties.insert(
            "asset".to_string(),
            PropertyValue::AssetRef("hero".to_string()),
        );
        state.entities.push(entity);
        let mut animation_times = HashMap::new();
        animation_times.insert(1, 125.0);

        let frame = RenderFrame::from_world_state_with_camera_positions_and_animation(
            &state,
            0,
            None,
            None,
            Some(&animation_times),
        );

        assert_eq!(frame.entities[0].animation_frame, Some(1));
        assert_eq!(frame.entities[0].slice_indices, vec![0, 1]);
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) -> i32 {
    web_sys::window()
        .expect("No global window found")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK")
}

fn resolve_animation_frame(
    animation: &pystral_compiler::assets::SpriteAnimation,
    elapsed_ms: f32,
) -> Option<u32> {
    if animation.frame_duration_ms == 0 || animation.frames.is_empty() {
        return None;
    }
    let elapsed_ms = elapsed_ms.max(0.0) as u32;
    let frame = (elapsed_ms / animation.frame_duration_ms) as usize;
    let frame = if animation.looped {
        frame % animation.frames.len()
    } else {
        frame.min(animation.frames.len() - 1)
    };
    Some(frame as u32)
}

pub struct RenderLoop {
    active: Rc<std::cell::Cell<bool>>,
    frame: Rc<std::cell::Cell<i32>>,
    _callback: Rc<RefCell<Option<Closure<dyn FnMut()>>>>,
}

impl RenderLoop {
    pub fn cancel(&self) {
        self.active.set(false);
        let _ = web_sys::window()
            .expect("No global window found")
            .cancel_animation_frame(self.frame.get());
    }
}

pub fn start_render_loop(
    history_manager: HistoryManager,
    app_rx: std::sync::mpsc::Receiver<crate::AppCommand>,
    worker_tx: futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
) -> RenderLoop {
    let ctx = RenderContext::new();
    let handler = Rc::new(RefCell::new(LoopHandler::new(
        ctx,
        history_manager,
        app_rx,
        worker_tx,
    )));

    let active = Rc::new(std::cell::Cell::new(true));
    let frame = Rc::new(std::cell::Cell::new(0));
    let callback: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let weak_callback: Weak<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::downgrade(&callback);
    let active_for_callback = active.clone();
    let frame_for_callback = frame.clone();
    *callback.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if !active_for_callback.get() {
            return;
        }
        handler.borrow_mut().tick();
        if let Some(callback) = weak_callback.upgrade() {
            if let Some(callback) = callback.borrow().as_ref() {
                frame_for_callback.set(request_animation_frame(callback));
            }
        }
    }) as Box<dyn FnMut()>));

    let initial_frame = request_animation_frame(callback.borrow().as_ref().expect("callback"));
    frame.set(initial_frame);
    RenderLoop {
        active,
        frame,
        _callback: callback,
    }
}
