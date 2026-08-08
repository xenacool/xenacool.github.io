use crate::demo::simulation::TacticalSimulation;
use rhai::{Engine, Scope, Dynamic};
use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue, TransitionConfig, TweenKind};
use pystral_core::domain::{HexMap, HexTile, Material, LightingConfig, Spritestack, SpritestackSlice};
use pystral_core::animation::{InactiveFSMDefinition, AnimationState, PropertyTrack, LoopBehavior};
use hexx::Hex;
use glam::{Vec2, Vec3};
use std::collections::HashMap;
use pystral_compiler::physics::TrajectorySystem;
use pystral_compiler::assets::{AssetCollection, SpriteAtlas};

pub fn generate_demo_log_rhai(
    history: &mut HistoryManager, 
    script: &str,
    atlas_json: &str,
    spritesheet_rgba: &[u8],
    spritesheet_width: u32
) -> Result<(), Box<rhai::EvalAltResult>> {
    let mut engine = Engine::new();

    register_all(&mut engine);

    let mut scope = Scope::new();
    scope.push("history", history.clone());
    scope.push("atlas_json", atlas_json.to_string());
    scope.push("spritesheet_rgba", rhai::Blob::from(spritesheet_rgba.to_vec()));
    scope.push("spritesheet_width", spritesheet_width as i64);
    scope.push("run_simulation", true);

    engine.run_with_scope(&mut scope, script)?;

    if let Some(h) = scope.get_value::<HistoryManager>("history") {
        *history = h;
    }

    Ok(())
}

fn register_basic_types(engine: &mut Engine) {
    // Register basic types
    engine.register_type_with_name::<Hex>("Hex")
        .register_fn("hex", |q: i64, r: i64| Hex::new(q as i32, r as i32))
        .register_fn("hex_to_world", |hex: Hex| {
            let layout = hexx::HexLayout {
                orientation: hexx::HexOrientation::Pointy,
                scale: Vec2::splat(1.0),
                origin: Vec2::ZERO,
            };
            let pos = layout.hex_to_world_pos(hex);
            Vec3::new(pos.x, 0.0, pos.y)
        });

    engine.register_type_with_name::<Vec3>("Vec3")
        .register_fn("vec3", |x: f64, y: f64, z: f64| Vec3::new(x as f32, y as f32, z as f32))
        .register_get("x", |v: &mut Vec3| v.x as f64)
        .register_get("y", |v: &mut Vec3| v.y as f64)
        .register_get("z", |v: &mut Vec3| v.z as f64);
}

fn register_domain_types(engine: &mut Engine) {
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
        .register_fn("new_hex_map", || HexMap::new())
        .register_fn("add_tile", |map: &mut HexMap, tile: HexTile| map.tiles.push(tile));

    engine.register_type_with_name::<HexTile>("HexTile")
        .register_fn("new_hex_tile", |hex: Hex, bottom: f64, height: f64, material: &str| HexTile {
            hex,
            bottom: bottom as f32,
            height: height as f32,
            material: material.to_string(),
        });
}

fn register_animation_types(engine: &mut Engine) {
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
}

