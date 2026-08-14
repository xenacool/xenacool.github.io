use super::RhaiMctsConfig;
use crate::pg_rpg::simulation::{NpcPlanningPolicy, TacticalSimulation};
use pystral_compiler::physics::ProjectileCollider;
use pystral_games::{
    GridCell, GridMap, ScriptAbilityDef, ScriptJobDef, ScriptMovementDef, ScriptPassiveDef,
    ScriptReactionDef, ScriptTagDef, SkirmishConfig,
};
use rhai::{Dynamic, Engine};
pub(super) fn register_simulation(engine: &mut Engine) {
    engine
        .register_type_with_name::<SkirmishConfig>("SkirmishConfig")
        .register_fn(
            "new_skirmish_config",
            |seed: i64| -> Result<SkirmishConfig, Box<rhai::EvalAltResult>> {
                if seed < 0 {
                    return Err(runtime_error("Scenario seed must be non-negative"));
                }
                Ok(SkirmishConfig::new_empty(seed as u64))
            },
        )
        .register_fn("with_builtin_script_registry", |config: SkirmishConfig| {
            config.with_builtin_script_registry()
        })
        .register_fn(
            "add_script_job",
            |config: &mut SkirmishConfig,
             job: ScriptJobDef|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_script_job(job).map_err(runtime_error)
            },
        )
        .register_fn(
            "add_script_ability",
            |config: &mut SkirmishConfig,
             ability: ScriptAbilityDef|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_script_ability(ability).map_err(runtime_error)
            },
        )
        .register_fn(
            "add_script_tag",
            |config: &mut SkirmishConfig,
             tag: ScriptTagDef|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_script_tag(tag).map_err(runtime_error)
            },
        )
        .register_fn(
            "add_script_passive",
            |config: &mut SkirmishConfig,
             passive: ScriptPassiveDef|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_script_passive(passive).map_err(runtime_error)
            },
        )
        .register_fn(
            "add_script_reaction",
            |config: &mut SkirmishConfig,
             reaction: ScriptReactionDef|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_script_reaction(reaction).map_err(runtime_error)
            },
        )
        .register_fn(
            "add_script_movement",
            |config: &mut SkirmishConfig,
             movement: ScriptMovementDef|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_script_movement(movement).map_err(runtime_error)
            },
        )
        .register_fn(
            "set_ct_threshold",
            |config: &mut SkirmishConfig, value: i64| -> Result<(), Box<rhai::EvalAltResult>> {
                config.set_ct_threshold(value).map_err(runtime_error)
            },
        )
        .register_fn("set_grid", |config: &mut SkirmishConfig, grid: GridMap| {
            config.set_grid(grid)
        })
        .register_fn(
            "add_unit",
            |config: &mut SkirmishConfig,
             id: i64,
             team: i64,
             job: &str,
             position: GridCell|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config
                    .add_unit(id, team, job, position)
                    .map_err(runtime_error)
            },
        )
        .register_fn(
            "add_secondary_job",
            |config: &mut SkirmishConfig,
             id: i64,
             job: &str|
             -> Result<(), Box<rhai::EvalAltResult>> {
                config.add_secondary_job(id, job).map_err(runtime_error)
            },
        )
        .register_fn(
            "set_projectile_speed_min",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_speed_min(v as f32),
        )
        .register_fn(
            "set_projectile_speed_max",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_speed_max(v as f32),
        )
        .register_fn(
            "set_projectile_speed_step",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_speed_step(v as f32),
        )
        .register_fn(
            "set_projectile_angle_min",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_angle_min(v as f32),
        )
        .register_fn(
            "set_projectile_angle_max",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_angle_max(v as f32),
        )
        .register_fn(
            "set_projectile_angle_step",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_angle_step(v as f32),
        )
        .register_fn(
            "set_projectile_gravity",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_gravity(v as f32),
        )
        .register_fn(
            "set_projectile_time_step",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_time_step(v as f32),
        )
        .register_fn(
            "set_projectile_max_steps",
            |c: &mut SkirmishConfig, v: i64| c.set_projectile_max_steps(v.max(0) as u32),
        )
        .register_fn(
            "set_projectile_ground_cutoff",
            |c: &mut SkirmishConfig, v: f64| c.set_projectile_ground_cutoff(v as f32),
        )
        .register_fn(
            "set_projectile_collider",
            |c: &mut SkirmishConfig, v: ProjectileCollider| c.set_projectile_collider(v),
        );
    engine.register_fn(
        "set_maximum_turn_count",
        |config: &mut SkirmishConfig, value: i64| -> Result<(), Box<rhai::EvalAltResult>> {
            config.set_maximum_turn_count(value).map_err(runtime_error)
        },
    );
    register_job_history(engine);
    engine
        .register_type_with_name::<RhaiMctsConfig>("MCTSConfiguration")
        .register_fn("new_mcts_config", default_mcts_config)
        .register_fn("set_visits", |c: &mut RhaiMctsConfig, v: i64| c.visits = v)
        .register_fn("set_depth", |c: &mut RhaiMctsConfig, v: i64| c.depth = v)
        .register_fn("set_seed", |c: &mut RhaiMctsConfig, v: i64| {
            c.seed = Some(v)
        })
        .register_fn(
            "set_minimum_hit_probability",
            |c: &mut RhaiMctsConfig, value: f64| c.minimum_hit_probability = value as f32,
        )
        .register_fn(
            "set_allow_desperation",
            |c: &mut RhaiMctsConfig, value: bool| c.allow_desperation = value,
        )
        .register_fn("clear_seed", |c: &mut RhaiMctsConfig| c.seed = None);
    engine.register_fn("new_mcts_config", default_mcts_config);
    engine.register_fn(
        "new_simulation",
        |scenario: SkirmishConfig,
         config: RhaiMctsConfig|
         -> Result<TacticalSimulation, Box<rhai::EvalAltResult>> {
            let policy = NpcPlanningPolicy {
                minimum_hit_probability: config.minimum_hit_probability.clamp(0.0, 1.0),
                allow_desperation: config.allow_desperation,
            };
            config
                .to_native()
                .map(|native| {
                    let mut simulation = TacticalSimulation::from_scenario(scenario, native);
                    simulation.planning_policy = policy;
                    simulation
                })
                .map_err(runtime_error)
        },
    );
    register_simulation_runtime(engine);
}

