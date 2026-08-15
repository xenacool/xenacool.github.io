use super::*;

impl Runtime {
    pub(super) fn start_pg_rpg_simulation(
        &mut self,
        bundle: ScenarioBundle,
        atlas_json: String,
        spritesheet_rgba: Vec<u8>,
        spritesheet_width: u32,
    ) -> RuntimeResponse {
        let script = match bundle.root_rhai() {
            Ok(script) => script,
            Err(error) => return RuntimeResponse::Error(error),
        };
        let mut session = match RhaiSession::new(
            &script,
            HistoryManager::new(),
            atlas_json,
            spritesheet_rgba,
            spritesheet_width,
        ) {
            Ok(session) => session,
            Err(error) => return RuntimeResponse::Error(format!("Rhai Error: {error}")),
        };
        let history = match session.history() {
            Ok(history) => history,
            Err(error) => return RuntimeResponse::Error(error),
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

    pub(super) fn run_rhai_case(
        &self,
        workspace: VirtualRhaiWorkspace,
        case_name: String,
        seed: u64,
    ) -> RuntimeResponse {
        let mut session = match RhaiSession::from_virtual_workspace(
            &workspace,
            HistoryManager::new(),
            String::new(),
            Vec::new(),
            0,
            seed,
        ) {
            Ok(session) => session,
            Err(error) => return RuntimeResponse::Error(format!("Rhai Error: {error}")),
        };
        match session.run_named_case_json(&case_name) {
            Ok(details) => RuntimeResponse::RhaiCaseResult {
                case_name,
                seed,
                replay_header: session.replay_header(),
                details,
            },
            Err(error) => RuntimeResponse::Error(format!("Rhai case error: {error}")),
        }
    }

    pub(super) fn commit_ability_request(
        &mut self,
        request_id: u64,
        unit_id: u64,
        ability_id: u64,
        target: RuntimeAbilityTarget,
        provenance: Option<DecisionProvenance>,
    ) -> RuntimeResponse {
        if let Err(error) = self.validate_ability_provenance(unit_id, ability_id, provenance) {
            return RuntimeResponse::Error(error);
        }
        let Some(sim) = self.pg_rpg_sim.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".to_string());
        };
        let simulation_before_request = sim.clone();
        // A reaction is a mandatory response in the tactical rules.  The
        // player protocol does not expose reaction choices yet, so consume a
        // pending reaction for this unit before revalidating the ability the
        // player selected.  Keeping this in the commit path is important: a
        // reaction can have been queued after the target menu was opened.
        let forced_reaction = sim
            .state
            .reaction_queue
            .iter()
            .find(|(agent, _, _)| *agent == npc_engine_core::AgentId(unit_id as u32))
            .map(
                |(_, reaction, target)| pystral_games::TacticalDisplayAction::Reaction {
                    reaction: *reaction,
                    target: *target,
                },
            );
        if let Some(reaction) = forced_reaction.as_ref() {
            if let Err(error) =
                sim.apply_npc_action(npc_engine_core::AgentId(unit_id as u32), reaction.clone())
            {
                return RuntimeResponse::ActionRejected {
                    request_id,
                    reason: ActionError::IllegalAbility(error),
                };
            }
        }
        let history_start_idx = self
            .pg_rpg_history
            .as_ref()
            .map(|history| history.log.len())
            .unwrap_or_default();
        let affected = match target {
            RuntimeAbilityTarget::Unit { unit_id: target_id } => {
                let action = pystral_games::TacticalDisplayAction::Ability {
                    target: npc_engine_core::AgentId(target_id as u32),
                    ability: pystral_games::AbilityId(ability_id as u32),
                };
                if let Err(error) =
                    sim.apply_npc_action(npc_engine_core::AgentId(unit_id as u32), action)
                {
                    // The reaction and ability form one player request.  Do
                    // not leave a partially committed reaction behind if the
                    // final authoritative ability check fails.
                    *sim = simulation_before_request.clone();
                    return RuntimeResponse::ActionRejected {
                        request_id,
                        reason: ActionError::IllegalAbility(error),
                    };
                }
                vec![target_id]
            }
            RuntimeAbilityTarget::Cell { hex, layer } => match sim.commit_area_ability(
                npc_engine_core::AgentId(unit_id as u32),
                pystral_games::AbilityId(ability_id as u32),
                GridCell::new(hex, layer),
            ) {
                Ok(affected) => affected.into_iter().map(|id| id.0 as u64).collect(),
                Err(error) => {
                    *sim = simulation_before_request.clone();
                    return RuntimeResponse::ActionRejected {
                        request_id,
                        reason: ActionError::IllegalAbility(error),
                    };
                }
            },
        };
        self.sync_rhai_simulation();
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".to_string());
        };
        let start_idx = history_start_idx.min(history.log.len());
        if let Some(pystral_games::TacticalDisplayAction::Reaction { reaction, target }) =
            forced_reaction
        {
            history.push_and_apply(Event::Log {
                msg: format!(
                    "Unit {unit_id} resolved forced reaction {} against {}",
                    reaction.0, target.0
                ),
            });
        }
        history.push_and_apply(Event::Log {
            msg: format!(
                "Unit {unit_id} used ability {ability_id} on {} target(s)",
                affected.len()
            ),
        });
        if let Some(sim) = self.pg_rpg_sim.as_ref() {
            for affected_id in std::iter::once(unit_id).chain(affected.iter().copied()) {
                if let Some(unit) = sim
                    .state
                    .agents
                    .get(&npc_engine_core::AgentId(affected_id as u32))
                {
                    history.push_and_apply(Event::UnitStateChanged {
                        unit_id: affected_id,
                        hex: unit.position.hex,
                        layer: unit.position.layer,
                        health: unit.health,
                        mana: unit.mana,
                        action_points: unit.action_points,
                    });
                }
            }
        }
        let barrier_id = Self::append_action_barrier(history, &mut self.pg_rpg_sequence_number);
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: format!(
                "ability ({} target{})",
                affected.len(),
                if affected.len() == 1 { "" } else { "s" }
            ),
            barrier_id,
            history: update,
        }
    }

    pub(super) fn update_continuation(&mut self, response: &RuntimeResponse) {
        match response {
            RuntimeResponse::ActionCommitted {
                unit_id, action, ..
            } => {
                let npc_boundary = matches!(
                    self.continuation,
                    RuntimeContinuation::AwaitMctsDecision { .. }
                );
                self.continuation = RuntimeContinuation::AwaitAnimationAck {
                    barrier_id: self.pg_rpg_sequence_number,
                    unit_id: *unit_id,
                    ends_turn: action == "wait",
                    npc: npc_boundary,
                };
            }
            RuntimeResponse::ActionRejected { request_id, .. } => {
                self.continuation = RuntimeContinuation::RecoverRejected {
                    request_id: *request_id,
                    unit_id: self.continuation_unit_id(),
                };
            }
            _ => {}
        }
    }

    pub(super) fn reject_after_completion(
        &self,
        request: &RuntimeRequest,
    ) -> Option<(RuntimeResponse, Vec<String>)> {
        if self.continuation != RuntimeContinuation::Completed
            || !matches!(
                request,
                RuntimeRequest::StepPgRpgSimulation
                    | RuntimeRequest::RequestMctsDecision { .. }
                    | RuntimeRequest::MctsDecisionReady { .. }
                    | RuntimeRequest::OpenMovePreview { .. }
                    | RuntimeRequest::OpenAbilityTargets { .. }
                    | RuntimeRequest::ActionInput { .. }
                    | RuntimeRequest::CommitMove { .. }
                    | RuntimeRequest::CommitWait { .. }
                    | RuntimeRequest::CommitDecision { .. }
                    | RuntimeRequest::AcknowledgeAnimation { .. }
                    | RuntimeRequest::ResumeBoundary
                    | RuntimeRequest::ResumeRejected { .. }
                    | RuntimeRequest::RunRhaiCase { .. }
            )
        {
            return None;
        }
        let message = "Unexpected gameplay request after game completion".to_string();
        Some((RuntimeResponse::Error(message.clone()), vec![message]))
    }

    pub(super) fn commit_move_request(
        &mut self,
        request_id: u64,
        unit_id: u64,
        destination: GridCell,
    ) -> RuntimeResponse {
        if let Err(message) = self.ensure_decision_boundary(unit_id) {
            return RuntimeResponse::Error(message);
        }
        let move_result = match self.pg_rpg_sim.as_mut() {
            Some(sim) => sim.commit_move(unit_id, destination),
            None => {
                return RuntimeResponse::ActionRejected {
                    request_id,
                    reason: ActionError::UnknownAgent(npc_engine_core::AgentId(unit_id as u32)),
                };
            }
        };
        let validated = match move_result {
            Ok(validated) => validated,
            Err(reason) => return RuntimeResponse::ActionRejected { request_id, reason },
        };
        self.sync_rhai_simulation();
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::MoveSprite {
            id: validated.agent.0 as u64,
            destination: validated.destination.hex,
            transition: Some(Self::default_movement_transition()),
        });
        history.push_and_apply(Event::UpdateProperty {
            id: validated.agent.0 as u64,
            property: "layer".to_string(),
            value: pystral_core::log::PropertyValue::Float(validated.destination.layer as f32),
        });
        let barrier_id = Self::append_action_barrier(history, &mut self.pg_rpg_sequence_number);
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: "move".to_string(),
            barrier_id,
            history: update,
        }
    }

    pub(super) fn commit_wait_request(&mut self, request_id: u64, unit_id: u64) -> RuntimeResponse {
        if let Err(message) = self.ensure_decision_boundary(unit_id) {
            return RuntimeResponse::Error(message);
        }
        let unit_snapshot = {
            let Some(sim) = self.pg_rpg_sim.as_mut() else {
                return RuntimeResponse::ActionRejected {
                    request_id,
                    reason: ActionError::UnknownAgent(npc_engine_core::AgentId(unit_id as u32)),
                };
            };
            if let Err(reason) = sim.commit_wait(unit_id) {
                return RuntimeResponse::ActionRejected { request_id, reason };
            }
            let position = sim.get_agent_position(unit_id as i64);
            sim.state
                .agents
                .get(&npc_engine_core::AgentId(unit_id as u32))
                .map(|unit| (position, unit.health, unit.mana, unit.action_points))
        };
        self.sync_rhai_simulation();
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::TurnCompleted { unit_id });
        if let Some((position, health, mana, action_points)) = unit_snapshot {
            history.push_and_apply(Event::UnitStateChanged {
                unit_id,
                hex: position.hex,
                layer: position.layer,
                health,
                mana,
                action_points,
            });
        }
        let barrier_id = Self::append_action_barrier(history, &mut self.pg_rpg_sequence_number);
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: "wait".to_string(),
            barrier_id,
            history: update,
        }
    }

    pub(super) fn acknowledge_animation(&mut self, barrier_id: u64) -> RuntimeResponse {
        match self.continuation {
            RuntimeContinuation::AwaitAnimationAck {
                barrier_id: expected,
                unit_id,
                ends_turn,
                npc,
            } if expected == barrier_id => {
                // A committed ability or reaction may have ended the game even
                // when it does not end the actor's turn. Do not offer another
                // decision after the terminal action; route completion through
                // the normal boundary step so it emits the typed outcome and
                // history event exactly once.
                let game_complete = self
                    .pg_rpg_sim
                    .as_ref()
                    .is_some_and(|simulation| simulation.is_complete());
                let actor_alive = self.pg_rpg_sim.as_ref().is_some_and(|simulation| {
                    simulation.is_alive(npc_engine_core::AgentId(unit_id as u32))
                });
                let next = if game_complete || ends_turn || !actor_alive {
                    RuntimeContinuation::AwaitBoundary
                } else if npc {
                    let request_id = self.next_npc_request_id;
                    self.next_npc_request_id += 1;
                    RuntimeContinuation::AwaitMctsDecision {
                        unit_id,
                        request_id,
                        state_version: self.pg_rpg_sequence_number,
                    }
                } else {
                    RuntimeContinuation::AwaitPlayerDecision { unit_id }
                };
                self.continuation = next.clone();
                RuntimeResponse::Continuation(next)
            }
            RuntimeContinuation::AwaitAnimationAck {
                barrier_id: expected,
                ..
            } => RuntimeResponse::Error(format!(
                "Unexpected animation acknowledgment {barrier_id}; expected {expected}"
            )),
            RuntimeContinuation::Completed => RuntimeResponse::Error(
                "Unexpected animation acknowledgment after game completion".to_string(),
            ),
            _ => RuntimeResponse::Error(
                "Unexpected animation acknowledgment without a pending barrier".to_string(),
            ),
        }
    }

    pub(super) fn resume_boundary(&mut self) -> RuntimeResponse {
        if self.continuation != RuntimeContinuation::AwaitBoundary {
            return RuntimeResponse::Error(
                "Unexpected boundary resume outside AwaitBoundary".to_string(),
            );
        }
        // Tactical commits and presentation acknowledgments do not need to
        // copy the complete simulation into Rhai. Synchronize only when the
        // continuation crosses back into script-owned orchestration.
        self.sync_rhai_session();
        self.step_pg_rpg_simulation()
    }

    pub(super) fn resume_rejected(&mut self, request_id: u64) -> RuntimeResponse {
        let RuntimeContinuation::RecoverRejected {
            request_id: expected,
            unit_id,
        } = self.continuation
        else {
            return RuntimeResponse::Error(
                "Unexpected rejection resume without RecoverRejected".to_string(),
            );
        };
        if expected != request_id {
            return RuntimeResponse::Error(format!(
                "Unexpected rejection resume {request_id}; expected {expected}"
            ));
        }
        self.continuation = RuntimeContinuation::AwaitPlayerDecision { unit_id };
        RuntimeResponse::Continuation(self.continuation.clone())
    }

    pub(super) fn continuation_unit_id(&self) -> u64 {
        match self.continuation {
            RuntimeContinuation::AwaitPlayerDecision { unit_id }
            | RuntimeContinuation::AwaitMctsDecision { unit_id, .. }
            | RuntimeContinuation::AwaitAnimationAck { unit_id, .. }
            | RuntimeContinuation::RecoverRejected { unit_id, .. } => unit_id,
            _ => 0,
        }
    }

    pub(super) fn ensure_decision_boundary(&self, unit_id: u64) -> Result<(), String> {
        match self.continuation {
            RuntimeContinuation::AwaitBoundary => Ok(()),
            RuntimeContinuation::AwaitPlayerDecision { unit_id: expected }
                if expected == unit_id =>
            {
                Ok(())
            }
            RuntimeContinuation::AwaitPlayerDecision { unit_id: expected } => Err(format!(
                "Decision for unit {unit_id} is stale; unit {expected} owns the boundary"
            )),
            RuntimeContinuation::AwaitAnimationAck { .. } => {
                Err("Decision submitted before animation acknowledgment".to_string())
            }
            RuntimeContinuation::RecoverRejected { .. } => {
                Err("Decision submitted before rejection recovery".to_string())
            }
            RuntimeContinuation::Completed => {
                Err("Decision submitted after game completion".to_string())
            }
            RuntimeContinuation::AwaitMctsDecision { .. } => {
                Err("Player decision submitted during MCTS boundary".to_string())
            }
        }
    }
}
