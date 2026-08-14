use pystral_core::history::HistoryManager;
use pystral_core::log::{AvailableActions, AvailableMove, GameOutcome};
use pystral_games::ActionError;
use pystral_runtime::{AbilityTarget, RuntimeRequest, RuntimeResponse, UnitStateInfo};
use serde::{Deserialize, Serialize};

pub mod render;
pub mod simulation_worker;
pub mod worker;

#[cfg(test)]
mod character_test;
#[cfg(test)]
mod strict_log_test;

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
    /// A response to a liveness probe. This is deliberately separate from
    /// gameplay output and the input watermark so an idle worker remains
    /// observable without manufacturing history events.
    Heartbeat(WorkerHeartbeat),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Simulating,
    WaitingForAnimationAck,
    AwaitingPlayerDecision,
    Completed,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerHeartbeat {
    pub latest_output_seq: u64,
    pub latest_input_seq: u64,
    pub status: WorkerStatus,
    #[serde(default)]
    pub pending_barrier: Option<u64>,
    #[serde(default)]
    pub active_request_seq: Option<u64>,
    #[serde(default)]
    pub last_progress_seq: u64,
}

impl WorkerHeartbeat {
    pub fn is_monotonic_after(&self, previous: &Self) -> bool {
        self.latest_output_seq >= previous.latest_output_seq
            && self.latest_input_seq >= previous.latest_input_seq
            && self.last_progress_seq >= previous.last_progress_seq
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerInput {
    LogInfo(String),
    LogError(String),
    ResetLog,
    ActionNav(String),
    RuntimeRequest(RuntimeRequest),
    Ack(u64),
    HeartbeatProbe,
    SimulationBridgeFailure {
        request_seq: Option<u64>,
        reason: String,
    },
    SimulationResponse(
        Box<
            crate::simulation_worker::SimulationEnvelope<
                crate::simulation_worker::SimulationResponse,
            >,
        >,
    ),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MovePreview {
    pub request_id: u64,
    pub unit_id: u64,
    pub source: Option<AvailableMove>,
    pub reachable: Vec<AvailableMove>,
    pub selected_destination: Option<AvailableMove>,
    pub path: Vec<AvailableMove>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TransientState {
    pub active_unit_id: Option<u64>,
    pub available_actions: Option<AvailableActions>,
    pub unit_states: Vec<UnitStateInfo>,
    pub action_feedback: Option<String>,
    pub menu_path: Vec<String>,
    pub preview: Option<MovePreview>,
    pub ability_targets: Option<AbilityTargetMenu>,
    pub action_pending: bool,
    pub wait_pending: bool,
    /// True only after the worker has released simulation/barrier ownership.
    #[serde(default)]
    pub input_enabled: bool,
    pub game_completed: bool,
    #[serde(default)]
    pub completion_outcome: Option<GameOutcome>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AbilityTargetMenu {
    pub request_id: u64,
    pub unit_id: u64,
    pub ability_id: u64,
    pub target_session_id: u64,
    pub state_version: u64,
    pub snapshot_fingerprint: u64,
    pub targets: Vec<AbilityTarget>,
    pub selected_index: usize,
    pub disabled_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerOutput {
    LogUpdate {
        messages: Vec<String>,
        total_errors: u32,
        total_info: u32,
    },
    RuntimeResponse(Box<RuntimeResponse>),
    TransientState(Box<TransientState>),
    DebugTrace {
        message: String,
    },
    SimulationRequest(
        crate::simulation_worker::SimulationEnvelope<pystral_runtime::RuntimeRequest>,
    ),
    ActionRejected {
        request_id: u64,
        reason: ActionError,
    },
}

pub enum AppCommand {
    SetHistoryIndex(u32),
    TogglePlayLog,
    TogglePlayAnimations,
    SetDebugMode(bool),
    SetHistoryStepMs(f64),
    UpdateHistory(Box<HistoryManager>),
    AppendHistory(Box<HistoryManager>),
    CameraNav(String),
    ActionNav(String),
    UpdateTransientState(Box<TransientState>),
    ActionRejected {
        request_id: u64,
        reason: ActionError,
    },
}

#[cfg(any(test, debug_assertions))]
pub fn load_test_assets() -> (String, Vec<u8>, u32) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let web = root.join("web");

    let atlas = std::fs::read_to_string(web.join("atlas.json"))
        .expect("Failed to load atlas.json for test");
    let spritesheet = std::fs::read(web.join("spritesheet.png"))
        .expect("Failed to load spritesheet.png for test");

    let decoder = png::Decoder::new(std::io::Cursor::new(&spritesheet));
    let mut reader = decoder
        .read_info()
        .expect("Failed to read spritesheet info for test");
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .expect("Failed to read spritesheet frame for test");

    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
            for chunk in buf.chunks_exact(3) {
                rgba.push(chunk[0]);
                rgba.push(chunk[1]);
                rgba.push(chunk[2]);
                rgba.push(255);
            }
            rgba
        }
        _ => panic!(
            "Unsupported spritesheet color type in test: {:?}",
            info.color_type
        ),
    };

    (atlas, rgba, info.width)
}

#[cfg(test)]
mod heartbeat_tests {
    use super::{WorkerHeartbeat, WorkerStatus};

    #[test]
    fn heartbeat_status_round_trips_without_losing_reason() {
        let heartbeat = WorkerHeartbeat {
            latest_output_seq: 9,
            latest_input_seq: 12,
            status: WorkerStatus::WaitingForAnimationAck,
            pending_barrier: Some(7),
            active_request_seq: None,
            last_progress_seq: 0,
        };
        let encoded = serde_json::to_string(&heartbeat).unwrap();
        let decoded: WorkerHeartbeat = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, heartbeat);
    }

    #[test]
    fn heartbeat_sequences_reject_regressing_progress() {
        let previous = WorkerHeartbeat {
            latest_output_seq: 9,
            latest_input_seq: 12,
            status: WorkerStatus::Simulating,
            pending_barrier: None,
            active_request_seq: None,
            last_progress_seq: 0,
        };
        let regressed = WorkerHeartbeat {
            latest_output_seq: 8,
            latest_input_seq: 12,
            status: WorkerStatus::Idle,
            pending_barrier: None,
            active_request_seq: None,
            last_progress_seq: 0,
        };
        assert!(!regressed.is_monotonic_after(&previous));
    }

    proptest::proptest! {
        #[test]
        fn heartbeat_sequences_accept_only_monotonic_progress(
            previous_output in 0u64..10_000,
            output_delta in 0u64..10_000,
            previous_input in 0u64..10_000,
            input_delta in 0u64..10_000,
            previous_progress in 0u64..10_000,
            progress_delta in 0u64..10_000,
        ) {
            let previous = WorkerHeartbeat {
                latest_output_seq: previous_output,
                latest_input_seq: previous_input,
                status: WorkerStatus::Idle,
                pending_barrier: None,
                active_request_seq: None,
                last_progress_seq: previous_progress,
            };
            let next = WorkerHeartbeat {
                latest_output_seq: previous_output.saturating_add(output_delta),
                latest_input_seq: previous_input.saturating_add(input_delta),
                status: WorkerStatus::Simulating,
                pending_barrier: None,
                active_request_seq: None,
                last_progress_seq: previous_progress.saturating_add(progress_delta),
            };
            assert!(next.is_monotonic_after(&previous));
        }
    }
}
