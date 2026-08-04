use super::simulation::TacticalSimulation;
use crate::{AbilityTarget, AbilityTargetKind, Runtime, RuntimeResponse};
use pystral_games::{AbilityDelivery, AbilityId, AgentId, legal_ability_targets};

pub fn next_ability_target(targets: &[AbilityTarget], current: usize, direction: &str) -> usize {
    let Some(current_target) = targets.get(current) else {
        return 0;
    };
    let layer = current_target.layer;
    let candidates = if direction == "layer-up" {
        let next_layer = targets
            .iter()
            .map(|target| target.layer)
            .filter(|candidate| *candidate > layer)
            .min();
        targets
            .iter()
            .enumerate()
            .filter(move |(_, target)| Some(target.layer) == next_layer)
            .collect::<Vec<_>>()
    } else if direction == "layer-down" {
        let next_layer = targets
            .iter()
            .map(|target| target.layer)
            .filter(|candidate| *candidate < layer)
            .max();
        targets
            .iter()
            .enumerate()
            .filter(move |(_, target)| Some(target.layer) == next_layer)
            .collect::<Vec<_>>()
    } else {
        let mut same_layer = targets
            .iter()
            .enumerate()
            .filter(|(_, target)| target.layer == layer)
            .collect::<Vec<_>>();
        let by_q = matches!(direction, "left" | "right");
        if !matches!(direction, "up" | "down" | "left" | "right") {
            return current;
        }
        same_layer.sort_by_key(|(_, target)| {
            if by_q {
                (target.hex.x, target.hex.y, target.label.clone())
            } else {
                (target.hex.y, target.hex.x, target.label.clone())
            }
        });
        let position = same_layer
            .iter()
            .position(|(index, _)| *index == current)
            .unwrap_or(0);
        let offset = if matches!(direction, "right" | "down") {
            1
        } else {
            same_layer.len() - 1
        };
        return same_layer[(position + offset) % same_layer.len()].0;
    };
    candidates
        .into_iter()
        .min_by_key(|(_, target)| {
            (
                current_target.hex.distance_to(target.hex),
                target.hex.x,
                target.hex.y,
                target.label.clone(),
            )
        })
        .map_or(current, |(index, _)| index)
}

impl TacticalSimulation {
    pub fn ability_targets(
        &self,
        agent_id: u64,
        ability_id: u64,
    ) -> (Vec<AbilityTarget>, Option<String>) {
        let agent = AgentId(agent_id as u32);
        let Some(attacker) = self.state.agents.get(&agent) else {
            return (Vec::new(), Some(format!("Unknown unit {agent_id}")));
        };
        let Some(ability) = self
            .state
            .ability_registry
            .get(&AbilityId(ability_id as u32))
        else {
            return (Vec::new(), Some(format!("Unknown ability {ability_id}")));
        };
        let mut tags = attacker.turn_tags.clone();
        if attacker.action_points < i32::from(ability.get_ap_cost(&mut tags)) {
            return (
                Vec::new(),
                Some(format!("Insufficient action points for {}", ability.name)),
            );
        }
        let mut targets = match ability.delivery {
            AbilityDelivery::Area => self
                .state
                .grid
                .tiles
                .keys()
                .filter(|cell| {
                    attacker.position.layer.abs_diff(cell.layer) <= u32::from(ability.range)
                        && attacker.position.hex.distance_to(cell.hex) <= i32::from(ability.range)
                        && self.state.agents.iter().any(|(target, unit)| {
                            *target != agent
                                && unit.health > 0
                                && unit.team_id != attacker.team_id
                                && unit.position.layer == cell.layer
                                && unit.position.hex.distance_to(cell.hex)
                                    <= i32::from(ability.area_radius)
                        })
                })
                .map(|cell| AbilityTarget {
                    kind: AbilityTargetKind::Cell,
                    hex: cell.hex,
                    layer: cell.layer,
                    label: format!(
                        "Cell q {}, r {}, layer {}",
                        cell.hex.x, cell.hex.y, cell.layer
                    ),
                })
                .collect(),
            AbilityDelivery::SelfTarget => vec![AbilityTarget {
                kind: AbilityTargetKind::Unit { unit_id: agent_id },
                hex: attacker.position.hex,
                layer: attacker.position.layer,
                label: format!("Unit {agent_id} (self)"),
            }],
            _ => legal_ability_targets(&self.state, agent, AbilityId(ability_id as u32))
                .into_iter()
                .filter_map(|target| {
                    self.state.agents.get(&target).map(|unit| AbilityTarget {
                        kind: AbilityTargetKind::Unit {
                            unit_id: target.0 as u64,
                        },
                        hex: unit.position.hex,
                        layer: unit.position.layer,
                        label: format!(
                            "Unit {} at q {}, r {}, layer {}",
                            target.0, unit.position.hex.x, unit.position.hex.y, unit.position.layer
                        ),
                    })
                })
                .collect(),
        };
        targets.sort_by_key(|target| {
            (
                target.layer,
                target.hex.x,
                target.hex.y,
                target.label.clone(),
            )
        });
        let reason = targets
            .is_empty()
            .then(|| format!("No legal targets for {}", ability.name));
        (targets, reason)
    }
}

impl Runtime {
    pub(crate) fn open_ability_targets(
        &mut self,
        request_id: u64,
        unit_id: u64,
        ability_id: u64,
    ) -> RuntimeResponse {
        if let Err(message) = self.ensure_decision_boundary(unit_id) {
            return RuntimeResponse::Error(message);
        }
        let Some(sim) = self.demo_sim.as_ref() else {
            return RuntimeResponse::Error("Simulation not started".into());
        };
        let (targets, disabled_reason) = sim.ability_targets(unit_id, ability_id);
        let snapshot_fingerprint = sim.snapshot_fingerprint();
        let target_session_id = self.next_target_session_id;
        self.next_target_session_id += 1;
        self.active_target_session = Some((unit_id, ability_id, target_session_id));
        RuntimeResponse::AbilityTargets {
            request_id,
            unit_id,
            ability_id,
            target_session_id,
            state_version: self.demo_sequence_number,
            snapshot_fingerprint,
            targets,
            disabled_reason,
        }
    }

    pub(crate) fn validate_ability_provenance(
        &self,
        unit_id: u64,
        ability_id: u64,
        provenance: Option<crate::DecisionProvenance>,
    ) -> Result<(), String> {
        let Some(provenance) = provenance else {
            return Err("Missing ability target provenance".to_string());
        };
        if provenance.state_version != self.demo_sequence_number {
            return Err("Stale ability target state version".to_string());
        }
        let Some(simulation) = self.demo_sim.as_ref() else {
            return Err("Simulation not started".to_string());
        };
        if provenance.snapshot_fingerprint != simulation.snapshot_fingerprint() {
            return Err("Stale ability target snapshot".to_string());
        }
        if self.continuation_unit_id() != unit_id {
            return Err("Stale ability target unit".to_string());
        }
        if self.active_target_session != Some((unit_id, ability_id, provenance.target_session_id)) {
            return Err("Stale ability target session".to_string());
        }
        Ok(())
    }
}
