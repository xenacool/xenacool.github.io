use super::*;
impl UnifiedWorker {
    pub(crate) fn handle_animation_ack(&mut self, sequence_number: u64, cx: &mut Context<'_>) {
        self.highest_animation_ack = self.highest_animation_ack.max(sequence_number);
        self.complete_pending_animation_ack(cx);
    }

    pub(crate) fn complete_pending_animation_ack(&mut self, cx: &mut Context<'_>) {
        let Some((barrier_id, wait)) =
            matching_animation_barrier(self.pending_barrier, self.highest_animation_ack)
        else {
            return;
        };

        self.last_acked_sequence_number = barrier_id;
        self.pending_barrier = None;
        self.enqueue_simulation_request(
            RuntimeRequest::AcknowledgeAnimation { barrier_id },
            PendingSimulation::AnimationAck {
                wait,
                refresh_preview: self.refresh_preview_after_barrier,
            },
        );
        self.refresh_preview_after_barrier = false;
        let _ = cx;
    }

    pub(crate) fn push_debug_trace(&mut self, message: impl Into<String>) {
        let out_envelope = Envelope {
            seq: self.next_output_seq,
            msg: WorkerOutput::DebugTrace {
                message: message.into(),
            },
        };
        self.next_output_seq += 1;
        self.outbox
            .push_back(ReliableOutput::Msg(Box::new(out_envelope)));
    }

