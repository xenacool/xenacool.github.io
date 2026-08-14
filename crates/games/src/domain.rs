use crate::{
    AbilityBehavior, AbilityDef, AbilityId, CombatVector, LogCommand, Logger, ModifierCard,
    MoveBehavior, ReactionBehavior, TacticalAccess, TacticalDiff, TacticalDisplayAction,
    TacticalState, UnitState, WaitBehavior,
};
pub use npc_engine_core::{
    AgentId, AgentValue, Context, ContextMut, Domain, StateDiffRef, StateDiffRefMut,
};
use npc_engine_utils::GlobalDomain;
use std::collections::BTreeSet;

pub struct TacticalDomain;

/// Estimate the chance that an ability produces positive damage for the
/// current modifier deck. This is a planning query: it never draws a card or
/// mutates the authoritative state.
pub fn ability_success_probability(
    state: &TacticalState,
    agent: AgentId,
    target: AgentId,
    ability_id: AbilityId,
) -> f32 {
    let Some(attacker) = state.agents.get(&agent) else {
        return 0.0;
    };
    let Some(defender) = state.agents.get(&target) else {
        return 0.0;
    };
    let Some(ability) = state.ability_registry.get(&ability_id) else {
        return 0.0;
    };
    let mut cards = attacker.modifier_deck.draw_pile.clone();
    if cards.is_empty() {
        cards = attacker.modifier_deck.discard_pile.clone();
    }
    if cards.is_empty() {
        cards.push(ModifierCard::Plus0);
    }
    let card_count = cards.len();
    let successes = cards
        .into_iter()
        .filter(|card| {
            calculate_damage(
                attacker,
                defender,
                ability,
                *card,
                "CON",
                &mut Logger::default(),
            ) > 0
        })
        .count();
    successes as f32 / card_count.max(1) as f32
}

/// Returns whether at least one possible modifier can kill the target. This
/// is used only for the explicitly permitted desperation exception.
pub fn ability_can_kill_with_any_modifier(
    state: &TacticalState,
    agent: AgentId,
    target: AgentId,
    ability_id: AbilityId,
) -> bool {
    let Some(attacker) = state.agents.get(&agent) else {
        return false;
    };
    let Some(defender) = state.agents.get(&target) else {
        return false;
    };
    let Some(ability) = state.ability_registry.get(&ability_id) else {
        return false;
    };
    let mut cards = attacker.modifier_deck.draw_pile.clone();
    if cards.is_empty() {
        cards = attacker.modifier_deck.discard_pile.clone();
    }
    if cards.is_empty() {
        cards.push(ModifierCard::Plus0);
    }
    cards.into_iter().any(|card| {
        calculate_damage(
            attacker,
            defender,
            ability,
            card,
            "CON",
            &mut Logger::default(),
        ) >= defender.health
    })
}

impl Domain for TacticalDomain {
    type State = TacticalState;
    type Diff = TacticalDiff;
    type DisplayAction = TacticalDisplayAction;

