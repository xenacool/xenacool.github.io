pub mod demo;

use pystral_compiler::ik::{IkRequest, IkResponse, IkSystem};
use pystral_compiler::physics::TrajectoryResponse;
use pystral_core::history::HistoryManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeRequest {
    SolveIk(IkRequest),
    GenerateDemoLog {
        atlas_json: String,
        spritesheet_rgba: Vec<u8>,
        spritesheet_width: u32,
    },
    StartDemoSimulation {
        atlas_json: String,
        spritesheet_rgba: Vec<u8>,
        spritesheet_width: u32,
    },
    StepDemoSimulation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeResponse {
    IkSolved(IkResponse),
    TrajectorySolved(TrajectoryResponse),
    DemoLogGenerated(HistoryManager),
    DemoSimulationStarted(HistoryManager),
    DemoSimulationStepped(HistoryManager),
    ScriptExecuted(String), // Result as string for now
    Error(String),
}

#[derive(Default)]
pub struct Runtime {
    ik_system: IkSystem,
    demo_sim: Option<demo::simulation::TacticalSimulation>,
    demo_history: Option<HistoryManager>,
    demo_segno: u64,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_request(&mut self, request: RuntimeRequest) -> (RuntimeResponse, Vec<String>) {
        let mut logs = Vec::new();
        let response = match request {
            RuntimeRequest::SolveIk(req) => match self.ik_system.solve(&req) {
                Ok(res) => RuntimeResponse::IkSolved(res),
                Err(e) => {
                    logs.push(format!("IK Error: {}", e));
                    RuntimeResponse::Error(e)
                }
            },
            RuntimeRequest::GenerateDemoLog {
                atlas_json,
                spritesheet_rgba,
                spritesheet_width,
            } => {
                let mut history = HistoryManager::new();
                demo::generate_demo_log(
                    &mut history,
                    &atlas_json,
                    &spritesheet_rgba,
                    spritesheet_width,
                );
                RuntimeResponse::DemoLogGenerated(history)
            }
            RuntimeRequest::StartDemoSimulation {
                atlas_json,
                spritesheet_rgba,
                spritesheet_width,
            } => {
                let mut history = HistoryManager::new();
                let script = include_str!("../../../assets/scripts/demo.rhai");

                // We run the script but we'll modify it to NOT run the simulation loop if possible,
                // or we just let it run and then we'll continue from where it left off?
                // Actually, let's modify the script to be more modular.

                let mut engine = rhai::Engine::new();
                demo::scripting::register_all(&mut engine);

                let mut scope = rhai::Scope::new();
                scope.push("history", history.clone());
                scope.push("atlas_json", atlas_json.to_string());
                scope.push(
                    "spritesheet_rgba",
                    rhai::Blob::from(spritesheet_rgba.to_vec()),
                );
                scope.push("spritesheet_width", spritesheet_width as i64);
                scope.push("run_simulation", false); // We'll add this check to demo.rhai

                if let Err(e) = engine.run_with_scope(&mut scope, script) {
                    return (RuntimeResponse::Error(format!("Rhai Error: {}", e)), logs);
                }

                if let Some(h) = scope.get_value::<HistoryManager>("history") {
                    history = h;
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(cam) =
                            history.current_state.entities.iter().find(|e| e.id == 101)
                        {
                            web_sys::console::log_1(
                                &format!(
                                    "History from scope: Cam 101 angle is {:?}",
                                    cam.properties.get("angle")
                                )
                                .into(),
                            );
                        }
                    }
                }

                let sim = scope.get_value::<demo::simulation::TacticalSimulation>("sim");

                self.demo_sim = sim;
                self.demo_history = Some(history.clone());
                self.demo_segno = 0;

                RuntimeResponse::DemoSimulationStarted(history)
            }
            RuntimeRequest::StepDemoSimulation => {
                if let (Some(sim), Some(history)) =
                    (self.demo_sim.as_mut(), self.demo_history.as_mut())
                {
                    let ready_agents = sim.step();
                    let prompt_id = 999;

                    let mut update_history = HistoryManager::new();
                    // We only want the NEW events
                    let start_idx = history.log.len();

                    for id in ready_agents {
                        let pos = sim.get_agent_position(id.0 as i64);
                        let hex = hexx::Hex::new(pos.0, pos.1);

                        let layout = hexx::HexLayout {
                            orientation: hexx::HexOrientation::Pointy,
                            scale: glam::Vec2::splat(1.0),
                            origin: glam::Vec2::ZERO,
                        };
                        let _world_pos = layout.hex_to_world_pos(hex);

                        history.push_and_apply(pystral_core::log::Event::MoveSprite {
                            id: id.0 as u64,
                            destination: hex,
                            transition: Some(pystral_core::log::TransitionConfig {
                                duration_ms: 500,
                                delta_time_ms: 16.0,
                                tween: pystral_core::log::TweenKind::SineInOut,
                            }),
                        });

                        let prompts = sim.get_prompts(id.0 as i64);
                        history.push_and_apply(pystral_core::log::Event::UpdateProperty {
                            id: prompt_id as u64,
                            property: "visible".to_string(),
                            value: pystral_core::log::PropertyValue::String("true".to_string()),
                        });

                        for (key, val) in prompts {
                            history.push_and_apply(pystral_core::log::Event::UpdateProperty {
                                id: prompt_id as u64,
                                property: key,
                                value: pystral_core::log::PropertyValue::String(if val {
                                    "true".to_string()
                                } else {
                                    "false".to_string()
                                }),
                            });
                        }
                    }

                    // Add a Segno to wait for UI
                    self.demo_segno += 1;
                    history.push_and_apply(pystral_core::log::Event::Segno(self.demo_segno));

                    update_history.log = history.log[start_idx..].to_vec();
                    RuntimeResponse::DemoSimulationStepped(update_history)
                } else {
                    RuntimeResponse::Error("Simulation not started".to_string())
                }
            }
        };
        (response, logs)
    }
}
