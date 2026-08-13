mod assets;
mod rules;
mod simulation;
use crate::demo::simulation::TacticalSimulation;
use glam::{Vec2, Vec3};
use hexx::Hex;
use npc_engine_core::MCTSConfiguration;
use pystral_compiler::physics::{ProjectileCollider, TrajectoryRequest, TrajectorySystem};
use pystral_core::animation::{AnimationState, InactiveFSMDefinition, LoopBehavior, PropertyTrack};
use pystral_core::domain::{HexMap, HexTile, LightingConfig, Material};
use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue, TransitionConfig, TweenKind};
use pystral_games::{GridCell, GridMap, TileType};
use rhai::{CallFnOptions, Dynamic, Engine, Scope};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct RhaiMctsConfig {
    visits: i64,
    depth: i64,
    seed: Option<i64>,
    minimum_hit_probability: f32,
    allow_desperation: bool,
}

impl RhaiMctsConfig {
    fn to_native(&self) -> Result<MCTSConfiguration, String> {
        let mut config = MCTSConfiguration::default();
        config.visits = self.visits.max(0) as u32;
        config.depth = self.depth.max(0) as u32;
        config.seed = self.seed.map(|seed| seed.max(0) as u64);
        Ok(config)
    }
}

pub fn generate_demo_log_rhai(
    history: &mut HistoryManager,
    script: &str,
    atlas_json: &str,
    spritesheet_rgba: &[u8],
    spritesheet_width: u32,
) -> Result<(), Box<rhai::EvalAltResult>> {
    let mut engine = Engine::new();

    register_all(&mut engine);

    let mut scope = Scope::new();
    scope.push("history", history.clone());
    scope.push("atlas_json", atlas_json.to_string());
    scope.push(
        "spritesheet_rgba",
        rhai::Blob::from(spritesheet_rgba.to_vec()),
    );
    scope.push("spritesheet_width", spritesheet_width as i64);
    let ast = engine.compile(script)?;
    let _: Dynamic = engine.eval_ast_with_scope(&mut scope, &ast)?;
    let mut completed = false;
    for step in 0..32 {
        let ready: rhai::Array = engine.call_fn_with_options(
            CallFnOptions::new().rewind_scope(true),
            &mut scope,
            &ast,
            "resume_game",
            (),
        )?;
        let mut simulation = scope
            .get_value::<TacticalSimulation>("sim")
            .ok_or_else(|| "Rhai demo did not define sim".to_string())?;
        // Static history generation exercises the same NPC action and
        // revalidation path as live play, but uses a bounded search budget so
        // asset/render tests do not run the production search repeatedly.
        simulation.config.visits = simulation.config.visits.min(1);
        simulation.config.depth = simulation.config.depth.min(1);
        for value in ready {
            let agent = npc_engine_core::AgentId(
                value
                    .as_int()
                    .map_err(|error| format!("Invalid Rhai agent ID: {error}"))?
                    as u32,
            );
            let team_id = simulation
                .state
                .agents
                .get(&agent)
                .map(|unit| unit.team_id)
                .ok_or_else(|| format!("Rhai returned unknown agent {}", agent.0))?;
            let action = if team_id == 1 {
                pystral_games::TacticalDisplayAction::Wait
            } else {
                simulation
                    .request_npc_decision(agent)
                    .or_else(|| simulation.fallback_npc_action(agent))
                    .ok_or_else(|| format!("NPC {agent:?} has no legal demo action"))?
            };
            if simulation.apply_npc_action(agent, action).is_err() {
                let fallback = simulation
                    .fallback_npc_action(agent)
                    .ok_or_else(|| format!("NPC {agent:?} fallback is not legal"))?;
                simulation
                    .apply_npc_action(agent, fallback)
                    .map_err(|error| format!("NPC {agent:?} fallback failed: {error}"))?;
            }
        }
        completed = simulation.is_complete();
        scope.set_value("sim", simulation);
        if completed {
            let mut generated_history = scope
                .get_value::<HistoryManager>("history")
                .ok_or_else(|| "Rhai demo did not define history".to_string())?;
            generated_history.push_and_apply(Event::Log {
                msg: format!("Rhai demo full playout completed at boundary {step}"),
            });
            scope.set_value("history", generated_history);
            break;
        }
    }
    if !completed {
        let mut generated_history = scope
            .get_value::<HistoryManager>("history")
            .ok_or_else(|| "Rhai demo did not define history".to_string())?;
        generated_history.push_and_apply(Event::Log {
            msg: "Rhai demo NPC playout reached its bounded preview window".to_string(),
        });
        scope.set_value("history", generated_history);
    }

    if let Some(h) = scope.get_value::<HistoryManager>("history") {
        *history = h;
    }

    Ok(())
}

