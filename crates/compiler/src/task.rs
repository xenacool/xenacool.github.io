use serde::{Serialize, Deserialize};
use crate::ik::{IkRequest, IkResponse};
use crate::physics::{TrajectoryRequest, TrajectoryResponse};
use pystral_core::domain::HexMap;
use pystral_core::history::HistoryManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilerTask {
    SolveIk(IkRequest),
    SolveTrajectory(TrajectoryRequest, HexMap),
    GenerateDemoLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilerResponse {
    IkSolved(IkResponse),
    TrajectorySolved(TrajectoryResponse),
    DemoLogGenerated(HistoryManager),
    Error(String),
}
