mod game_loop;
mod game_loop_helpers;
pub mod pg_rpg;
mod rhai_session;
use pg_rpg::ScenarioBundle;

use hexx::Hex;
use pystral_compiler::ik::{IkRequest, IkResponse, IkSystem};
use pystral_compiler::physics::TrajectoryResponse;
use pystral_core::history::HistoryManager;
use pystral_core::log::{AvailableMove, Event, GameOutcome};
use pystral_games::{ActionError, GridCell};
use rhai_session::RhaiSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeDecisionAction {
    Move {
        hex: Hex,
        layer: i32,
    },
    Wait,
    Reaction {
        reaction_id: u64,
        target: u64,
    },
    Ability {
        ability_id: u64,
        target: RuntimeAbilityTarget,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeAbilityTarget {
    Unit { unit_id: u64 },
    Cell { hex: Hex, layer: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDecision {
    pub unit_id: u64,
    pub action: RuntimeDecisionAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitStateInfo {
    pub unit_id: u64,
    pub state: pystral_games::UnitState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionProvenance {
    pub state_version: u64,
    pub target_session_id: u64,
    pub snapshot_fingerprint: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AbilityTargetKind {
    Unit { unit_id: u64 },
    Cell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AbilityTarget {
    pub kind: AbilityTargetKind,
    pub hex: Hex,
    pub layer: i32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RuntimeContinuation {
    #[default]
    AwaitBoundary,
    AwaitPlayerDecision {
        unit_id: u64,
    },
    AwaitMctsDecision {
        unit_id: u64,
        request_id: u64,
        state_version: u64,
    },
    AwaitAnimationAck {
        barrier_id: u64,
        unit_id: u64,
        ends_turn: bool,
        npc: bool,
    },
    RecoverRejected {
        request_id: u64,
        unit_id: u64,
    },
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeRequest {
    SolveIk(IkRequest),
    GeneratePgRpgLog {
        bundle: ScenarioBundle,
        atlas_json: String,
        spritesheet_rgba: Vec<u8>,
        spritesheet_width: u32,
    },
    StartPgRpgSimulation {
        bundle: ScenarioBundle,
        atlas_json: String,
        spritesheet_rgba: Vec<u8>,
        spritesheet_width: u32,
    },
    StepPgRpgSimulation,
    RequestMctsDecision {
        request_id: u64,
        unit_id: u64,
        state_version: u64,
    },
    MctsDecisionReady {
        request_id: u64,
        decision: RuntimeDecision,
        state_version: u64,
    },
    OpenMovePreview {
        request_id: u64,
        unit_id: u64,
    },
    OpenAbilityTargets {
        request_id: u64,
        unit_id: u64,
        ability_id: u64,
    },
    ActionInput {
        direction: String,
    },
    TestOccupyDestination {
        unit_id: u64,
        hex: Hex,
        layer: i32,
    },
    CommitMove {
        request_id: u64,
        preview_request_id: u64,
        unit_id: u64,
        hex: Hex,
        layer: i32,
    },
    CommitWait {
        request_id: u64,
        unit_id: u64,
    },
    CommitDecision {
        request_id: u64,
        decision: RuntimeDecision,
        provenance: Option<DecisionProvenance>,
    },
    AcknowledgeAnimation {
        barrier_id: u64,
    },
    ResumeBoundary,
    ResumeRejected {
        request_id: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeResponse {
    IkSolved(IkResponse),
    TrajectorySolved(TrajectoryResponse),
    PgRpgLogGenerated(HistoryManager),
    PgRpgSimulationStarted(HistoryManager),
    PgRpgSimulationStepped(HistoryManager),
    SimulationProgress {
        work_units: u32,
    },
    GameCompleted {
        outcome: GameOutcome,
        history: HistoryManager,
    },
    MovePreview {
        request_id: u64,
        unit_id: u64,
        source: AvailableMove,
        reachable: Vec<AvailableMove>,
        selected_destination: Option<AvailableMove>,
    },
    AbilityTargets {
        request_id: u64,
        unit_id: u64,
        ability_id: u64,
        target_session_id: u64,
        state_version: u64,
        snapshot_fingerprint: u64,
        targets: Vec<AbilityTarget>,
        disabled_reason: Option<String>,
    },
    ScriptExecuted(String), // Result as string for now
    ActionInputRouted(String),
    ActionRejected {
        request_id: u64,
        reason: ActionError,
    },
    ActionCommitted {
        request_id: u64,
        unit_id: u64,
        action: String,
        barrier_id: u64,
        history: HistoryManager,
    },
    MctsDecisionReady {
        request_id: u64,
        decision: RuntimeDecision,
        state_version: u64,
    },
    Continuation(RuntimeContinuation),
    Error(String),
}

#[derive(Default)]
pub struct Runtime {
    ik_system: IkSystem,
    pg_rpg_sim: Option<pg_rpg::simulation::TacticalSimulation>,
    pg_rpg_history: Option<HistoryManager>,
    pg_rpg_sequence_number: u64,
    pg_rpg_completion_emitted: bool,
    continuation: RuntimeContinuation,
    rhai_session: Option<RhaiSession>,
    next_npc_request_id: u64,
    next_target_session_id: u64,
    active_target_session: Option<(u64, u64, u64)>,
}

impl Runtime {
    pub fn unit_states(&self) -> Vec<UnitStateInfo> {
        self.pg_rpg_sim
            .as_ref()
            .map(|simulation| {
                simulation
                    .state
                    .agents
                    .iter()
                    .map(|(id, state)| UnitStateInfo {
                        unit_id: id.0 as u64,
                        state: state.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn continuation(&self) -> RuntimeContinuation {
        self.continuation.clone()
    }

    pub fn snapshot_fingerprint(&self) -> Option<u64> {
        self.pg_rpg_sim
            .as_ref()
            .map(pg_rpg::simulation::TacticalSimulation::snapshot_fingerprint)
    }

    pub fn process_request(&mut self, request: RuntimeRequest) -> (RuntimeResponse, Vec<String>) {
        if let Some(rejection) = self.reject_after_completion(&request) {
            return rejection;
        }
        let mut logs = Vec::new();
        let response = match request {
            RuntimeRequest::SolveIk(req) => match self.ik_system.solve(&req) {
                Ok(res) => RuntimeResponse::IkSolved(res),
                Err(e) => {
                    logs.push(format!("IK Error: {}", e));
                    RuntimeResponse::Error(e)
                }
            },
            RuntimeRequest::GeneratePgRpgLog {
                bundle,
                atlas_json,
                spritesheet_rgba,
                spritesheet_width,
            } => {
                let mut history = HistoryManager::new();
                pg_rpg::generate_pg_rpg_log_bundle(
                    &mut history,
                    &bundle,
                    &atlas_json,
                    &spritesheet_rgba,
                    spritesheet_width,
                );
                RuntimeResponse::PgRpgLogGenerated(history)
            }
            RuntimeRequest::StartPgRpgSimulation {
                bundle,
                atlas_json,
                spritesheet_rgba,
                spritesheet_width,
            } => {
                let script = match bundle.root_rhai() {
                    Ok(script) => script,
                    Err(error) => return (RuntimeResponse::Error(error), logs),
                };
                let session = match RhaiSession::new(
                    &script,
                    HistoryManager::new(),
                    atlas_json,
                    spritesheet_rgba,
                    spritesheet_width,
                ) {
                    Ok(session) => session,
                    Err(error) => {
                        return (RuntimeResponse::Error(format!("Rhai Error: {error}")), logs);
                    }
                };
                let mut session = session;
                let history = match session.history() {
                    Ok(history) => history,
                    Err(error) => return (RuntimeResponse::Error(error), logs),
                };
                self.pg_rpg_sim = session.simulation().ok();
                self.pg_rpg_history = Some(history.clone());
                self.rhai_session = Some(session);
                self.pg_rpg_sequence_number = 0;
                self.pg_rpg_completion_emitted = false;
                self.next_npc_request_id = 1;
                self.next_target_session_id = 1;
                self.active_target_session = None;

                RuntimeResponse::PgRpgSimulationStarted(history)
            }
            RuntimeRequest::StepPgRpgSimulation => self.step_pg_rpg_simulation(),
            RuntimeRequest::RequestMctsDecision {
                request_id,
                unit_id,
                state_version,
            } => self.request_mcts_decision(request_id, unit_id, state_version),
            RuntimeRequest::MctsDecisionReady {
                request_id,
                decision,
                state_version,
            } => self.apply_mcts_decision(request_id, decision, state_version),
            RuntimeRequest::CommitDecision {
                request_id,
                decision,
                provenance,
            } => match decision.action {
                RuntimeDecisionAction::Move { hex, layer } => {
                    self.process_request(RuntimeRequest::CommitMove {
                        request_id,
                        preview_request_id: 0,
                        unit_id: decision.unit_id,
                        hex,
                        layer,
                    })
                    .0
                }
                RuntimeDecisionAction::Wait => {
                    self.process_request(RuntimeRequest::CommitWait {
                        request_id,
                        unit_id: decision.unit_id,
                    })
                    .0
                }
                RuntimeDecisionAction::Reaction { .. } => RuntimeResponse::Error(
                    "Reaction decisions are reserved for NPC revalidation".into(),
                ),
                RuntimeDecisionAction::Ability { ability_id, target } => self
                    .commit_ability_request(
                        request_id,
                        decision.unit_id,
                        ability_id,
                        target,
                        provenance,
                    ),
            },
            RuntimeRequest::OpenMovePreview {
                request_id,
                unit_id,
            } => {
                let Some(sim) = self.pg_rpg_sim.as_ref() else {
                    return (
                        RuntimeResponse::ActionRejected {
                            request_id,
                            reason: ActionError::UnknownAgent(npc_engine_core::AgentId(
                                unit_id as u32,
                            )),
                        },
                        logs,
                    );
                };
                match sim.move_preview(unit_id) {
                    Ok((source, reachable)) => RuntimeResponse::MovePreview {
                        request_id,
                        unit_id,
                        source,
                        selected_destination: reachable.first().cloned(),
                        reachable,
                    },
                    Err(reason) => RuntimeResponse::ActionRejected { request_id, reason },
                }
            }
            RuntimeRequest::OpenAbilityTargets {
                request_id,
                unit_id,
                ability_id,
            } => self.open_ability_targets(request_id, unit_id, ability_id),
            RuntimeRequest::ActionInput { direction } => {
                RuntimeResponse::ActionInputRouted(direction)
            }
            RuntimeRequest::TestOccupyDestination {
                unit_id,
                hex,
                layer,
            } => {
                let destination = GridCell::new(hex, layer);
                let Some(sim) = self.pg_rpg_sim.as_mut() else {
                    return (
                        RuntimeResponse::Error("Simulation not started".to_string()),
                        logs,
                    );
                };
                let occupant = sim
                    .state
                    .agents
                    .keys()
                    .copied()
                    .find(|agent| agent.0 as u64 != unit_id);
                if let Some(agent) = occupant {
                    sim.state
                        .agents
                        .get_mut(&agent)
                        .expect("occupant exists")
                        .position = destination;
                    RuntimeResponse::ActionInputRouted("test-occupied-destination".to_string())
                } else {
                    RuntimeResponse::Error(
                        "No other unit is available for test occupancy".to_string(),
                    )
                }
            }
            RuntimeRequest::CommitMove {
                request_id,
                unit_id,
                hex,
                layer,
                ..
            } => self.commit_move_request(request_id, unit_id, GridCell::new(hex, layer)),
            RuntimeRequest::CommitWait {
                request_id,
                unit_id,
            } => self.commit_wait_request(request_id, unit_id),
            RuntimeRequest::AcknowledgeAnimation { barrier_id } => {
                self.acknowledge_animation(barrier_id)
            }
            RuntimeRequest::ResumeBoundary => self.resume_boundary(),
            RuntimeRequest::ResumeRejected { request_id } => self.resume_rejected(request_id),
        };
        self.update_continuation(&response);
        (response, logs)
    }

    fn sync_rhai_session(&mut self) {
        let (Some(session), Some(simulation)) =
            (self.rhai_session.as_mut(), self.pg_rpg_sim.clone())
        else {
            return;
        };
        // History is runtime-owned and append-only. Rhai's boundary function
        // does not need a cloned copy of the complete checkpoint/event log.
        session.set_simulation(simulation);
    }
}

#[cfg(test)]
mod ability_target_tests;
#[cfg(test)]
mod full_playout_tests;
#[cfg(test)]
mod npc_protocol_tests;
#[cfg(test)]
mod tests;
