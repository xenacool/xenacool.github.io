#[path = "worker_actions.rs"]
mod worker_actions;
#[path = "worker_proxy.rs"]
mod worker_proxy;
#[path = "worker_scheduler.rs"]
mod worker_scheduler;
use crate::simulation_worker::{SimulationEnvelope, SimulationResponse};
use crate::{
    AbilityTargetMenu, Envelope, MovePreview, ReliableInput, ReliableOutput, RuntimeResponse,
    TransientState, WorkerHeartbeat, WorkerInput, WorkerOutput, WorkerStatus,
};
use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{Reactor, ReactorScope};
use pystral_core::log::AvailableMove;
use pystral_runtime::{RuntimeContinuation, RuntimeRequest};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasm_bindgen::prelude::*;
use worker_scheduler::{MAX_INPUTS_PER_POLL, simulation_step_allowed_after_input_drain};

const SIMULATION_RETRY_HEARTBEATS: u8 = 8;
const MAX_SIMULATION_RETRIES: u8 = 12;

pub(crate) fn simulation_retry_due(heartbeat_ticks: u8) -> bool {
    heartbeat_ticks >= SIMULATION_RETRY_HEARTBEATS
}

pub(crate) enum PendingSimulation {
    Initial,
    Action { is_confirm: bool },
    AnimationAck { wait: bool, refresh_preview: bool },
    PreviewRefresh { invalidate_selection: bool },
    AutoStep { resume_boundary: bool },
    MctsDecision { request_id: u64, state_version: u64 },
    ResumeRejected { preview: Option<MovePreview> },
}

pub struct UnifiedWorker {
    scope: ReactorScope<ReliableInput, ReliableOutput>,
    logger: pystral_core::ui_log::Logger,
    next_output_seq: u64,
    next_simulation_request_seq: u64,
    last_received_seq: u64,
    last_acked_sequence_number: u64,
    last_sent_sequence_number: u64,
    is_simulating: bool,
    outbox: VecDeque<ReliableOutput>,
    current_actions: Option<pystral_core::log::AvailableActions>,
    transient_state: TransientState,
    next_action_request_id: u64,
    pending_barrier: Option<(u64, bool)>,
    highest_animation_ack: u64,
    refresh_preview_after_barrier: bool,
    boundary_resume_pending: bool,
    continuation: RuntimeContinuation,
    unit_states: Vec<pystral_runtime::UnitStateInfo>,
    snapshot_fingerprint: Option<u64>,
    pending_simulation: Option<(u64, PendingSimulation)>,
    pending_simulation_request: Option<RuntimeRequest>,
    last_simulation_progress_seq: u64,
    simulation_retry_heartbeats: u8,
    simulation_retry_attempts: u8,
}

fn transient_state_from_history(
    history: &pystral_core::history::HistoryManager,
) -> Option<TransientState> {
    history.log.iter().rev().find_map(|event| {
        if let pystral_core::log::Event::AvailableActions(actions) = event {
            Some(TransientState {
                active_unit_id: Some(actions.unit_id),
                available_actions: Some(actions.clone()),
                unit_states: Vec::new(),
                action_feedback: None,
                menu_path: Vec::new(),
                preview: None,
                ability_targets: None,
                action_pending: false,
                wait_pending: false,
                input_enabled: false,
                game_completed: false,
                completion_outcome: None,
            })
        } else {
            None
        }
    })
}

fn matching_animation_barrier(
    pending_barrier: Option<(u64, bool)>,
    highest_ack: u64,
) -> Option<(u64, bool)> {
    // Renderer ACKs are watermarks and may arrive before barrier installation.
    pending_barrier.filter(|(barrier_id, _)| *barrier_id <= highest_ack)
}

fn status_for(
    game_completed: bool,
    pending_barrier: bool,
    is_simulating: bool,
    has_available_actions: bool,
) -> WorkerStatus {
    if game_completed {
        WorkerStatus::Completed
    } else if pending_barrier {
        WorkerStatus::WaitingForAnimationAck
    } else if is_simulating {
        WorkerStatus::Simulating
    } else if has_available_actions {
        WorkerStatus::AwaitingPlayerDecision
    } else {
        WorkerStatus::Idle
    }
}

