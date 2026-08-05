use pystral_compiler::task::{CompilerTask, CompilerResponse};
use pystral_core::history::HistoryManager;
use serde::{Deserialize, Serialize};

pub mod render;
pub mod worker;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope<T> {
    pub seq: u64,
    pub msg: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReliableInput {
    Msg(Envelope<WorkerInput>),
    Watermark(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReliableOutput {
    Msg(Envelope<WorkerOutput>),
    Watermark(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerInput {
    Log(String),
    ResetLog,
    CompilerTask(CompilerTask),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerOutput {
    LogUpdate { messages: Vec<String>, total_errors: u32 },
    CompilerResponse(CompilerResponse),
}

pub enum AppCommand {
    SetHistoryIndex(u32),
    TogglePlayLog,
    TogglePlayAnimations,
    SetDebugMode(bool),
    UpdateHistory(HistoryManager),
    CameraNav(String),
}
