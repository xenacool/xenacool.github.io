use super::*;

impl Runtime {
    pub(super) fn commit_reaction_request(
        &mut self,
        request_id: u64,
        unit_id: u64,
        reaction_id: u64,
        target_id: u64,
        state_version: u64,
    ) -> RuntimeResponse {
        let RuntimeContinuation::AwaitPlayerReaction { reaction, .. } = &self.continuation else {
            return RuntimeResponse::Error("No player reaction is pending".into());
        };
        let reaction = reaction.clone();
        if reaction.unit_id != unit_id
            || reaction.reaction_id != reaction_id
            || reaction.target_id != target_id
            || reaction.state_version != state_version
        {
            return RuntimeResponse::Error("Stale reaction confirmation".into());
        }
        let action = pystral_games::TacticalDisplayAction::Reaction {
            reaction: pystral_games::ReactionId(reaction_id as u32),
            target: npc_engine_core::AgentId(target_id as u32),
        };
        let Some(sim) = self.pg_rpg_sim.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        if let Err(error) = sim.apply_npc_action(npc_engine_core::AgentId(unit_id as u32), action) {
            return RuntimeResponse::ActionRejected {
                request_id,
                reason: ActionError::IllegalAbility(error),
            };
        }
        self.sync_rhai_simulation();
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::Log {
            msg: format!(
                "Unit {unit_id} resolved {} against unit {target_id}",
                reaction.reaction_name
            ),
        });
        for affected_id in [unit_id, target_id] {
            if let Some(unit) = self.pg_rpg_sim.as_ref().and_then(|sim| {
                sim.state
                    .agents
                    .get(&npc_engine_core::AgentId(affected_id as u32))
            }) {
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
}