fn register_simulation_runtime(engine: &mut Engine) {
    engine.register_fn(
        "advance_to_boundary",
        |sim: &mut TacticalSimulation| -> Result<rhai::Array, Box<rhai::EvalAltResult>> {
            sim.advance_to_boundary()
                .map_err(runtime_error)
                .map(|agents| {
                    agents
                        .into_iter()
                        .map(|agent| Dynamic::from(agent.0 as i64))
                        .collect::<rhai::Array>()
                })
        },
    );
    engine.register_fn(
        "advance_to_boundary_budgeted",
        |sim: &mut TacticalSimulation,
         max_ticks: i64|
         -> Result<rhai::Array, Box<rhai::EvalAltResult>> {
            sim.advance_to_boundary_budgeted(max_ticks.max(0) as usize)
                .map_err(runtime_error)
                .map(|agents| {
                    agents
                        .unwrap_or_default()
                        .into_iter()
                        .map(|agent| Dynamic::from(agent.0 as i64))
                        .collect::<rhai::Array>()
                })
        },
    );
    engine.register_fn(
        "get_agent_position",
        |sim: &mut TacticalSimulation, id: i64| sim.get_agent_position(id).hex,
    );
    engine.register_fn(
        "get_agent_health",
        |sim: &mut TacticalSimulation, id: i64| sim.get_agent_health(id) as i64,
    );
    engine.register_fn(
        "set_agent_health",
        |sim: &mut TacticalSimulation, id: i64, health: i64| {
            sim.set_agent_health(id, health).map_err(runtime_error)
        },
    );
    engine.register_fn("get_prompts", |sim: &mut TacticalSimulation, id: i64| {
        sim.get_prompts(id)
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect::<rhai::Map>()
    });
    engine.register_fn("list_agents", |sim: &mut TacticalSimulation| {
        sim.list_agents()
            .into_iter()
            .map(Dynamic::from)
            .collect::<rhai::Array>()
    });
}
fn register_job_history(engine: &mut Engine) {
    engine.register_fn(
        "add_job_history",
        |config: &mut SkirmishConfig,
         id: i64,
         job: &str,
         levels: i64|
         -> Result<(), Box<rhai::EvalAltResult>> {
            config
                .add_job_history(id, job, levels)
                .map_err(runtime_error)
        },
    );
}

fn runtime_error(error: impl Into<String>) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorRuntime(
        error.into().into(),
        rhai::Position::NONE,
    ))
}

fn default_mcts_config() -> RhaiMctsConfig {
    RhaiMctsConfig {
        visits: 50,
        depth: 10,
        seed: Some(42),
        minimum_hit_probability: 0.20,
        allow_desperation: false,
    }
}
