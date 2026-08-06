pub mod character;
pub mod demo;

use pystral_compiler::ik::{IkSystem, IkRequest, IkResponse};
use pystral_compiler::physics::{TrajectorySystem, TrajectoryRequest, TrajectoryResponse};
use pystral_core::history::HistoryManager;
use pystral_core::domain::HexMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeRequest {
    SolveIk(IkRequest),
    SolveTrajectory(TrajectoryRequest, HexMap),
    GenerateDemoLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeResponse {
    IkSolved(IkResponse),
    TrajectorySolved(TrajectoryResponse),
    DemoLogGenerated(HistoryManager),
    ScriptExecuted(String), // Result as string for now
    Error(String),
}

#[derive(Default)]
pub struct Runtime {
    ik_system: IkSystem,
    trajectory_system: TrajectorySystem,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_request(&mut self, request: RuntimeRequest) -> (RuntimeResponse, Vec<String>) {
        let mut logs = Vec::new();
        let response = match request {
            RuntimeRequest::SolveIk(req) => {
                match self.ik_system.solve(&req) {
                    Ok(res) => RuntimeResponse::IkSolved(res),
                    Err(e) => {
                        logs.push(format!("IK Error: {}", e));
                        RuntimeResponse::Error(e)
                    }
                }
            }
            RuntimeRequest::SolveTrajectory(req, map) => {
                match self.trajectory_system.solve(&req, &map) {
                    Ok(res) => RuntimeResponse::TrajectorySolved(res),
                    Err(e) => {
                        logs.push(format!("Trajectory Error: {}", e));
                        RuntimeResponse::Error(e)
                    }
                }
            }
            RuntimeRequest::GenerateDemoLog => {
                let mut history = HistoryManager::new();
                demo::generate_demo_log(&mut history);
                RuntimeResponse::DemoLogGenerated(history)
            }
        };
        (response, logs)
    }
}