fn register_basic_types(engine: &mut Engine) {
    // Register basic types
    engine
        .register_type_with_name::<Hex>("Hex")
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

    engine
        .register_type_with_name::<Vec3>("Vec3")
        .register_fn("vec3", |x: f64, y: f64, z: f64| {
            Vec3::new(x as f32, y as f32, z as f32)
        })
        .register_get("x", |v: &mut Vec3| v.x as f64)
        .register_get("y", |v: &mut Vec3| v.y as f64)
        .register_get("z", |v: &mut Vec3| v.z as f64);
}

fn register_domain_types(engine: &mut Engine) {
    // Register Domain Types
    engine
        .register_type_with_name::<Material>("Material")
        .register_fn("material", |r: f64, g: f64, b: f64| Material {
            color: [r as f32, g as f32, b as f32],
            roughness: 0.5,
            metalness: 0.0,
            emissive: 0.0,
        });

    engine
        .register_type_with_name::<LightingConfig>("LightingConfig")
        .register_fn("default_lighting", || LightingConfig::default());

    engine
        .register_type_with_name::<HexMap>("HexMap")
        .register_fn("new_hex_map", || HexMap::new())
        .register_fn("add_tile", |map: &mut HexMap, tile: HexTile| {
            map.tiles.push(tile)
        });

    engine
        .register_type_with_name::<GridCell>("GridCell")
        .register_fn("cell", |hex: Hex, layer: i64| {
            GridCell::new(hex, layer as i32)
        })
        .register_get("hex", |cell: &mut GridCell| cell.hex)
        .register_get("layer", |cell: &mut GridCell| cell.layer as i64);

    engine
        .register_type_with_name::<GridMap>("GridMap")
        .register_fn("new_grid_map", || GridMap::default())
        .register_fn(
            "set_horizontal_bounds",
            |grid: &mut GridMap, center: Hex, radius: i64| {
                grid.bounds.horizontal = hexx::HexBounds::new(center, radius.max(0) as u32);
            },
        )
        .register_fn(
            "set_layer_bounds",
            |grid: &mut GridMap, min_layer: i64, max_layer: i64| {
                grid.bounds.min_layer = min_layer as i32;
                grid.bounds.max_layer = max_layer as i32;
            },
        )
        .register_fn(
            "set_tile",
            |grid: &mut GridMap, cell: GridCell, name: &str| {
                let tile = TileType::from_name(name)
                    .ok_or_else(|| format!("Unknown tile type: {name}"))?;
                grid.set_tile(cell, tile).map_err(|error| error)
            },
        );
    engine.register_fn("cell", |hex: Hex, layer: i64| {
        GridCell::new(hex, layer as i32)
    });
    engine.register_fn("new_grid_map", || GridMap::default());

    engine
        .register_type_with_name::<HexTile>("HexTile")
        .register_fn(
            "new_hex_tile",
            |hex: Hex, bottom: f64, height: f64, material: &str| HexTile {
                hex,
                layer: 0,
                bottom: bottom as f32,
                height: height as f32,
                material: material.to_string(),
            },
        )
        .register_fn(
            "new_layered_hex_tile",
            |hex: Hex, layer: i64, bottom: f64, height: f64, material: &str| HexTile {
                hex,
                layer: layer as i32,
                bottom: bottom as f32,
                height: height as f32,
                material: material.to_string(),
            },
        );
}

