use super::*;

impl Runtime {
    pub(super) fn commit_npc_ability(
        &mut self,
        request_id: u64,
        unit_id: u64,
        target: npc_engine_core::AgentId,
        ability: pystral_games::AbilityId,
    ) -> RuntimeResponse {
        let presentation_target = RuntimeAbilityTarget::Unit {
            unit_id: target.0 as u64,
        };
        let (
            projectile_route,
            presentation_facing,
            ability_name,
            presentation_animation,
            unit_states,
        ) = {
            let Some(sim) = self.pg_rpg_sim.as_ref() else {
                return RuntimeResponse::Error("Simulation not started".into());
            };
            let unit_states = [unit_id, target.0 as u64]
                .into_iter()
                .filter_map(|affected_id| {
                    sim.state
                        .agents
                        .get(&npc_engine_core::AgentId(affected_id as u32))
                        .map(|unit| {
                            (
                                affected_id,
                                unit.position.hex,
                                unit.position.layer,
                                unit.health,
                                unit.mana,
                                unit.action_points,
                            )
                        })
                })
                .collect::<Vec<_>>();
            let (ability_name, presentation_animation) = sim
                .state
                .ability_registry
                .get(&ability)
                .map(|definition| {
                    (
                        definition.name.clone(),
                        definition.presentation_animation.clone(),
                    )
                })
                .unwrap_or_else(|| ("unknown ability".to_string(), None));
            (
                Self::ability_projectile_route(sim, unit_id, ability, &presentation_target),
                Self::ability_presentation_facing(sim, unit_id, &presentation_target),
                ability_name,
                presentation_animation,
                unit_states,
            )
        };
        let projectile_events = projectile_route
            .map(|(profile, source, target)| self.projectile_flight_events(profile, source, target))
            .unwrap_or_default();
        let Some(history) = self.pg_rpg_history.as_mut() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let start_idx = history.log.len();
        history.push_and_apply(Event::Log {
            msg: format!(
                "NPC unit {unit_id} used {ability_name} on unit {}",
                target.0
            ),
        });
        for event in projectile_events {
            history.push_and_apply(event);
        }
        for (affected_id, hex, layer, health, mana, action_points) in unit_states {
            history.push_and_apply(Event::UnitStateChanged {
                unit_id: affected_id,
                hex,
                layer,
                health,
                mana,
                action_points,
            });
        }
        let barrier_id = Self::append_ability_animation_barrier(
            history,
            &mut self.pg_rpg_sequence_number,
            unit_id,
            presentation_animation.as_deref(),
            presentation_facing,
        );
        let mut update = HistoryManager::new();
        update.log = history.log[start_idx..].to_vec();
        RuntimeResponse::ActionCommitted {
            request_id,
            unit_id,
            action: "ability".into(),
            barrier_id,
            history: update,
        }
    }
}