fn register_asset_management(engine: &mut Engine) {
    // Register Asset Management
    engine.register_type_with_name::<SpriteAtlas>("SpriteAtlas")
        .register_fn("load_atlas", |json: &str| {
            SpriteAtlas::from_json(json).unwrap_or_else(|_| SpriteAtlas { width: 0, height: 0, spritestacks: HashMap::new() })
        });

    engine.register_type_with_name::<AssetCollection>("AssetCollection")
        .register_fn("new_asset_collection", || AssetCollection::new())
        .register_fn("add_atlas_spritestack", |collection: &mut AssetCollection, name: &str, spacing: f64, atlas: SpriteAtlas, spritesheet_rgba: rhai::Blob, width: i64| {
             if atlas.spritestacks.contains_key(name) {
                 collection.add_atlas_spritestack(name, spacing as f32, &atlas, &spritesheet_rgba, width as u32);
             } else {
                 // Log error to history if possible, but here we don't have history directly.
                 // For now, let's just avoid panicking.
             }
        })
        .register_fn("add_arrow", |collection: &mut AssetCollection, name: &str, color: rhai::Array, spacing: f64| {
            let r = color.get(0).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let g = color.get(1).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let b = color.get(2).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let a = color.get(3).and_then(|v| v.as_int().ok()).unwrap_or(255) as u8;
            let color = [r, g, b, a];
            let spacing = spacing as f32;

            let resolution = 32;
            let mut slices = Vec::new();
            let pixel_count = (resolution * resolution) as usize;
            
            for i in 0..resolution {
                let mut color_data = vec![0u8; pixel_count * 4];
                let mut normal_data = vec![0u8; pixel_count * 4];
                
                for y in 0..resolution {
                    for x in 0..resolution {
                        let idx = (y * resolution + x) as usize * 4;
                        
                        let fx = x as f32 / (resolution - 1) as f32;
                        let fy = y as f32 / (resolution - 1) as f32;
                        let fi = i as f32 / (resolution - 1) as f32;
                        
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

            add_to_sprite_stacks(collection, name, spacing, resolution, resolution, slices);
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

            add_to_sprite_stacks(collection, name, spacing, res, res, slices);
        });

    engine.register_fn("define_asset_collection", |history: &mut HistoryManager, name: &str, collection: AssetCollection| {
        history.push_and_apply(Event::DefineAssetCollection {
            name: name.to_string(),
            data: collection.to_binary(),
        });
    });
}

fn add_to_sprite_stacks(collection: &mut AssetCollection, name: &str, spacing: f32, width: u32, height: u32, slices: Vec<SpritestackSlice>) {
    collection.spritestacks.insert(name.to_string(), Spritestack {
        width,
        height,
        spacing,
        aabb: Vec3::new(
            (width as f32 - 0.5) * spacing,
            (slices.len() as f32 - 1.0) * spacing,
            (height as f32 - 0.5) * spacing,
        ),
        slices,
    });
}

fn register_png_assets(engine: &mut Engine) {
    engine.register_fn("add_png_spritestack", |collection: &mut AssetCollection, name: &str, spacing: f64, atlas: SpriteAtlas, spritesheet_rgba: rhai::Blob, width: i64| {
        collection.add_atlas_spritestack(name, spacing as f32, &atlas, &spritesheet_rgba, width as u32);
    });
}

fn register_physics(engine: &mut Engine) {
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
}

fn register_history_methods(engine: &mut Engine) {
    // Register HistoryManager methods
    engine.register_type_with_name::<HistoryManager>("History");
    engine.register_type_with_name::<TransitionConfig>("Transition")
        .register_fn("transition", |duration_ms: i64, delta_time_ms: f64, tween: &str| {
            TransitionConfig {
                duration_ms: duration_ms.max(1) as u32,
                delta_time_ms: delta_time_ms.max(0.0) as f32,
                tween: match tween { "SineInOut" => TweenKind::SineInOut, _ => TweenKind::SineInOut },
            }
        });
    engine.register_fn("configure_transition", |history: &mut HistoryManager, id: i64, config: TransitionConfig| {
        history.push_and_apply(Event::ConfigureTransition { id: id as u64, config });
    });

    engine.register_fn("spawn_entity", |history: &mut HistoryManager, id: i64, kind: &str, hex: Hex| {
        history.push_and_apply(Event::SpawnEntity {
            id: id as u64,
            kind: kind.to_string(),
            hex,
        });
        id
    });

    engine.register_fn("despawn_entity", |history: &mut HistoryManager, id: i64| {
        history.push_and_apply(Event::DespawnEntity { id: id as u64 });
    });

    engine.register_fn("set", |history: &mut HistoryManager, id: i64, prop: &str, value: Dynamic| {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Setting property {} for entity {} to {:?}", prop, id, value).into());

        let val = if let Some(f) = value.as_float().ok() {
            PropertyValue::Float(f as f32)
        } else if let Some(i) = value.as_int().ok() {
            PropertyValue::Float(i as f32)
        } else if let Some(b) = value.as_bool().ok() {
            PropertyValue::String(if b { "true".to_string() } else { "false".to_string() })
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

    engine.register_fn("move_sprite", |history: &mut HistoryManager, id: i64, destination: Hex, transition: TransitionConfig| {
        history.push_and_apply(Event::MoveSprite {
            id: id as u64,
            destination,
            transition: Some(transition),
        });
    });

    engine.register_fn("set_animation_state", |history: &mut HistoryManager, id: i64, state: &str| {
        history.push_and_apply(Event::SetAnimationState {
            id: id as u64,
            state: state.to_string(),
        });
    });

    engine.register_fn("segno", |history: &mut HistoryManager, n: i64| {
        history.push_and_apply(Event::Segno(n as u64));
    });
}

fn register_simulation(engine: &mut Engine) {
    engine.register_type_with_name::<TacticalSimulation>("TacticalSimulation");

    engine.register_fn("new_simulation", TacticalSimulation::new);
    
    engine.register_fn("step", |sim: &mut TacticalSimulation| {
        let agents = sim.step();
        agents.into_iter().map(|id| Dynamic::from(id.0 as i64)).collect::<rhai::Array>()
    });
    
    engine.register_fn("get_agent_position", |sim: &mut TacticalSimulation, id: i64| {
        let pos = sim.get_agent_position(id);
        Hex::new(pos.0, pos.1) // We only care about Q, R for now
    });
    
    engine.register_fn("get_agent_health", |sim: &mut TacticalSimulation, id: i64| {
        sim.get_agent_health(id)
    });
    
    engine.register_fn("get_prompts", |sim: &mut TacticalSimulation, id: i64| {
        let prompts = sim.get_prompts(id);
        let mut map = rhai::Map::new();
        for (k, v) in prompts {
            map.insert(k.into(), v.into());
        }
        map
    });
    
    engine.register_fn("list_agents", |sim: &mut TacticalSimulation| {
        let agents = sim.list_agents();
        agents.into_iter().map(Dynamic::from).collect::<rhai::Array>()
    });
}

pub fn register_all(engine: &mut Engine) {
    engine.set_max_expr_depths(500, 500);
    engine.set_max_operations(1000000);
    engine.set_max_variables(1000);

    register_basic_types(engine);
    register_domain_types(engine);
    register_animation_types(engine);
    register_asset_management(engine);
    register_png_assets(engine);
    register_physics(engine);
    register_history_methods(engine);
    register_simulation(engine);
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
        let result = generate_demo_log_rhai(&mut history, script, "", &[], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_error_script() {
        let mut history = HistoryManager::new();
        let script = "history.spawn(\"world\", hex(0, 0)); history.non_existent_function();";
        let result = generate_demo_log_rhai(&mut history, script, "", &[], 0);
        assert!(result.is_err());
    }
}
