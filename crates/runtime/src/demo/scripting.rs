use rhai::{Engine, Scope, Dynamic};
use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue};
use pystral_core::domain::{HexMap, Material, LightingConfig, Spritestack, SpritestackSlice};
use pystral_core::animation::{InactiveFSMDefinition, AnimationState, PropertyTrack, LoopBehavior};
use hexx::Hex;
use glam::Vec3;
use std::collections::HashMap;
use pystral_compiler::physics::TrajectorySystem;
use pystral_compiler::assets::AssetCollection;
use pystral_macros::include_layers;

pub fn generate_demo_log_rhai(history: &mut HistoryManager, script: &str) -> Result<(), Box<rhai::EvalAltResult>> {
    let mut engine = Engine::new();

    // Register basic types
    engine.register_type_with_name::<Hex>("Hex")
        .register_fn("hex", |q: i64, r: i64| Hex::new(q as i32, r as i32));

    engine.register_type_with_name::<Vec3>("Vec3")
        .register_fn("vec3", |x: f64, y: f64, z: f64| Vec3::new(x as f32, y as f32, z as f32))
        .register_get("x", |v: &mut Vec3| v.x as f64)
        .register_get("y", |v: &mut Vec3| v.y as f64)
        .register_get("z", |v: &mut Vec3| v.z as f64);

    // Register Domain Types
    engine.register_type_with_name::<Material>("Material")
        .register_fn("material", |r: f64, g: f64, b: f64| Material {
            color: [r as f32, g as f32, b as f32],
            roughness: 0.5,
            metalness: 0.0,
            emissive: 0.0,
        });

    engine.register_type_with_name::<LightingConfig>("LightingConfig")
        .register_fn("default_lighting", || LightingConfig::default());

    engine.register_type_with_name::<HexMap>("HexMap")
        .register_fn("create_demo_map", || crate::demo::world::create_demo_world());

    // Register Animation Types
    engine.register_type_with_name::<LoopBehavior>("LoopBehavior");
    engine.register_static_module("LoopBehavior", rhai::exported_module!(loop_behavior_module).into());

    engine.register_type_with_name::<InactiveFSMDefinition>("FSM")
        .register_fn("new_fsm", || InactiveFSMDefinition { states: HashMap::new() })
        .register_fn("add_state", |fsm: &mut InactiveFSMDefinition, name: &str, state: AnimationState| {
            fsm.states.insert(name.to_string(), state);
        });

    engine.register_type_with_name::<AnimationState>("AnimationState")
        .register_fn("new_animation_state", |name: &str| AnimationState { name: name.to_string(), tracks: Vec::new() })
        .register_fn("add_track", |state: &mut AnimationState, track: PropertyTrack| {
            state.tracks.push(track);
        });

    engine.register_type_with_name::<PropertyTrack>("PropertyTrack");


    // Register Asset Management
    engine.register_type_with_name::<AssetCollection>("AssetCollection")
        .register_fn("new_asset_collection", || AssetCollection::new())
        .register_fn("add_arrow", |collection: &mut AssetCollection, name: &str, color: rhai::Array, spacing: f64| {
            let r = color.get(0).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let g = color.get(1).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let b = color.get(2).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let a = color.get(3).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let color = [r, g, b, a];
            let spacing = spacing as f32;

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
            
            collection.spritestacks.insert(name.to_string(), Spritestack {
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
        })
        .register_fn("add_rock", |collection: &mut AssetCollection, name: &str, size: i64, color: rhai::Array, spacing: f64| {
            let r = color.get(0).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let g = color.get(1).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let b = color.get(2).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let a = color.get(3).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let color = [r, g, b, a];
            let spacing = spacing as f32;
            let res = size as u32;
            let layers = res;

            let mut slices = Vec::new();
            let pixel_count = (res * res) as usize;
            
            for i in 0..layers {
                let mut color_data = vec![0u8; pixel_count * 4];
                let mut normal_data = vec![0u8; pixel_count * 4];
                
                for y in 0..res {
                    for x in 0..res {
                        let idx = (y * res + x) as usize * 4;
                        
                        let fx = x as f32 / (res as f32 - 1.0).max(1.0) - 0.5;
                        let fy = y as f32 / (res as f32 - 1.0).max(1.0) - 0.5;
                        let fi = i as f32 / (layers as f32 - 1.0).max(1.0) - 0.5;
                        
                        let dist = (fx*fx + fy*fy + fi*fi).sqrt();
                        let limit = 0.45;
                        
                        if dist < limit {
                            color_data[idx..idx+4].copy_from_slice(&color);
                            
                            let nx = if dist > 0.0 { fx / dist } else { 0.0 };
                            let ny = if dist > 0.0 { fi / dist } else { 1.0 };
                            let nz = if dist > 0.0 { fy / dist } else { 0.0 };
                            
                            normal_data[idx] = ((nx * 0.5 + 0.5) * 255.0) as u8;
                            normal_data[idx+1] = ((ny * 0.5 + 0.5) * 255.0) as u8;
                            normal_data[idx+2] = ((nz * 0.5 + 0.5) * 255.0) as u8;
                            normal_data[idx+3] = 255;
                        }
                    }
                }
                
                slices.push(SpritestackSlice { color_data, normal_data });
            }
            
            collection.spritestacks.insert(name.to_string(), Spritestack {
                width: res,
                height: res,
                spacing,
                aabb: Vec3::new(
                    (res as f32 - 0.5) * spacing,
                    (slices.len() as f32 - 1.0) * spacing,
                    (res as f32 - 0.5) * spacing,
                ),
                slices,
            });
        });

    engine.register_fn("add_png_spritestack", |collection: &mut AssetCollection, name: &str, path: &str, spacing: f64, height: f64| {
        let name_to_load = if path.contains("skeleton_minion") {
            "SkeletonMinion"
        } else if path.contains("necromancer") {
            "Necromancer"
        } else if path.contains("caveman") {
            "Caveman"
        } else if path.contains("mage") {
            "Mage"
        } else {
            name
        };

        match name_to_load {
            "SkeletonMinion" => {
                let layers = include_layers!(1..=300, "../../../../assets/spritestacks/skeleton_minion/layer-{}.png").to_vec();
                collection.add_png_spritestack(name, spacing as f32, layers);
                if let Some(stack) = collection.spritestacks.get_mut(name) {
                    stack.aabb.y = height as f32;
                }
            }
            "Necromancer" => {
                let layers = include_layers!(1..=300, "../../../../assets/spritestacks/necromancer/layer-{}.png").to_vec();
                collection.add_png_spritestack(name, spacing as f32, layers);
                if let Some(stack) = collection.spritestacks.get_mut(name) {
                    stack.aabb.y = height as f32;
                }
            }
            "Caveman" => {
                let layers = include_layers!(1..=300, "../../../../assets/spritestacks/caveman/layer-{}.png").to_vec();
                collection.add_png_spritestack(name, spacing as f32, layers);
                if let Some(stack) = collection.spritestacks.get_mut(name) {
                    stack.aabb.y = height as f32;
                }
            }
            "Mage" => {
                let layers = include_layers!(1..=300, "../../../../assets/spritestacks/mage/layer-{}.png").to_vec();
                collection.add_png_spritestack(name, spacing as f32, layers);
                if let Some(stack) = collection.spritestacks.get_mut(name) {
                    stack.aabb.y = height as f32;
                }
            }
            _ => {}
        }
    });


    engine.register_fn("define_asset_collection", |history: &mut HistoryManager, name: &str, collection: AssetCollection| {
        history.push_and_apply(Event::DefineAssetCollection {
            name: name.to_string(),
            data: collection.to_binary(),
        });
    });

    // Physics
    engine.register_type_with_name::<TrajectorySystem>("TrajectorySystem")
        .register_fn("new_trajectory_system", || TrajectorySystem::new());

    engine.register_fn("generate_arrow_tracks", |ts: TrajectorySystem, start: Vec3, target: Vec3, map: HexMap| {
        let tracks = crate::demo::animation::generate_arrow_tracks(&ts, start, target, &map);
        let mut arr = rhai::Array::new();
        for t in tracks {
            arr.push(rhai::Dynamic::from(t));
        }
        arr
    });

    // Register HistoryManager methods
    engine.register_type_with_name::<HistoryManager>("History");

    engine.register_fn("spawn_entity", |history: &mut HistoryManager, id: i64, kind: &str, hex: Hex| {
        history.push_and_apply(Event::SpawnEntity {
            id: id as u64,
            kind: kind.to_string(),
            hex,
        });
        id
    });

    engine.register_fn("spawn_entity", |history: &mut HistoryManager, kind: &str, hex: Hex| {
        let id = history.log.len() as u64;
        history.push_and_apply(Event::SpawnEntity {
            id,
            kind: kind.to_string(),
            hex,
        });
        id as i64
    });

    engine.register_fn("set", |history: &mut HistoryManager, id: i64, prop: &str, value: Dynamic| {
        let val = if let Some(f) = value.as_float().ok() {
            PropertyValue::Float(f as f32)
        } else if let Some(s) = value.clone().into_string().ok() {
            PropertyValue::String(s)
        } else if value.is::<Vec3>() {
            PropertyValue::Vec3(value.cast::<Vec3>())
        } else if value.is::<HexMap>() {
            PropertyValue::HexMap(value.cast::<HexMap>())
        } else if let Ok(v) = rhai::serde::from_dynamic::<PropertyValue>(&value) {
            v
        } else {
            if let Ok(m) = rhai::serde::from_dynamic::<Material>(&value) {
                PropertyValue::Material(m)
            } else if let Ok(l) = rhai::serde::from_dynamic::<LightingConfig>(&value) {
                PropertyValue::Lighting(l)
            } else if let Ok(_fsm) = rhai::serde::from_dynamic::<InactiveFSMDefinition>(&value) {
                PropertyValue::String("".into()) // This is not quite right, but we'll fix it if needed
            } else {
                return;
            }
        };

        history.push_and_apply(Event::UpdateProperty {
            id: id as u64,
            property: prop.to_string(),
            value: val,
        });
    });

    engine.register_fn("define_fsm", |history: &mut HistoryManager, name: &str, definition: InactiveFSMDefinition| {
        history.push_and_apply(Event::DefineFSM {
            name: name.to_string(),
            definition,
        });
    });

    engine.register_fn("define_material", |history: &mut HistoryManager, name: &str, material: Material| {
        history.push_and_apply(Event::DefineMaterial {
            name: name.to_string(),
            material,
        });
    });

    engine.register_fn("set_animation_state", |history: &mut HistoryManager, id: i64, state: &str| {
        history.push_and_apply(Event::SetAnimationState {
            id: id as u64,
            state: state.to_string(),
        });
    });

    let mut scope = Scope::new();
    scope.push("history", history.clone());

    engine.run_with_scope(&mut scope, script)?;

    if let Some(h) = scope.get_value::<HistoryManager>("history") {
        *history = h;
    }

    Ok(())
}

#[rhai::plugin::export_module]
mod loop_behavior_module {
    pub const NONE: LoopBehavior = LoopBehavior::None;
    pub const LOOP: LoopBehavior = LoopBehavior::Loop;
    pub const PING_PONG: LoopBehavior = LoopBehavior::PingPong;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_script() {
        let mut history = HistoryManager::new();
        let script = "syntax error here";
        let result = generate_demo_log_rhai(&mut history, script);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_error_script() {
        let mut history = HistoryManager::new();
        let script = "history.spawn(\"world\", hex(0, 0)); history.non_existent_function();";
        let result = generate_demo_log_rhai(&mut history, script);
        assert!(result.is_err());
    }
}