fn register_animation_types(engine: &mut Engine) {
    // Register Animation Types
    engine.register_type_with_name::<LoopBehavior>("LoopBehavior");
    engine.register_static_module(
        "LoopBehavior",
        rhai::exported_module!(loop_behavior_module).into(),
    );

    engine
        .register_type_with_name::<InactiveFSMDefinition>("FSM")
        .register_fn("new_fsm", || InactiveFSMDefinition {
            states: HashMap::new(),
        })
        .register_fn(
            "add_state",
            |fsm: &mut InactiveFSMDefinition, name: &str, state: AnimationState| {
                fsm.states.insert(name.to_string(), state);
            },
        );

    engine
        .register_type_with_name::<AnimationState>("AnimationState")
        .register_fn("new_animation_state", |name: &str| AnimationState {
            name: name.to_string(),
            tracks: Vec::new(),
        })
        .register_fn(
            "add_track",
            |state: &mut AnimationState, track: PropertyTrack| {
                state.tracks.push(track);
            },
        );

    engine.register_type_with_name::<PropertyTrack>("PropertyTrack");
}

fn register_physics(engine: &mut Engine) {
    // Physics
    engine
        .register_type_with_name::<TrajectorySystem>("TrajectorySystem")
        .register_fn("new_trajectory_system", || TrajectorySystem::new());

    engine
        .register_type_with_name::<ProjectileCollider>("ProjectileCollider")
        .register_fn("ball_collider", |radius: f64| ProjectileCollider::Ball {
            radius: radius as f32,
        })
        .register_fn(
            "capsule_collider",
            |segment_half_height: f64, radius: f64| ProjectileCollider::Capsule {
                segment_half_height: segment_half_height as f32,
                radius: radius as f32,
            },
        );

    engine
        .register_type_with_name::<TrajectoryRequest>("TrajectoryRequest")
        .register_fn("new_trajectory_request", TrajectoryRequest::new)
        .register_fn(
            "set_speed_range",
            |request: &mut TrajectoryRequest, min: f64, max: f64, step: f64| {
                request.speed_min = min as f32;
                request.speed_max = max as f32;
                request.speed_step = step as f32;
            },
        )
        .register_fn(
            "set_angle_range",
            |request: &mut TrajectoryRequest, min: f64, max: f64, step: f64| {
                request.angle_min_degrees = min as f32;
                request.angle_max_degrees = max as f32;
                request.angle_step_degrees = step as f32;
            },
        )
        .register_fn(
            "set_gravity",
            |request: &mut TrajectoryRequest, gravity: f64| request.gravity = gravity as f32,
        )
        .register_fn(
            "set_time_step",
            |request: &mut TrajectoryRequest, time_step: f64| request.time_step = time_step as f32,
        )
        .register_fn(
            "set_max_steps",
            |request: &mut TrajectoryRequest, max_steps: i64| {
                request.max_steps = max_steps.max(0) as u32
            },
        )
        .register_fn(
            "set_ground_cutoff",
            |request: &mut TrajectoryRequest, cutoff: f64| request.ground_cutoff = cutoff as f32,
        )
        .register_fn(
            "set_collider",
            |request: &mut TrajectoryRequest, collider: ProjectileCollider| {
                request.collider = collider
            },
        );

    engine.register_fn(
        "generate_arrow_tracks",
        |ts: TrajectorySystem, request: TrajectoryRequest, map: HexMap| {
            let tracks = crate::demo::animation::generate_arrow_tracks(&ts, request, &map);
            let mut arr = rhai::Array::new();
            for t in tracks {
                arr.push(rhai::Dynamic::from(t));
            }
            arr
        },
    );
}

