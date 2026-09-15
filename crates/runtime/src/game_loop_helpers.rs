use super::*;
pub(super) const BOUNDARY_WORK_BUDGET: usize = 8;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BoundaryResolution {
    Progress,
    Ready(npc_engine_core::AgentId),
    Completed(GameOutcome),
}
pub(super) fn resolve_pg_rpg_boundary(
    simulation: &pg_rpg::simulation::TacticalSimulation,
    ready_agents: &[npc_engine_core::AgentId],
) -> BoundaryResolution {
    if let Some(outcome) = simulation.outcome() {
        return BoundaryResolution::Completed(outcome);
    }
    ready_agents
        .iter()
        .copied()
        .find(|agent| simulation.is_alive(*agent))
        .map_or(BoundaryResolution::Progress, BoundaryResolution::Ready)
}
impl Runtime {
    pub(super) fn pending_reaction(&self) -> Option<PendingReaction> {
        let sim = self.pg_rpg_sim.as_ref()?;
        let (unit, reaction, target) = *sim.state.reaction_queue.first()?;
        let name = sim
            .state
            .reaction_registry
            .get(&reaction)
            .map(|definition| definition.name.clone())
            .unwrap_or_else(|| format!("Reaction {}", reaction.0));
        Some(PendingReaction {
            unit_id: unit.0 as u64,
            reaction_id: reaction.0 as u64,
            reaction_name: name,
            target_id: target.0 as u64,
            trigger_unit_id: 0,
            state_version: self.pg_rpg_sequence_number,
        })
    }
    pub(super) fn discard_dead_reactions(&mut self) {
        if let Some(sim) = self.pg_rpg_sim.as_mut() {
            let agents = sim.state.agents.clone();
            sim.state
                .reaction_queue
                .retain(|(owner, _, _)| agents.get(owner).is_some_and(|unit| unit.health > 0));
        }
    }
    pub(super) fn enter_pending_reaction(
        &mut self,
        mut reaction: PendingReaction,
        resume: ReactionResume,
    ) -> RuntimeContinuation {
        reaction.trigger_unit_id = resume.unit_id;
        reaction.state_version = self.pg_rpg_sequence_number;
        if self.pg_rpg_sim.as_ref().is_some_and(|sim| {
            sim.is_player_controlled(npc_engine_core::AgentId(reaction.unit_id as u32))
        }) {
            RuntimeContinuation::AwaitPlayerReaction { reaction, resume }
        } else {
            self.reaction_resume = Some(resume);
            let request_id = self.next_npc_request_id;
            self.next_npc_request_id += 1;
            RuntimeContinuation::AwaitMctsDecision {
                unit_id: reaction.unit_id,
                request_id,
                state_version: reaction.state_version,
            }
        }
    }
    pub(super) fn append_action_barrier(
        history: &mut HistoryManager,
        sequence_number: &mut u64,
    ) -> u64 {
        *sequence_number += 1;
        history.push_and_apply(Event::SequenceNumber(*sequence_number));
        *sequence_number
    }
    pub(super) fn append_ability_animation_barrier(
        history: &mut HistoryManager,
        sequence_number: &mut u64,
        unit_id: u64,
        clip: Option<&str>,
        facing: Option<pystral_games::Facing>,
    ) -> u64 {
        *sequence_number += 1;
        let barrier = *sequence_number;
        if let Some(facing) = facing {
            history.push_and_apply(Event::UpdateProperty {
                id: unit_id,
                property: "facing".to_string(),
                value: pystral_core::log::PropertyValue::String(facing.as_property().to_string()),
            });
        }
        if let Some(clip) = clip {
            history.push_and_apply(Event::UpdateProperty {
                id: unit_id,
                property: "animation_cue_clip".to_string(),
                value: pystral_core::log::PropertyValue::String(clip.to_string()),
            });
            history.push_and_apply(Event::UpdateProperty {
                id: unit_id,
                property: "animation_cue".to_string(),
                value: pystral_core::log::PropertyValue::Float(barrier as f32),
            });
            history.push_and_apply(Event::UpdateProperty {
                id: unit_id,
                property: "animation_barrier".to_string(),
                value: pystral_core::log::PropertyValue::Float(barrier as f32),
            });
        }
        history.push_and_apply(Event::SequenceNumber(barrier));
        barrier
    }
    pub(super) fn ability_presentation_facing(
        simulation: &pg_rpg::simulation::TacticalSimulation,
        unit_id: u64,
        target: &RuntimeAbilityTarget,
    ) -> Option<pystral_games::Facing> {
        let from = simulation
            .state
            .agents
            .get(&npc_engine_core::AgentId(unit_id as u32))?
            .position
            .hex;
        let target_hex = match target {
            RuntimeAbilityTarget::Unit { unit_id } => {
                simulation
                    .state
                    .agents
                    .get(&npc_engine_core::AgentId(*unit_id as u32))?
                    .position
                    .hex
            }
            RuntimeAbilityTarget::Cell { hex, .. } => *hex,
        };
        pystral_games::Facing::direction_to(from, target_hex)
    }
    pub(super) fn default_movement_transition() -> pystral_core::log::TransitionConfig {
        Self::movement_transition(1)
    }
    pub(super) fn movement_transition(steps: usize) -> pystral_core::log::TransitionConfig {
        pystral_core::log::TransitionConfig {
            duration_ms: 500u32.saturating_mul(steps.max(1) as u32),
            delta_time_ms: 16.0,
            tween: pystral_core::log::TweenKind::SineInOut,
        }
    }
    pub(super) fn step_pg_rpg_simulation(&mut self) -> RuntimeResponse {
        if self.continuation != RuntimeContinuation::AwaitBoundary {
            return RuntimeResponse::Error(
                "Simulation step requested outside AwaitBoundary".to_string(),
            );
        }
        let Some(session) = self.rhai_session.as_mut() else {
            return RuntimeResponse::Error("Rhai simulation session not started".to_string());
        };
        let ready_agents = match session.resume_game_budgeted(BOUNDARY_WORK_BUDGET) {
            Ok(ready_agents) => ready_agents,
            Err(error) => return RuntimeResponse::Error(format!("Rhai Error: {error}")),
        };
        let simulation = match session.simulation() {
            Ok(simulation) => simulation,
            Err(error) => return RuntimeResponse::Error(error),
        };
        self.pg_rpg_sim = Some(simulation);
        let (Some(sim), Some(history)) = (self.pg_rpg_sim.as_mut(), self.pg_rpg_history.as_mut())
        else {
            return RuntimeResponse::Error("Simulation not started".to_string());
        };
        let boundary = resolve_pg_rpg_boundary(sim, &ready_agents);
        let start_idx = history.log.len();
        if matches!(boundary, BoundaryResolution::Progress) {
            self.pg_rpg_sequence_number += 1;
            history.push_and_apply(Event::SequenceNumber(self.pg_rpg_sequence_number));
            let mut update = HistoryManager::new();
            update.log = history.log[start_idx..].to_vec();
            return RuntimeResponse::PgRpgSimulationSlice {
                work_units: BOUNDARY_WORK_BUDGET as u32,
                history: update,
            };
        }
        if let BoundaryResolution::Ready(id) = boundary {
            Self::append_turn_events(history, sim, id);
        }
        if let BoundaryResolution::Completed(outcome) = boundary {
            if self.pg_rpg_completion_emitted {
                return RuntimeResponse::Error("Duplicate pg_rpg completion boundary".to_string());
            }
            history.push_and_apply(Event::GameCompleted {
                winning_team: sim.winning_team(),
                outcome: outcome.clone(),
                completed_rounds: sim.completed_rounds,
            });
            self.pg_rpg_completion_emitted = true;
            self.continuation = RuntimeContinuation::Completed;
            self.pg_rpg_sequence_number += 1;
            history.push_and_apply(Event::SequenceNumber(self.pg_rpg_sequence_number));
            let mut update = HistoryManager::new();
            update.log = history.log[start_idx..].to_vec();
            return RuntimeResponse::GameCompleted {
                outcome,
                history: update,
            };
        } else if let BoundaryResolution::Ready(agent) = boundary {
            sim.state.agents.get(&agent).map(|_unit| {
                if sim.is_player_controlled(agent) {
                    self.continuation = RuntimeContinuation::AwaitPlayerDecision {
                        unit_id: agent.0 as u64,
                    };
                } else {
                    let request_id = self.next_npc_request_id;
                    self.next_npc_request_id += 1;
                    self.continuation = RuntimeContinuation::AwaitMctsDecision {
                        unit_id: agent.0 as u64,
                        request_id,
                        state_version: self.pg_rpg_sequence_number,
                    };
                }
            });
        }
        self.pg_rpg_sequence_number += 1;
        history.push_and_apply(Event::SequenceNumber(self.pg_rpg_sequence_number));
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::PgRpgSimulationStepped(update)
    }
    pub(super) fn request_mcts_decision(
        &mut self,
        request_id: u64,
        unit_id: u64,
        state_version: u64,
    ) -> RuntimeResponse {
        let RuntimeContinuation::AwaitMctsDecision {
            unit_id: expected_unit,
            request_id: expected_request,
            state_version: expected_version,
        } = self.continuation
        else {
            return RuntimeResponse::Error("Unexpected MCTS request outside MCTS boundary".into());
        };
        if (request_id, unit_id, state_version)
            != (expected_request, expected_unit, expected_version)
        {
            return RuntimeResponse::Error("Stale MCTS request".into());
        }
        let Some(sim) = self.pg_rpg_sim.as_ref() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let action = sim
            .request_npc_decision(npc_engine_core::AgentId(unit_id as u32))
            .unwrap_or(pystral_games::TacticalDisplayAction::Wait);
        let decision = RuntimeDecision {
            unit_id,
            action: match action {
                pystral_games::TacticalDisplayAction::Move { to } => RuntimeDecisionAction::Move {
                    hex: to.hex,
                    layer: to.layer,
                },
                pystral_games::TacticalDisplayAction::Ability { target, ability } => {
                    RuntimeDecisionAction::Ability {
                        ability_id: ability.0 as u64,
                        target: RuntimeAbilityTarget::Unit {
                            unit_id: target.0 as u64,
                        },
                    }
                }
                pystral_games::TacticalDisplayAction::Reaction { reaction, target } => {
                    RuntimeDecisionAction::Reaction {
                        reaction_id: reaction.0 as u64,
                        target: target.0 as u64,
                    }
                }
                pystral_games::TacticalDisplayAction::Face { facing } => {
                    RuntimeDecisionAction::Face {
                        facing: facing.as_property().into(),
                    }
                }
                pystral_games::TacticalDisplayAction::Wait => RuntimeDecisionAction::Wait,
            },
        };
        RuntimeResponse::MctsDecisionReady {
            request_id,
            decision,
            state_version,
        }
    }
    pub(super) fn apply_mcts_decision(
        &mut self,
        request_id: u64,
        decision: RuntimeDecision,
        state_version: u64,
    ) -> RuntimeResponse {
        let RuntimeContinuation::AwaitMctsDecision {
            unit_id,
            request_id: expected_request,
            state_version: expected_version,
        } = self.continuation
        else {
            return RuntimeResponse::Error("Unexpected MCTS result outside MCTS boundary".into());
        };
        if request_id != expected_request
            || state_version != expected_version
            || decision.unit_id != unit_id
        {
            return RuntimeResponse::Error("Stale MCTS result".into());
        }
        let Some(sim) = self.pg_rpg_sim.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let action = match decision.action {
            RuntimeDecisionAction::Move { hex, layer } => {
                pystral_games::TacticalDisplayAction::Move {
                    to: GridCell::new(hex, layer),
                }
            }
            RuntimeDecisionAction::Wait => pystral_games::TacticalDisplayAction::Wait,
            RuntimeDecisionAction::Reaction {
                reaction_id,
                target,
            } => pystral_games::TacticalDisplayAction::Reaction {
                reaction: pystral_games::ReactionId(reaction_id as u32),
                target: npc_engine_core::AgentId(target as u32),
            },
            RuntimeDecisionAction::Ability { ability_id, target } => {
                let RuntimeAbilityTarget::Unit { unit_id: target_id } = target else {
                    return RuntimeResponse::Error(
                        "Cell ability targets are not yet executable".into(),
                    );
                };
                pystral_games::TacticalDisplayAction::Ability {
                    target: npc_engine_core::AgentId(target_id as u32),
                    ability: pystral_games::AbilityId(ability_id as u32),
                }
            }
            RuntimeDecisionAction::Face { facing } => {
                let Some(facing) = pystral_games::Facing::from_name(&facing) else {
                    return RuntimeResponse::Error("invalid NPC facing direction".into());
                };
                pystral_games::TacticalDisplayAction::Face { facing }
            }
        };
        let agent = npc_engine_core::AgentId(unit_id as u32);
        if !sim.is_alive(agent) {
            self.sync_rhai_simulation();
            self.continuation = RuntimeContinuation::AwaitBoundary;
            return self.resume_boundary();
        }
        let mut fallback_reason = None;
        let forced_reaction = sim.fallback_npc_action(agent).filter(|candidate| {
            matches!(
                candidate,
                pystral_games::TacticalDisplayAction::Reaction { .. }
            )
        });
        let submitted_action = if let Some(reaction) = forced_reaction {
            if action != reaction {
                fallback_reason = Some(format!(
                    "forced reaction {reaction:?} superseded NPC candidate {action:?}"
                ));
            }
            reaction
        } else {
            action
        };
        let submitted_route = match &submitted_action {
            pystral_games::TacticalDisplayAction::Move { to } => {
                pystral_games::validate_move(&sim.state, agent, *to)
                    .ok()
                    .map(|move_| move_.path)
            }
            _ => None,
        };
        let (action, route) = match sim.apply_npc_action(agent, submitted_action) {
            Ok(action) => (action, submitted_route),
            Err(reason) => {
                fallback_reason = Some(reason.clone());
                let fallback = sim
                    .fallback_npc_action(agent)
                    .ok_or_else(|| "no legal NPC fallback action".to_string());
                match fallback {
                    Ok(fallback) => {
                        let route = match &fallback {
                            pystral_games::TacticalDisplayAction::Move { to } => {
                                pystral_games::validate_move(&sim.state, agent, *to)
                                    .ok()
                                    .map(|move_| move_.path)
                            }
                            _ => None,
                        };
                        match sim.apply_npc_action(agent, fallback) {
                            Ok(action) => (action, route),
                            Err(wait_error) => {
                                return RuntimeResponse::Error(format!(
                                    "{reason}; fallback action failed: {wait_error}"
                                ));
                            }
                        }
                    }
                    Err(wait_error) => {
                        return RuntimeResponse::Error(format!(
                            "{reason}; fallback action failed: {wait_error}"
                        ));
                    }
                }
            }
        };
        self.sync_rhai_simulation();
        let committed_action = action.clone();
        let mut response = match action {
            pystral_games::TacticalDisplayAction::Move { to } => match route {
                Some(route) => self.commit_npc_move(request_id, unit_id, to, route),
                None => return RuntimeResponse::Error("validated NPC move has no route".into()),
            },
            pystral_games::TacticalDisplayAction::Wait => self.commit_npc_wait(request_id, unit_id),
            pystral_games::TacticalDisplayAction::Ability { target, ability } => {
                self.commit_npc_ability(request_id, unit_id, target, ability)
            }
            pystral_games::TacticalDisplayAction::Reaction { reaction, target } => {
                self.commit_npc_reaction(request_id, unit_id, reaction, target)
            }
            pystral_games::TacticalDisplayAction::Face { facing } => {
                self.commit_npc_facing(request_id, unit_id, facing)
            }
        };
        if let Some(reason) = fallback_reason {
            let event = pystral_core::log::Event::Log {
                msg: format!(
                    "NPC unit {unit_id} candidate rejected; fallback {committed_action:?}: {reason}"
                ),
            };
            if let RuntimeResponse::ActionCommitted { history, .. } = &mut response {
                history.push_and_apply(event.clone());
                if let Some(full_history) = self.pg_rpg_history.as_mut() {
                    full_history.push_and_apply(event);
                }
            }
        }
        response
    }
    fn commit_npc_facing(
        &mut self,
        request_id: u64,
        unit_id: u64,
        facing: pystral_games::Facing,
    ) -> RuntimeResponse {
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::UpdateProperty {
            id: unit_id,
            property: "facing".into(),
            value: pystral_core::log::PropertyValue::String(facing.as_property().into()),
        });
        let barrier_id = Self::append_action_barrier(history, &mut self.pg_rpg_sequence_number);
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: "face".into(),
            barrier_id,
            history: update,
        }
    }
    fn commit_npc_reaction(
        &mut self,
        request_id: u64,
        unit_id: u64,
        reaction: pystral_games::ReactionId,
        target: npc_engine_core::AgentId,
    ) -> RuntimeResponse {
        let reaction_name = self
            .pg_rpg_sim
            .as_ref()
            .and_then(|sim| sim.state.reaction_registry.get(&reaction))
            .map(|definition| definition.name.as_str())
            .unwrap_or("unknown reaction");
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::Log {
            msg: format!(
                "NPC unit {unit_id} resolved {reaction_name} (reaction {}) against unit {}",
                reaction.0, target.0
            ),
        });
        if let Some(sim) = self.pg_rpg_sim.as_ref() {
            for affected_id in [unit_id, target.0 as u64] {
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
            action: "reaction".into(),
            barrier_id,
            history: update,
        }
    }
    fn commit_npc_move(
        &mut self,
        request_id: u64,
        unit_id: u64,
        destination: GridCell,
        path: Vec<GridCell>,
    ) -> RuntimeResponse {
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::Log {
            msg: format!(
                "NPC unit {unit_id} moved to q {}, r {}, layer {}",
                destination.hex.x, destination.hex.y, destination.layer
            ),
        });
        let transition = Self::movement_transition(path.len().saturating_sub(1));
        history.push_and_apply(Event::MoveSprite {
            id: unit_id,
            destination: destination.hex,
            path: path
                .into_iter()
                .map(|cell| pystral_core::log::MovementWaypoint {
                    hex: cell.hex,
                    layer: cell.layer,
                })
                .collect(),
            transition: Some(transition),
        });
        history.push_and_apply(Event::UpdateProperty {
            id: unit_id,
            property: "layer".to_string(),
            value: pystral_core::log::PropertyValue::Float(destination.layer as f32),
        });
        if let Some(unit) = self.pg_rpg_sim.as_ref().and_then(|sim| {
            sim.state
                .agents
                .get(&npc_engine_core::AgentId(unit_id as u32))
        }) {
            history.push_and_apply(Event::UpdateProperty {
                id: unit_id,
                property: "facing".to_string(),
                value: pystral_core::log::PropertyValue::String(unit.facing.as_property().into()),
            });
        }
        let barrier_id = Self::append_action_barrier(history, &mut self.pg_rpg_sequence_number);
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: "move".into(),
            barrier_id,
            history: update,
        }
    }
    fn commit_npc_wait(&mut self, request_id: u64, unit_id: u64) -> RuntimeResponse {
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let Some(sim) = self.pg_rpg_sim.as_ref() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::TurnCompleted { unit_id });
        let position = sim.get_agent_position(unit_id as i64);
        if let Some(unit) = sim
            .state
            .agents
            .get(&npc_engine_core::AgentId(unit_id as u32))
        {
            history.push_and_apply(Event::UnitStateChanged {
                unit_id,
                hex: position.hex,
                layer: position.layer,
                health: unit.health,
                mana: unit.mana,
                action_points: unit.action_points,
            });
        }
        let barrier_id = Self::append_action_barrier(history, &mut self.pg_rpg_sequence_number);
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: "wait".into(),
            barrier_id,
            history: update,
        }
    }
    fn append_turn_events(
        history: &mut HistoryManager,
        sim: &pg_rpg::simulation::TacticalSimulation,
        id: npc_engine_core::AgentId,
    ) {
        let unit_id = id.0 as u64;
        history.push_and_apply(Event::TurnStarted { unit_id });
        if let Some(actions) = sim.get_available_actions(id.0 as i64) {
            history.push_and_apply(Event::AvailableActions(actions));
        }
        let position = sim.get_agent_position(id.0 as i64);
        history.push_and_apply(Event::MoveSprite {
            id: unit_id,
            destination: position.hex,
            path: vec![pystral_core::log::MovementWaypoint {
                hex: position.hex,
                layer: position.layer,
            }],
            transition: Some(Self::default_movement_transition()),
        });
        if let Some(unit) = sim.state.agents.get(&id) {
            history.push_and_apply(Event::UpdateProperty {
                id: unit_id,
                property: "facing".to_string(),
                value: pystral_core::log::PropertyValue::String(unit.facing.as_property().into()),
            });
        }
        let prompt_id = 999;
        history.push_and_apply(Event::UpdateProperty {
            id: prompt_id,
            property: "visible".to_string(),
            value: pystral_core::log::PropertyValue::String("true".to_string()),
        });
        for (key, value) in sim.get_prompts(id.0 as i64) {
            history.push_and_apply(Event::UpdateProperty {
                id: prompt_id,
                property: key,
                value: pystral_core::log::PropertyValue::String(value.to_string()),
            });
        }
        if let Some(unit) = sim.state.agents.get(&id) {
            history.push_and_apply(Event::UnitStateChanged {
                unit_id,
                hex: unit.position.hex,
                layer: unit.position.layer,
                health: unit.health,
                mana: unit.mana,
                action_points: unit.action_points,
            });
        }
    }
    pub(super) fn sync_rhai_simulation(&mut self) {
        if let (Some(session), Some(simulation)) =
            (self.rhai_session.as_mut(), self.pg_rpg_sim.as_ref())
        {
            session.set_simulation(simulation.clone());
        }
    }
}
#[cfg(test)]
mod tests;