impl Reactor for UnifiedWorker {
    type Scope = ReactorScope<ReliableInput, ReliableOutput>;

    fn create(scope: Self::Scope) -> Self {
        Self {
            scope,
            logger: pystral_core::ui_log::Logger::new(),
            next_output_seq: 0,
            next_simulation_request_seq: 1,
            last_received_seq: 0,
            last_acked_sequence_number: 0,
            last_sent_sequence_number: 0,
            is_simulating: false,
            outbox: VecDeque::new(),
            current_actions: None,
            transient_state: TransientState::default(),
            next_action_request_id: 1,
            pending_barrier: None,
            highest_animation_ack: 0,
            refresh_preview_after_barrier: false,
            boundary_resume_pending: false,
            continuation: RuntimeContinuation::default(),
            unit_states: Vec::new(),
            snapshot_fingerprint: None,
            pending_simulation: None,
            pending_simulation_request: None,
            last_simulation_progress_seq: 0,
            simulation_retry_heartbeats: 0,
            simulation_retry_attempts: 0,
        }
    }
}

impl Future for UnifiedWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // web_sys::console::log_1(&"Worker polling...".into());

        // Drain outbox
        while self.outbox.front().is_some() {
            match self.scope.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let msg = self.outbox.pop_front().expect("Outbox should not be empty");
                    let _ = self.scope.start_send_unpin(msg);
                }
                Poll::Ready(Err(_)) => {
                    self.outbox.pop_front();
                }
                Poll::Pending => {
                    let _ = self.scope.poll_flush_unpin(cx);
                    return Poll::Pending;
                }
            }
        }

        let mut received_any = false;
        let mut processed_inputs = 0;
        while processed_inputs < MAX_INPUTS_PER_POLL {
            let Poll::Ready(Some(input)) = self.scope.poll_next_unpin(cx) else {
                break;
            };
            processed_inputs += 1;
            received_any = true;
            match input {
                ReliableInput::Msg(envelope) => {
                    self.last_received_seq = envelope.seq;

                    let response = match envelope.msg {
                        WorkerInput::LogInfo(msg) => {
                            self.logger
                                .apply_command(pystral_core::ui_log::LogCommand::Info(msg));
                            Some(WorkerOutput::LogUpdate {
                                messages: self.logger.get_messages(),
                                total_errors: self.logger.total_errors as u32,
                                total_info: self.logger.total_info as u32,
                            })
                        }
                        WorkerInput::LogError(msg) => {
                            self.logger
                                .apply_command(pystral_core::ui_log::LogCommand::Error(msg));
                            Some(WorkerOutput::LogUpdate {
                                messages: self.logger.get_messages(),
                                total_errors: self.logger.total_errors as u32,
                                total_info: self.logger.total_info as u32,
                            })
                        }
                        WorkerInput::ResetLog => {
                            self.logger
                                .apply_command(pystral_core::ui_log::LogCommand::Reset);
                            Some(WorkerOutput::LogUpdate {
                                messages: self.logger.get_messages(),
                                total_errors: self.logger.total_errors as u32,
                                total_info: self.logger.total_info as u32,
                            })
                        }
                        WorkerInput::ActionNav(direction) => self.route_action_input(direction),
                        WorkerInput::RuntimeRequest(task) => {
                            let task = match task {
                                RuntimeRequest::GeneratePgRpgLog {
                                    bundle,
                                    atlas_json,
                                    spritesheet_rgba,
                                    spritesheet_width,
                                } => {
                                    self.is_simulating = true;
                                    self.last_sent_sequence_number = 0;
                                    self.last_acked_sequence_number = 0;
                                    self.pending_barrier = None;
                                    self.highest_animation_ack = 0;
                                    self.boundary_resume_pending = false;
                                    RuntimeRequest::StartPgRpgSimulation {
                                        bundle,
                                        atlas_json,
                                        spritesheet_rgba,
                                        spritesheet_width,
                                    }
                                }
                                task => task,
                            };
                            self.enqueue_simulation_request(task, PendingSimulation::Initial);
                            None
                        }
                        WorkerInput::Ack(sequence_number) => {
                            self.handle_animation_ack(sequence_number, cx);
                            None
                        }
                        WorkerInput::HeartbeatProbe => {
                            self.retry_stalled_simulation(cx);
                            None
                        }
                        WorkerInput::SimulationBridgeFailure {
                            request_seq,
                            reason,
                        } => {
                            self.handle_simulation_bridge_failure(request_seq, reason);
                            None
                        }
                        WorkerInput::SimulationResponse(response) => {
                            self.handle_simulation_response(*response, cx);
                            None
                        }
                    };

                    if let Some(msg) = response {
                        let out_envelope = Envelope {
                            seq: self.next_output_seq,
                            msg,
                        };
                        self.next_output_seq += 1;
                        self.outbox
                            .push_back(ReliableOutput::Msg(Box::new(out_envelope)));
                    }

                    let ack_seq = self.last_received_seq;
                    self.outbox.push_back(ReliableOutput::Watermark(ack_seq));
                    self.push_heartbeat();
                }
                ReliableInput::Watermark(_seq) => {
                    // Handle ACK
                }
            }
        }

        if received_any {
            self.flush_outbox_after_input(cx);
        }

        if simulation_step_allowed_after_input_drain(processed_inputs) {
            self.auto_step_if_ready(cx);
        }

        let _ = self.scope.poll_flush_unpin(cx);
        Poll::Pending
    }
}

