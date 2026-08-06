pub mod character;
pub mod demo;
pub mod script;

use pystral_compiler::ik::{IkSystem, IkRequest, IkResponse};
use pystral_compiler::physics::{TrajectorySystem, TrajectoryRequest, TrajectoryResponse};
use pystral_core::history::HistoryManager;
use pystral_core::domain::HexMap;
use pystral_core::script::ScriptIR;
use crate::script::vm::{ScriptVM, Value};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeRequest {
    SolveIk(IkRequest),
    SolveTrajectory(TrajectoryRequest, HexMap),
    GenerateDemoLog,
    ExecuteScript(ScriptIR, usize, Vec<Value>),
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
    vm: ScriptVM,
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
            RuntimeRequest::ExecuteScript(ir, fn_idx, args) => {
                match self.vm.execute(&ir, fn_idx, args) {
                    Ok(res) => RuntimeResponse::ScriptExecuted(format!("{:?}", res)),
                    Err(e) => {
                        logs.push(format!("VM Error: {}", e));
                        RuntimeResponse::Error(e)
                    }
                }
            }
        };
        (response, logs)
    }
}