    fn list_behaviors() -> &'static [&'static dyn npc_engine_core::Behavior<Self>] {
        &[
            &ReactionBehavior,
            &MoveBehavior,
            &AbilityBehavior,
            &WaitBehavior,
        ]
    }

    fn get_current_value(_tick: u64, state_diff: StateDiffRef<Self>, agent: AgentId) -> AgentValue {
        let team_id = state_diff.get_agent(agent).map(|u| u.team_id).unwrap_or(0);
        let mut score = 0.0;

        for id in state_diff.list_agents() {
            if let Some(unit) = state_diff.get_agent(id) {
                let unit_val = (unit.health as f32 / unit.derived_stats.health_max as f32) * 10.0
                    + (unit.mana as f32 / unit.derived_stats.mana_max as f32) * 2.0
                    + (unit.action_points as f32 / unit.derived_stats.action_points_max as f32)
                        * 0.1;

                if unit.team_id == team_id {
                    score += unit_val;
                    if unit.health <= 0 {
                        score -= 100.0;
                    }
                } else {
                    score -= unit_val;
                    if unit.health <= 0 {
                        score += 100.0;
                    }
                }

                // Prefer closing on enemies for friendly units.  Do not score
                // enemy proximity with the opposite sign: that made each
                // friendly/enemy pair cancel out, leaving movement choices
                // effectively ordered by hex coordinates.
                if unit.team_id == team_id {
                    let nearest_enemy_distance = state_diff
                        .list_agents()
                        .into_iter()
                        .filter_map(|enemy_id| {
                            state_diff
                                .get_agent(enemy_id)
                                .filter(|enemy| enemy.health > 0 && enemy.team_id != team_id)
                                .map(|enemy| unit.position.hex.distance_to(enemy.position.hex))
                        })
                        .min()
                        .unwrap_or(12) as f32;
                    score += (12.0 - nearest_enemy_distance).max(0.0) * 0.15;
                }
            }
        }

        npc_engine_core::AgentValue::new(score)
            .unwrap_or_else(|_| npc_engine_core::AgentValue::new(0.0).unwrap())
    }

    fn update_visible_agents(
        _start_tick: u64,
        _ctx: Context<Self>,
        _agents: &mut BTreeSet<AgentId>,
    ) {
        // Implementation for visibility/fog of war
    }
}

impl GlobalDomain for TacticalDomain {
    type GlobalState = TacticalState;

    fn derive_local_state(global_state: &Self::GlobalState, _agent: AgentId) -> Self::State {
        global_state.clone()
    }

    fn apply(global_state: &mut Self::GlobalState, _local_state: &Self::State, diff: &Self::Diff) {
        if let Some(new_rng) = &diff.rng_update {
            global_state.rng = new_rng.clone();
        }
        for (agent_id, unit_state) in &diff.agents {
            let mut unit_state = unit_state.clone();
            unit_state.health = unit_state
                .health
                .clamp(0, unit_state.derived_stats.health_max);
            unit_state.mana = unit_state.mana.min(unit_state.derived_stats.mana_max);
            unit_state.action_points = unit_state
                .action_points
                .min(unit_state.derived_stats.action_points_max);
            global_state.agents.insert(*agent_id, unit_state);
        }
        if let Some(queue) = &diff.reaction_queue_replace {
            global_state.reaction_queue = queue.clone();
        } else {
            global_state
                .reaction_queue
                .extend(diff.reaction_queue.iter().cloned());
        }
    }
}

pub fn calculate_damage(
    attacker: &UnitState,
    defender: &UnitState,
    ability: &AbilityDef,
    modifier_card: ModifierCard,
    defender_stat_name: &str,
    logger: &mut Logger,
) -> i32 {
    let attacker_stats = attacker.stats_with_passives();
    let defender_stats = defender.stats_with_passives();

    if defender_stat_name != "CON" {
        logger.apply_command(LogCommand::Error(format!(
            "Defender stat selection is deprecated: {defender_stat_name}"
        )));
    }
    let attack_vector =
        CombatVector::from_stats(&attacker_stats).component_mul(ability.attack_scaling_vector());
    for stat_name in ability.scaling.keys() {
        if !matches!(
            stat_name.as_str(),
            "STR" | "DEX" | "INT" | "WIS" | "CHA" | "CON" | "WITS" | "STA"
        ) {
            logger.apply_command(LogCommand::Error(format!(
                "Unknown attacker attribute: {stat_name}"
            )));
        }
    }
    let raw_total = attack_vector.sum();
    let modified_total = modifier_card.apply(raw_total as i32) as f32;
    let modified_vector = if raw_total.abs() > f32::EPSILON {
        attack_vector.scale(modified_total / raw_total)
    } else {
        attack_vector
    };
    // Armor class is authored on the existing roughly 0..=20 scale; normalize
    // it before applying it to the full defender-stat vector so ordinary
    // attacks remain meaningful while AC remains multiplicative.
    let defense_vector =
        CombatVector::from_stats(&defender_stats).scale(defender_stats.armor_class as f32 / 10.0);
    modified_vector
        .component_sub_clamped(defense_vector)
        .sum()
        .floor()
        .max(0.0) as i32
}