    fn trace_ability_menu(&mut self, phase: &str, menu: &AbilityTargetMenu) {
        let targets = menu
            .targets
            .iter()
            .map(|target| target.label.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        self.push_debug_trace(format!(
            "ability-trace phase={phase} request={} unit={} ability={} selected_index={} targets=[{}] session={} version={} fingerprint={}",
            menu.request_id,
            menu.unit_id,
            menu.ability_id,
            menu.selected_index,
            targets,
            menu.target_session_id,
            menu.state_version,
            menu.snapshot_fingerprint,
        ));
    }

    pub(crate) fn finish_action_input(
        &mut self,
        res: RuntimeResponse,
        is_confirm: bool,
    ) -> WorkerOutput {
        self.sync_unit_states();
        self.track_committed_action(&res);
        self.apply_preview_response(&res, false);
        self.apply_ability_targets_response(&res);
        if let RuntimeResponse::AbilityTargets { .. } = &res {
            if let Some(menu) = self.transient_state.ability_targets.clone() {
                self.trace_ability_menu("menu-installed", &menu);
            }
        }
        if let RuntimeResponse::ActionRejected { request_id, reason } = &res {
            self.push_debug_trace(format!(
                "ability-trace phase=commit-response request={} reason={reason:?} runtime_fingerprint={:?}",
                request_id,
                self.snapshot_fingerprint,
            ));
        }
        if let RuntimeResponse::ActionCommitted { action, .. } = &res {
            self.push_debug_trace(format!(
                "ability-trace phase=commit-response action={action} runtime_fingerprint={:?}",
                self.snapshot_fingerprint,
            ));
        }
        if matches!(&res, RuntimeResponse::ActionRejected { .. }) && is_confirm {
            let preview = self.transient_state.preview.clone();
            self.enqueue_simulation_request(
                RuntimeRequest::ResumeRejected {
                    request_id: match &res {
                        RuntimeResponse::ActionRejected { request_id, .. } => *request_id,
                        _ => 0,
                    },
                },
                PendingSimulation::ResumeRejected { preview },
            );
        }
        self.queue_action_rejection(&res);
        WorkerOutput::RuntimeResponse(Box::new(res))
    }

    pub(crate) fn queue_action_rejection(&mut self, response: &RuntimeResponse) {
        if let RuntimeResponse::ActionRejected { request_id, reason } = response {
            let out_envelope = Envelope {
                seq: self.next_output_seq,
                msg: WorkerOutput::ActionRejected {
                    request_id: *request_id,
                    reason: reason.clone(),
                },
            };
            self.next_output_seq += 1;
            self.outbox
                .push_back(ReliableOutput::Msg(Box::new(out_envelope)));
        }
    }

    pub(crate) fn route_action_input(&mut self, direction: String) -> Option<WorkerOutput> {
        if !self.transient_state.input_enabled
            || self.pending_simulation.is_some()
            || self.pending_barrier.is_some()
        {
            return None;
        }
        self.push_debug_trace(format!("unified worker accepted action input {direction}"));
        let is_confirm = direction == "confirm";
        self.logger
            .apply_command(pystral_core::ui_log::LogCommand::Info(format!(
                "Action input: {}",
                direction
            )));
        let log_msg = WorkerOutput::LogUpdate {
            messages: self.logger.get_messages(),
            total_errors: self.logger.total_errors as u32,
            total_info: self.logger.total_info as u32,
        };
        let out_envelope = Envelope {
            seq: self.next_output_seq,
            msg: log_msg,
        };
        self.next_output_seq += 1;
        self.outbox
            .push_back(ReliableOutput::Msg(Box::new(out_envelope)));
        let res = if direction == "test-occupy" {
            let Some(preview) = self.transient_state.preview.clone() else {
                return Some(WorkerOutput::RuntimeResponse(Box::new(
                    RuntimeResponse::Error("No active move preview".to_string()),
                )));
            };
            let Some(destination) = preview.selected_destination else {
                return Some(WorkerOutput::RuntimeResponse(Box::new(
                    RuntimeResponse::Error("No selected destination".to_string()),
                )));
            };
            self.enqueue_simulation_request(
                RuntimeRequest::TestOccupyDestination {
                    unit_id: preview.unit_id,
                    hex: destination.hex,
                    layer: destination.layer,
                },
                PendingSimulation::Action { is_confirm },
            );
            return None;
        } else if direction == "wait" {
            if let Some(actions) = self.current_actions.clone() {
                let request_id = self.next_action_request_id;
                self.next_action_request_id += 1;
                self.is_simulating = true;
                self.last_sent_sequence_number = self.last_acked_sequence_number;
                self.enqueue_simulation_request(
                    RuntimeRequest::CommitWait {
                        request_id,
                        unit_id: actions.unit_id,
                    },
                    PendingSimulation::Action { is_confirm },
                );
                return None;
            } else {
                RuntimeResponse::Error("No player action menu is available".to_string())
            }
        } else if direction == "confirm" && self.transient_state.ability_targets.is_some() {
            let Some(menu) = self.transient_state.ability_targets.clone() else {
                unreachable!();
            };
            let Some(target) = menu.targets.get(menu.selected_index) else {
                return Some(WorkerOutput::RuntimeResponse(Box::new(
                    RuntimeResponse::Error(
                        "Ability target menu has no selected target".to_string(),
                    ),
                )));
            };
            let target = match &target.kind {
                pystral_runtime::AbilityTargetKind::Unit { unit_id } => {
                    pystral_runtime::RuntimeAbilityTarget::Unit { unit_id: *unit_id }
                }
                pystral_runtime::AbilityTargetKind::Cell => {
                    pystral_runtime::RuntimeAbilityTarget::Cell {
                        hex: target.hex,
                        layer: target.layer,
                    }
                }
            };
            self.trace_ability_menu("commit-input", &menu);
            self.push_debug_trace(format!(
                "ability-trace phase=commit-input-target target={target:?} runtime_fingerprint={:?}",
                self.snapshot_fingerprint,
            ));
            let request_id = self.next_action_request_id;
            self.next_action_request_id += 1;
            self.enqueue_simulation_request(
                RuntimeRequest::CommitDecision {
                    request_id,
                    decision: pystral_runtime::RuntimeDecision {
                        unit_id: menu.unit_id,
                        action: pystral_runtime::RuntimeDecisionAction::Ability {
                            ability_id: menu.ability_id,
                            target,
                        },
                    },
                    provenance: Some(pystral_runtime::DecisionProvenance {
                        state_version: menu.state_version,
                        snapshot_fingerprint: menu.snapshot_fingerprint,
                        target_session_id: menu.target_session_id,
                    }),
                },
                PendingSimulation::Action { is_confirm },
            );
            return None;
        } else if direction == "confirm" {
            if let Some(preview) = self.transient_state.preview.clone() {
                if let Some(destination) = preview.selected_destination {
                    let preview_request_id = preview.request_id;
                    let request_id = self.next_action_request_id;
                    self.next_action_request_id += 1;
                    self.enqueue_simulation_request(
                        RuntimeRequest::CommitMove {
                            request_id,
                            preview_request_id,
                            unit_id: preview.unit_id,
                            hex: destination.hex,
                            layer: destination.layer,
                        },
                        PendingSimulation::Action { is_confirm },
                    );
                    return None;
                } else {
                    RuntimeResponse::Error("Move preview has no selected destination".to_string())
                }
            } else if let Some(actions) = self.current_actions.clone() {
                let request_id = self.next_action_request_id;
                self.next_action_request_id += 1;
                self.enqueue_simulation_request(
                    RuntimeRequest::OpenMovePreview {
                        request_id,
                        unit_id: actions.unit_id,
                    },
                    PendingSimulation::Action { is_confirm },
                );
                return None;
            } else {
                RuntimeResponse::Error("No player action menu is available".to_string())
            }
        } else if let Some(job) = direction.strip_prefix("menu-job:") {
            self.select_menu_job(job);
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            RuntimeResponse::ActionInputRouted("job-selected".to_string())
        } else if let Some(ability) = direction.strip_prefix("menu-ability:") {
            return self.open_menu_ability(ability);
        } else if let Some(target) = direction.strip_prefix("menu-target:") {
            self.select_menu_target(target);
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            RuntimeResponse::ActionInputRouted("target-selected".to_string())
        } else if direction == "return" {
            if self.transient_state.ability_targets.is_some() {
                self.transient_state.ability_targets = None;
                self.push_output(WorkerOutput::TransientState(Box::new(
                    self.transient_state.clone(),
                )));
                return Some(WorkerOutput::RuntimeResponse(Box::new(
                    RuntimeResponse::ActionInputRouted("return-to-job-menu".to_string()),
                )));
            }
            self.transient_state.preview = None;
            self.transient_state.menu_path.clear();
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            RuntimeResponse::ActionInputRouted("return-to-top-level-menu".to_string())
        } else if self.transient_state.ability_targets.is_some() {
            self.select_ability_target(&direction);
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            RuntimeResponse::ActionInputRouted("target-navigation".to_string())
        } else if self.transient_state.preview.is_some() {
            self.select_preview_destination(&direction);
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
            self.enqueue_simulation_request(
                RuntimeRequest::ActionInput { direction },
                PendingSimulation::Action { is_confirm },
            );
            return None;
        } else {
            self.enqueue_simulation_request(
                RuntimeRequest::ActionInput { direction },
                PendingSimulation::Action { is_confirm },
            );
            return None;
        };
        Some(self.finish_action_input(res, is_confirm))
    }

    pub(crate) fn track_committed_action(&mut self, response: &RuntimeResponse) {
        if let RuntimeResponse::ActionCommitted {
            barrier_id, action, ..
        } = response
        {
            let wait = action == "wait";
            self.pending_barrier = Some((*barrier_id, wait));
            // NPC moves are followed by the automatic boundary handoff, not
            // a player move preview. Refreshing here during an NPC barrier
            // sends OpenMovePreview while runtime awaits MCTS and forces the
            // continuation into RecoverRejected.
            self.refresh_preview_after_barrier = action == "move" && !self.is_simulating;
            self.last_sent_sequence_number = *barrier_id;
            self.transient_state.preview = None;
            self.transient_state.ability_targets = None;
            if action == "move" || wait {
                self.transient_state.menu_path.clear();
            }
            self.transient_state.action_pending = true;
            self.transient_state.wait_pending = wait;
            self.transient_state.input_enabled = false;
            let display_action = action.get(..1).map_or_else(
                || action.clone(),
                |first| first.to_ascii_uppercase() + &action[1..],
            );
            self.transient_state.action_feedback = Some(format!("{display_action} committed"));
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
        }
    }

    pub(crate) fn apply_preview_response(
        &mut self,
        response: &RuntimeResponse,
        invalidate_selection: bool,
    ) {
        if let RuntimeResponse::MovePreview {
            request_id,
            unit_id,
            source,
            reachable,
            selected_destination,
        } = response
        {
            self.transient_state.preview = Some(MovePreview {
                request_id: *request_id,
                unit_id: *unit_id,
                source: Some(source.clone()),
                reachable: reachable.clone(),
                selected_destination: if invalidate_selection {
                    None
                } else {
                    selected_destination.clone()
                },
                path: if invalidate_selection {
                    Vec::new()
                } else {
                    preview_path(source, selected_destination.as_ref())
                },
            });
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
        }
    }

    fn select_menu_job(&mut self, job: &str) {
        if self.current_actions.is_none() {
            return;
        }
        let valid = job == "primary"
            || job
                .strip_prefix("secondary:")
                .and_then(|index| index.parse::<usize>().ok())
                .is_some_and(|index| {
                    self.current_actions
                        .as_ref()
                        .is_some_and(|actions| index < actions.secondary_jobs.len())
                });
        if valid {
            self.transient_state.menu_path = vec![format!("job:{job}")];
            self.transient_state.ability_targets = None;
        }
    }

    fn open_menu_ability(&mut self, ability: &str) -> Option<WorkerOutput> {
        let Ok(ability_id) = ability.parse::<u64>() else {
            return Some(WorkerOutput::RuntimeResponse(Box::new(
                RuntimeResponse::Error("Invalid ability descriptor".to_string()),
            )));
        };
        let Some(actions) = self.current_actions.as_ref() else {
            return Some(WorkerOutput::RuntimeResponse(Box::new(
                RuntimeResponse::Error("No player action menu is available".to_string()),
            )));
        };
        let available = actions
            .primary_job
            .abilities
            .iter()
            .chain(
                actions
                    .secondary_jobs
                    .iter()
                    .flat_map(|job| job.abilities.iter()),
            )
            .any(|candidate| u64::from(candidate.id) == ability_id);
        if !available {
            return Some(WorkerOutput::RuntimeResponse(Box::new(
                RuntimeResponse::Error(format!("Ability {ability_id} is not available")),
            )));
        }
        let request_id = self.next_action_request_id;
        self.next_action_request_id += 1;
        self.transient_state
            .menu_path
            .push(format!("ability:{ability_id}"));
        self.enqueue_simulation_request(
            RuntimeRequest::OpenAbilityTargets {
                request_id,
                unit_id: actions.unit_id,
                ability_id,
            },
            PendingSimulation::Action { is_confirm: false },
        );
        None
    }

    fn select_menu_target(&mut self, target: &str) {
        let Ok(index) = target.parse::<usize>() else {
            return;
        };
        if let Some(menu) = self.transient_state.ability_targets.as_mut() {
            if index < menu.targets.len() {
                menu.selected_index = index;
            }
        }
    }

    fn select_ability_target(&mut self, direction: &str) {
        let Some(menu) = self.transient_state.ability_targets.as_mut() else {
            return;
        };
        if menu.targets.is_empty() {
            return;
        }
        menu.selected_index = pystral_runtime::demo::ability_targets::next_ability_target(
            &menu.targets,
            menu.selected_index,
            direction,
        );
    }

    pub(crate) fn apply_ability_targets_response(&mut self, response: &RuntimeResponse) {
        if let RuntimeResponse::AbilityTargets {
            request_id,
            unit_id,
            ability_id,
            target_session_id,
            state_version,
            snapshot_fingerprint,
            targets,
            disabled_reason,
        } = response
        {
            self.transient_state.ability_targets = Some(AbilityTargetMenu {
                request_id: *request_id,
                unit_id: *unit_id,
                ability_id: *ability_id,
                target_session_id: *target_session_id,
                state_version: *state_version,
                snapshot_fingerprint: *snapshot_fingerprint,
                targets: targets.clone(),
                selected_index: 0,
                disabled_reason: disabled_reason.clone(),
            });
            self.push_output(WorkerOutput::TransientState(Box::new(
                self.transient_state.clone(),
            )));
        }
    }

    fn select_preview_destination(&mut self, direction: &str) {
        let Some(preview) = self.transient_state.preview.as_mut() else {
            return;
        };
        let Some(current) = preview
            .selected_destination
            .as_ref()
            .or(preview.source.as_ref())
        else {
            return;
        };
        let (dq, dr) = match direction {
            "up" => (0, -1),
            "down" => (0, 1),
            "left" => (-1, 0),
            "right" => (1, 0),
            "layer-up" | "layer-down" => {
                self.select_preview_layer(direction);
                return;
            }
            _ => return,
        };
        let mut candidates = preview
            .reachable
            .iter()
            .filter(|candidate| {
                let delta_q = candidate.hex.x - current.hex.x;
                let delta_r = candidate.hex.y - current.hex.y;
                delta_q * dq + delta_r * dr > 0
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| {
            let distance = current.hex.distance_to(candidate.hex);
            let delta_q = candidate.hex.x - current.hex.x;
            let delta_r = candidate.hex.y - current.hex.y;
            (
                distance,
                -(delta_q * dq + delta_r * dr),
                candidate.layer,
                candidate.hex.x,
                candidate.hex.y,
            )
        });
        if let Some(next) = candidates.into_iter().next() {
            preview.selected_destination = Some(next);
            preview.path = preview_path(
                preview.source.as_ref().expect("preview source"),
                preview.selected_destination.as_ref(),
            );
        }
    }

    fn select_preview_layer(&mut self, direction: &str) {
        let Some(preview) = self.transient_state.preview.as_mut() else {
            return;
        };
        let Some(current) = preview
            .selected_destination
            .as_ref()
            .or(preview.source.as_ref())
        else {
            return;
        };
        let candidate_layer = if direction == "layer-up" {
            preview
                .reachable
                .iter()
                .map(|candidate| candidate.layer)
                .filter(|layer| *layer > current.layer)
                .min()
        } else {
            preview
                .reachable
                .iter()
                .map(|candidate| candidate.layer)
                .filter(|layer| *layer < current.layer)
                .max()
        };
        let Some(candidate_layer) = candidate_layer else {
            return;
        };
        let mut candidates = preview
            .reachable
            .iter()
            .filter(|candidate| candidate.layer == candidate_layer)
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| {
            (
                current.hex.distance_to(candidate.hex),
                candidate.hex.x,
                candidate.hex.y,
            )
        });
        if let Some(next) = candidates.into_iter().next() {
            preview.selected_destination = Some(next);
            preview.path = preview_path(
                preview.source.as_ref().expect("preview source"),
                preview.selected_destination.as_ref(),
            );
        }
    }

    pub fn spawn() -> ReactorBridge<UnifiedWorker> {
        gloo_worker::reactor::ReactorSpawner::<UnifiedWorker>::new()
            .as_module(true)
            .with_loader(true)
            .spawn("/worker.js")
    }
}

fn preview_path(source: &AvailableMove, destination: Option<&AvailableMove>) -> Vec<AvailableMove> {
    let Some(destination) = destination else {
        return Vec::new();
    };
    let mut path = source
        .hex
        .line_to(destination.hex)
        .into_iter()
        .map(|hex| AvailableMove {
            hex,
            layer: source.layer,
            ap_cost: 0,
        })
        .collect::<Vec<_>>();
    if source.layer != destination.layer {
        path.push(AvailableMove {
            hex: destination.hex,
            layer: destination.layer,
            ap_cost: destination.ap_cost,
        });
    } else if let Some(last) = path.last_mut() {
        last.ap_cost = destination.ap_cost;
    }
    path
}