impl UnifiedWorker {
    fn push_heartbeat(&mut self) {
        // Watermark acknowledges main->worker delivery. This separate probe
        // response reports output progress and the state that explains why
        // progress may be temporarily absent.
        self.outbox
            .push_back(ReliableOutput::Heartbeat(WorkerHeartbeat {
                latest_output_seq: self.next_output_seq.saturating_sub(1),
                latest_input_seq: self.last_received_seq,
                status: self.status(),
                pending_barrier: self.pending_barrier.map(|(barrier_id, _)| barrier_id),
                active_request_seq: self.pending_simulation.as_ref().map(|(seq, _)| *seq),
                last_progress_seq: self.last_simulation_progress_seq,
            }));
    }

    fn status(&self) -> WorkerStatus {
        let waiting_for_simulation_ack = self
            .pending_simulation
            .as_ref()
            .is_some_and(|(_, pending)| matches!(pending, PendingSimulation::AnimationAck { .. }));
        status_for(
            self.transient_state.game_completed,
            self.pending_barrier.is_some() || waiting_for_simulation_ack,
            self.is_simulating,
            self.transient_state.available_actions.is_some(),
        )
    }

    fn flush_outbox_after_input(&mut self, cx: &mut Context<'_>) {
        while self.outbox.front().is_some() {
            match self.scope.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let msg = self.outbox.pop_front().expect("outbox should not be empty");
                    let _ = self.scope.start_send_unpin(msg);
                }
                Poll::Ready(Err(_)) => {
                    self.outbox.pop_front();
                }
                Poll::Pending => break,
            }
        }
    }

    fn emit_runtime_logs(&mut self, logs: Vec<String>, response: &RuntimeResponse) {
        let has_error = matches!(response, RuntimeResponse::Error(_));
        if logs.is_empty() && !has_error {
            return;
        }
        for log in logs {
            self.logger
                .apply_command(pystral_core::ui_log::LogCommand::Error(log));
        }
        if let RuntimeResponse::Error(error) = response {
            self.logger
                .apply_command(pystral_core::ui_log::LogCommand::Error(format!(
                    "Runtime Error: {error}"
                )));
        }
        self.push_output(WorkerOutput::LogUpdate {
            messages: self.logger.get_messages(),
            total_errors: self.logger.total_errors as u32,
            total_info: self.logger.total_info as u32,
        });
    }

    fn auto_step_if_ready(&mut self, cx: &mut Context<'_>) {
        self.complete_pending_animation_ack(cx);
        if !self.is_simulating
            || self.pending_barrier.is_some()
            || self.pending_simulation.is_some()
            || self.last_acked_sequence_number < self.last_sent_sequence_number
        {
            return;
        }
        let pending = match self.continuation.clone() {
            RuntimeContinuation::AwaitBoundary => {
                if self.boundary_resume_pending {
                    self.boundary_resume_pending = false;
                    (
                        RuntimeRequest::ResumeBoundary,
                        PendingSimulation::AutoStep {
                            resume_boundary: true,
                        },
                    )
                } else {
                    (
                        RuntimeRequest::StepPgRpgSimulation,
                        PendingSimulation::AutoStep {
                            resume_boundary: false,
                        },
                    )
                }
            }
            RuntimeContinuation::AwaitMctsDecision {
                request_id,
                unit_id,
                state_version,
            } => (
                RuntimeRequest::RequestMctsDecision {
                    request_id,
                    unit_id,
                    state_version,
                },
                PendingSimulation::MctsDecision {
                    request_id,
                    state_version,
                },
            ),
            continuation => {
                self.logger
                    .apply_command(pystral_core::ui_log::LogCommand::Error(format!(
                        "Automatic simulation step deferred in {continuation:?}"
                    )));
                return;
            }
        };
        self.enqueue_simulation_request(pending.0, pending.1);
    }

    fn finish_auto_step(
        &mut self,
        res: RuntimeResponse,
        resume_boundary: bool,
        cx: &mut Context<'_>,
    ) {
        if let RuntimeResponse::SimulationProgress { work_units } = &res {
            self.push_debug_trace(format!(
                "simulation progress boundary={} work_units={work_units}",
                if resume_boundary { "resume" } else { "step" }
            ));
            self.push_output(WorkerOutput::RuntimeResponse(Box::new(
                RuntimeResponse::SimulationProgress {
                    work_units: *work_units,
                },
            )));
            self.enqueue_simulation_request(
                if resume_boundary {
                    RuntimeRequest::ResumeBoundary
                } else {
                    RuntimeRequest::StepPgRpgSimulation
                },
                PendingSimulation::AutoStep { resume_boundary },
            );
            cx.waker().wake_by_ref();
            return;
        }
        if let RuntimeResponse::PgRpgSimulationStepped(ref history)
        | RuntimeResponse::GameCompleted { ref history, .. } = res
        {
            if let Some(transient) = transient_state_from_history(history) {
                if matches!(
                    self.continuation,
                    RuntimeContinuation::AwaitPlayerDecision { .. }
                ) {
                    self.current_actions = transient.available_actions.clone();
                    self.transient_state = transient;
                    self.transient_state.unit_states = self.unit_states.clone();
                    self.is_simulating = false;
                    self.transient_state.input_enabled = true;
                    self.push_debug_trace(format!(
                        "unified worker published player transient active_unit={:?} actions={}",
                        self.transient_state.active_unit_id,
                        self.transient_state.available_actions.is_some()
                    ));
                    self.push_output(WorkerOutput::TransientState(Box::new(
                        self.transient_state.clone(),
                    )));
                }
            } else {
                // A casualty or terminal-adjacent boundary may contain state
                // updates without publishing AvailableActions. The previous
                // menu must not remain actionable while the runtime owns a
                // non-player boundary or is reclassifying the match.
                self.current_actions = None;
                self.transient_state.active_unit_id = None;
                self.transient_state.available_actions = None;
                self.transient_state.menu_path.clear();
                self.transient_state.preview = None;
                self.transient_state.ability_targets = None;
                self.transient_state.action_pending = false;
                self.transient_state.wait_pending = false;
                self.transient_state.input_enabled = false;
                self.push_debug_trace("unified worker cleared stale player transient at boundary");
                self.push_output(WorkerOutput::TransientState(Box::new(
                    self.transient_state.clone(),
                )));
            }
        }
        if let RuntimeResponse::GameCompleted { outcome, .. } = &res {
            self.is_simulating = false;
            self.current_actions = None;
            self.transient_state.available_actions = None;
            self.transient_state.preview = None;
            self.transient_state.ability_targets = None;
            self.transient_state.action_pending = false;
            self.transient_state.wait_pending = false;
            self.transient_state.game_completed = true;
            self.transient_state.completion_outcome = Some(outcome.clone());
            self.transient_state.action_feedback = Some("Game completed".to_string());
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
        }
        self.track_committed_action(&res);
        self.push_output(WorkerOutput::RuntimeResponse(Box::new(res)));
        cx.waker().wake_by_ref();
    }

    fn sync_unit_states(&mut self) {
        self.transient_state.unit_states = self.unit_states.clone();
    }

    fn push_output(&mut self, msg: WorkerOutput) {
        let envelope = Envelope {
            seq: self.next_output_seq,
            msg,
        };
        self.next_output_seq += 1;
        self.outbox
            .push_back(ReliableOutput::Msg(Box::new(envelope)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prop_assert_eq;
    use pystral_core::log::{AvailableActions, AvailableJobActions, Event};

    #[test]
    fn simulation_retry_waits_for_a_bounded_heartbeat_window() {
        assert!(!simulation_retry_due(0));
        assert!(!simulation_retry_due(7));
        assert!(simulation_retry_due(8));
        assert!(simulation_retry_due(u8::MAX));
    }

    #[test]
    fn availability_history_becomes_transient_state() {
        let mut history = pystral_core::history::HistoryManager::new();
        history.push_and_apply(Event::AvailableActions(AvailableActions {
            unit_id: 7,
            movement: Vec::new(),
            primary_job: AvailableJobActions {
                name: "Caveman".into(),
                abilities: Vec::new(),
            },
            secondary_jobs: Vec::new(),
        }));

        let transient = transient_state_from_history(&history).unwrap();
        assert_eq!(transient.active_unit_id, Some(7));
        assert_eq!(
            transient.available_actions.unwrap().primary_job.name,
            "Caveman"
        );
        assert!(transient.menu_path.is_empty());
        assert!(transient.preview.is_none());
    }

    #[test]
    fn unrelated_history_ack_does_not_match_animation_barrier() {
        assert_eq!(matching_animation_barrier(Some((12, true)), 11), None);
        assert_eq!(
            matching_animation_barrier(Some((12, true)), 12),
            Some((12, true))
        );
        assert_eq!(
            matching_animation_barrier(Some((12, true)), 13),
            Some((12, true))
        );
        assert_eq!(matching_animation_barrier(None, 12), None);
    }

    #[test]
    fn animation_ack_watermark_survives_barrier_publish_race() {
        // The renderer can observe and acknowledge a history barrier before
        // the worker has processed the input that installs that barrier.
        // The watermark must be consumable once the barrier is installed.
        let highest_ack = 13;
        assert_eq!(
            matching_animation_barrier(Some((13, true)), highest_ack),
            Some((13, true))
        );
        assert_eq!(
            matching_animation_barrier(Some((14, true)), highest_ack),
            None
        );
    }

    #[test]
    fn status_explains_idle_and_blocking_states_with_stable_precedence() {
        assert_eq!(status_for(false, false, false, false), WorkerStatus::Idle);
        assert_eq!(
            status_for(false, false, false, true),
            WorkerStatus::AwaitingPlayerDecision
        );
        assert_eq!(
            status_for(false, false, true, false),
            WorkerStatus::Simulating
        );
        assert_eq!(
            status_for(false, true, true, true),
            WorkerStatus::WaitingForAnimationAck
        );
        assert_eq!(status_for(true, true, true, true), WorkerStatus::Completed);
    }

    proptest::proptest! {
        #[test]
    fn status_is_never_reported_as_idle_while_work_is_owned(
            completed: bool,
            waiting_for_ack: bool,
            simulating: bool,
            available_actions: bool,
        ) {
            let status = status_for(
                completed,
                waiting_for_ack,
                simulating,
                available_actions,
            );
            if completed {
                prop_assert_eq!(status, WorkerStatus::Completed);
            } else if waiting_for_ack {
                prop_assert_eq!(status, WorkerStatus::WaitingForAnimationAck);
            } else if simulating {
                prop_assert_eq!(status, WorkerStatus::Simulating);
            } else if available_actions {
                prop_assert_eq!(status, WorkerStatus::AwaitingPlayerDecision);
            } else {
                prop_assert_eq!(status, WorkerStatus::Idle);
            }
        }
    }
}

#[wasm_bindgen]
pub fn init_worker() {
    gloo_worker::reactor::ReactorRegistrar::<UnifiedWorker>::new().register();
}

pub use gloo_worker::reactor::ReactorBridge;
