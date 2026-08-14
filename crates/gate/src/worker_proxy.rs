use super::*;

impl UnifiedWorker {
    pub(crate) fn handle_simulation_bridge_failure(
        &mut self,
        request_seq: Option<u64>,
        reason: String,
    ) {
        let Some((pending_seq, _)) = self.pending_simulation.as_ref() else {
            return;
        };
        if request_seq.is_some_and(|seq| seq != *pending_seq) {
            return;
        }
        let pending_seq = *pending_seq;
        self.pending_simulation = None;
        self.pending_simulation_request = None;
        self.simulation_retry_heartbeats = 0;
        self.simulation_retry_attempts = 0;
        self.is_simulating = false;
        self.transient_state.action_pending = false;
        self.transient_state.wait_pending = false;
        self.transient_state.action_feedback = Some(format!(
            "Simulation worker failed for request {pending_seq}: {reason}"
        ));
        self.push_debug_trace(format!(
            "simulation bridge failed request seq {pending_seq}: {reason}"
        ));
        self.push_output(WorkerOutput::TransientState(Box::new(
            self.transient_state.clone(),
        )));
        self.push_output(WorkerOutput::RuntimeResponse(Box::new(
            RuntimeResponse::Error(format!(
                "Simulation worker failed for request {pending_seq}: {reason}"
            )),
        )));
    }

    pub(crate) fn retry_stalled_simulation(&mut self, cx: &mut Context<'_>) {
        let Some(seq) = self.pending_simulation.as_ref().map(|(seq, _)| *seq) else {
            self.simulation_retry_heartbeats = 0;
            return;
        };
        self.simulation_retry_heartbeats = self.simulation_retry_heartbeats.saturating_add(1);
        if !super::simulation_retry_due(self.simulation_retry_heartbeats) {
            return;
        }

        self.simulation_retry_heartbeats = 0;
        self.simulation_retry_attempts = self.simulation_retry_attempts.saturating_add(1);
        if self.simulation_retry_attempts > super::MAX_SIMULATION_RETRIES {
            self.handle_simulation_bridge_failure(
                Some(seq),
                format!(
                    "no response after {} retries",
                    super::MAX_SIMULATION_RETRIES
                ),
            );
            return;
        }
        // SimulationWorker executes runtime requests synchronously.  A
        // retry here would be queued behind the original request and could
        // cause duplicate work once that request finally returns.  Keep the
        // heartbeat useful as diagnostics and let the bounded failure below
        // be the only recovery action.
        self.push_debug_trace(format!(
            "simulation bridge still waiting for request seq {} after {} heartbeat windows",
            seq, self.simulation_retry_attempts
        ));
        let _ = cx;
    }

    pub(crate) fn enqueue_simulation_request(
        &mut self,
        request: RuntimeRequest,
        pending: PendingSimulation,
    ) {
        let seq = self.next_simulation_request_seq;
        self.next_simulation_request_seq += 1;
        self.pending_simulation_request = Some(request.clone());
        self.pending_simulation = Some((seq, pending));
        self.simulation_retry_heartbeats = 0;
        self.simulation_retry_attempts = 0;
        self.push_output(WorkerOutput::SimulationRequest(SimulationEnvelope {
            seq,
            msg: request,
        }));
    }