fn register_history_methods(engine: &mut Engine) {
    // Register HistoryManager methods
    engine.register_type_with_name::<HistoryManager>("History");
    engine
        .register_type_with_name::<TransitionConfig>("Transition")
        .register_fn(
            "transition",
            |duration_ms: i64, delta_time_ms: f64, tween: &str| TransitionConfig {
                duration_ms: duration_ms.max(1) as u32,
                delta_time_ms: delta_time_ms.max(0.0) as f32,
                tween: match tween {
                    "SineInOut" => TweenKind::SineInOut,
                    _ => TweenKind::SineInOut,
                },
            },
        );
    engine.register_fn(
        "configure_transition",
        |history: &mut HistoryManager, id: i64, config: TransitionConfig| {
            history.push_and_apply(Event::ConfigureTransition {
                id: id as u64,
                config,
            });
        },
    );

    engine.register_fn(
        "spawn_entity",
        |history: &mut HistoryManager,
         id: i64,
         kind: &str,
         hex: Hex,
         init_properties: rhai::Array| {
            let init_properties = init_properties
                .into_iter()
                .filter_map(|value| value.into_string().ok())
                .collect();
            history.push_and_apply(Event::SpawnEntity {
                id: id as u64,
                kind: kind.to_string(),
                hex,
                init_properties,
            });
            id
        },
    );

    engine.register_fn("despawn_entity", |history: &mut HistoryManager, id: i64| {
        history.push_and_apply(Event::DespawnEntity { id: id as u64 });
    });

    engine.register_fn(
        "set",
        |history: &mut HistoryManager, id: i64, prop: &str, value: Dynamic| {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(
                &format!("Setting property {} for entity {} to {:?}", prop, id, value).into(),
            );

            let val = if let Some(f) = value.as_float().ok() {
                PropertyValue::Float(f as f32)
            } else if let Some(i) = value.as_int().ok() {
                PropertyValue::Float(i as f32)
            } else if let Some(b) = value.as_bool().ok() {
                PropertyValue::String(if b {
                    "true".to_string()
                } else {
                    "false".to_string()
                })
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
                } else if let Ok(_fsm) = rhai::serde::from_dynamic::<InactiveFSMDefinition>(&value)
                {
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
        },
    );

    engine.register_fn(
        "define_fsm",
        |history: &mut HistoryManager, name: &str, definition: InactiveFSMDefinition| {
            history.push_and_apply(Event::DefineFSM {
                name: name.to_string(),
                definition,
            });
        },
    );

    engine.register_fn(
        "define_material",
        |history: &mut HistoryManager, name: &str, material: Material| {
            history.push_and_apply(Event::DefineMaterial {
                name: name.to_string(),
                material,
            });
        },
    );

    engine.register_fn(
        "move_sprite",
        |history: &mut HistoryManager, id: i64, destination: Hex, transition: TransitionConfig| {
            history.push_and_apply(Event::MoveSprite {
                id: id as u64,
                destination,
                transition: Some(transition),
            });
        },
    );

    engine.register_fn(
        "set_animation_state",
        |history: &mut HistoryManager, id: i64, state: &str| {
            history.push_and_apply(Event::SetAnimationState {
                id: id as u64,
                state: state.to_string(),
            });
        },
    );

    engine.register_fn("sequence_number", |history: &mut HistoryManager, n: i64| {
        history.push_and_apply(Event::SequenceNumber(n as u64));
    });
}

pub fn register_all(engine: &mut Engine) {
    engine.set_max_expr_depths(500, 500);
    // Multifile asset generators may intentionally build medium-sized
    // procedural spritestacks in Rhai; keep the guard finite but above the
    // Arrow/Rock initialization workload.
    engine.set_max_operations(100_000_000);
    engine.set_max_variables(1000);

    register_basic_types(engine);
    register_domain_types(engine);
    register_animation_types(engine);
    assets::register_asset_management(engine);
    register_physics(engine);
    register_history_methods(engine);
    engine.register_type_with_name::<TacticalSimulation>("TacticalSimulation");
    rules::register_script_job_schema(engine);
    simulation::register_simulation(engine);
}

#[rhai::plugin::export_module]
mod loop_behavior_module {
    pub const NONE: LoopBehavior = LoopBehavior::None;
    pub const LOOP: LoopBehavior = LoopBehavior::Loop;
    pub const PING_PONG: LoopBehavior = LoopBehavior::PingPong;
}

#[cfg(test)]
#[path = "scripting_tests.rs"]
mod tests;
