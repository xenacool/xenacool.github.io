use pystral_runtime::{RuntimeRequest, RuntimeResponse};
use pystral_core::history::HistoryManager;
use serde::{Deserialize, Serialize};

pub mod render;
pub mod worker;

#[cfg(test)]
mod strict_log_test;
#[cfg(test)]
mod character_test;

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
    Msg(Box<Envelope<WorkerOutput>>),
    Watermark(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerInput {
    Log(String),
    ResetLog,
    RuntimeRequest(RuntimeRequest),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerOutput {
    LogUpdate { messages: Vec<String>, total_errors: u32 },
    RuntimeResponse(Box<RuntimeResponse>),
}

pub enum AppCommand {
    SetHistoryIndex(u32),
    TogglePlayLog,
    TogglePlayAnimations,
    SetDebugMode(bool),
    UpdateHistory(Box<HistoryManager>),
    CameraNav(String),
}