    pub(crate) fn handle_simulation_response(
        &mut self,
        envelope: SimulationEnvelope<SimulationResponse>,
        cx: &mut Context<'_>,
    ) {
        let Some((request_seq, pending)) = self.pending_simulation.take() else {
            return;
        };
        if !simulation_response_matches(request_seq, envelope.msg.request_seq) {
            self.pending_simulation = Some((request_seq, pending));
            return;
        }
        let response_request_seq = envelope.msg.request_seq;
        self.simulation_retry_heartbeats = 0;
        self.simulation_retry_attempts = 0;
        self.pending_simulation_request = None;
        let SimulationResponse {
            response,
            logs,
            continuation,
            unit_states,
            snapshot_fingerprint,
            ..
        } = envelope.msg;
        self.push_debug_trace(format!(
            "unified worker received simulation response request seq {} continuation {:?}",
            response_request_seq, continuation
        ));
        self.continuation = continuation;
        self.unit_states = unit_states;
        self.snapshot_fingerprint = snapshot_fingerprint;
        self.last_simulation_progress_seq = response_request_seq;
        self.transient_state.unit_states = self.unit_states.clone();
        if matches!(
            self.continuation,
            RuntimeContinuation::AwaitPlayerDecision { .. }
        ) && !self.transient_state.action_pending
        {
            // Preview/target responses can return directly to the player
            // boundary without passing through the auto-step publisher.
            // Re-enable input at the authoritative continuation as well.
            self.transient_state.input_enabled = true;
        }
        self.emit_runtime_logs(logs, &response);
        if let RuntimeResponse::Error(error) = &response {
            // A failed simulation request must not leave the controller in a
            // self-sustaining Simulating state. `pending_simulation` has just
            // been consumed, so auto_step_if_ready would otherwise retry the
            // same invalid continuation forever while heartbeats continue to
            // look healthy.
            self.is_simulating = false;
            self.transient_state.action_pending = false;
            self.transient_state.wait_pending = false;
            self.transient_state.input_enabled = false;
            self.transient_state.action_feedback = Some(format!("Simulation failed: {error}"));
            self.push_debug_trace(format!(
                "simulation response error request seq {}: {error}",
                response_request_seq
            ));
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            self.push_output(WorkerOutput::RuntimeResponse(Box::new(response)));
            return;
        }
        match pending {
            PendingSimulation::Initial => {
                if let RuntimeResponse::PgRpgSimulationStarted(ref history) = response {
                    self.apply_history_transient(history);
                }
                // A freshly created pg_rpg can already be at the first player
                // boundary.  In that case there is no step response whose
                // normal completion path can clear `is_simulating`; leaving
                // it set strands the UI in Simulating with the action menu
                // hidden forever.
                self.is_simulating = !matches!(
                    self.continuation,
                    RuntimeContinuation::AwaitPlayerDecision { .. }
                );
                self.transient_state.input_enabled = !self.is_simulating;
                self.push_output(WorkerOutput::RuntimeResponse(Box::new(response)));
            }
            PendingSimulation::Action { is_confirm } => {
                let output = self.finish_action_input(response, is_confirm);
                self.push_output(output);
            }
            PendingSimulation::AnimationAck {
                wait,
                refresh_preview,
            } => {
                self.transient_state.action_pending = false;
                self.transient_state.input_enabled = false;
                if wait {
                    self.transient_state.wait_pending = false;
                    self.boundary_resume_pending = true;
                } else if animation_ack_resumes_boundary(wait, &self.continuation) {
                    // A non-wait action can still end the match when its
                    // mutation kills the last living unit. Runtime routes
                    // that actor through AwaitBoundary so terminal
                    // classification occurs after presentation ACK. Resume
                    // that boundary instead of leaving the worker with no
                    // request in flight.
                    self.is_simulating = true;
                    self.boundary_resume_pending = true;
                } else if refresh_preview {
                    let request_id = self.next_action_request_id;
                    self.next_action_request_id += 1;
                    let unit_id = self.current_actions.as_ref().map_or(0, |a| a.unit_id);
                    self.enqueue_simulation_request(
                        RuntimeRequest::OpenMovePreview {
                            request_id,
                            unit_id,
                        },
                        PendingSimulation::PreviewRefresh {
                            invalidate_selection: false,
                        },
                    );
                } else if matches!(
                    self.continuation,
                    RuntimeContinuation::AwaitPlayerDecision { .. }
                ) {
                    // A non-turn-ending player ability returns directly to
                    // the player boundary after its animation barrier. There
                    // is no auto-step that can publish PlayerReady for this
                    // path, so restore ownership here from the authoritative
                    // continuation.
                    self.is_simulating = false;
                    self.transient_state.input_enabled = true;
                    self.push_debug_trace(format!(
                        "unified worker published player transient after animation ack active_unit={:?} actions={}",
                        self.transient_state.active_unit_id,
                        self.transient_state.available_actions.is_some()
                    ));
                }
                self.push_output(WorkerOutput::TransientState(Box::new(
                    self.transient_state.clone(),
                )));
            }
            PendingSimulation::PreviewRefresh {
                invalidate_selection,
            } => {
                self.apply_preview_response(&response, invalidate_selection);
                self.push_output(WorkerOutput::RuntimeResponse(Box::new(response)));
            }
            PendingSimulation::AutoStep { resume_boundary } => {
                self.finish_auto_step(response, resume_boundary, cx)
            }
            PendingSimulation::MctsDecision {
                request_id,
                state_version,
            } => {
                if let RuntimeResponse::MctsDecisionReady { decision, .. } = response {
                    self.enqueue_simulation_request(
                        RuntimeRequest::MctsDecisionReady {
                            request_id,
                            decision,
                            state_version,
                        },
                        PendingSimulation::AutoStep {
                            resume_boundary: false,
                        },
                    );
                } else if let RuntimeResponse::ActionRejected { request_id, .. } = response {
                    // NPC decisions are automatic confirmations. Recover the
                    // runtime boundary immediately instead of leaving the
                    // unified worker in RecoverRejected, where every poll
                    // would defer another automatic step forever.
                    self.enqueue_simulation_request(
                        RuntimeRequest::ResumeRejected { request_id },
                        PendingSimulation::ResumeRejected { preview: None },
                    );
                } else {
                    self.push_output(WorkerOutput::RuntimeResponse(Box::new(response)));
                }
            }
            PendingSimulation::ResumeRejected { preview } => {
                if let Some(preview) = preview {
                    let request_id = self.next_action_request_id;
                    self.next_action_request_id += 1;
                    self.enqueue_simulation_request(
                        RuntimeRequest::OpenMovePreview {
                            request_id,
                            unit_id: preview.unit_id,
                        },
                        PendingSimulation::PreviewRefresh {
                            invalidate_selection: true,
                        },
                    );
                }
            }
        }
    }

    fn apply_history_transient(&mut self, history: &pystral_core::history::HistoryManager) {
        if let Some(transient) = transient_state_from_history(history) {
            self.current_actions = transient.available_actions.clone();
            self.transient_state = transient;
            self.transient_state.unit_states = self.unit_states.clone();
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            self.push_debug_trace(format!(
                "unified worker applied transient actions={} active_unit={:?}",
                self.transient_state.available_actions.is_some(),
                self.transient_state.active_unit_id
            ));
        }
    }
}
