use std::collections::BTreeSet;
pub use npc_engine_core::{Domain, StateDiffRef, AgentId, AgentValue, Context, ContextMut, StateDiffRefMut};
use npc_engine_utils::GlobalDomain;
use crate::{TacticalState, TacticalDiff, TacticalDisplayAction, TacticalAccess, ReactionBehavior, MoveBehavior, AbilityBehavior, WaitBehavior, UnitState, AbilityDef, ModifierCard, Logger, LogCommand};

pub struct TacticalDomain;

impl Domain for TacticalDomain {
    type State = TacticalState;
    type Diff = TacticalDiff;
    type DisplayAction = TacticalDisplayAction;

    fn list_behaviors() -> &'static [&'static dyn npc_engine_core::Behavior<Self>] {
        &[&ReactionBehavior, &MoveBehavior, &AbilityBehavior, &WaitBehavior]
    }

    fn get_current_value(_tick: u64, state_diff: StateDiffRef<Self>, agent: AgentId) -> AgentValue {
        let team_id = state_diff.get_agent(agent).map(|u| u.team_id).unwrap_or(0);
        let mut score = 0.0;

        for id in state_diff.list_agents() {
            if let Some(unit) = state_diff.get_agent(id) {
                let unit_val = (unit.health as f32 / unit.derived_stats.health_max as f32) * 10.0
                    + (unit.mana as f32 / unit.derived_stats.mana_max as f32) * 2.0
                    + (unit.action_points as f32 / unit.derived_stats.action_points_max as f32) * 1.0;
                
                if unit.team_id == team_id {
                    score += unit_val;
                    if unit.health <= 0 { score -= 100.0; }
                } else {
                    score -= unit_val;
                    if unit.health <= 0 { score += 100.0; }
                }
            }
        }
        
        score.try_into().unwrap()
    }

    fn update_visible_agents(_start_tick: u64, _ctx: Context<Self>, _agents: &mut BTreeSet<AgentId>) {
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
            global_state.agents.insert(*agent_id, unit_state.clone());
        }
        global_state.reaction_queue.extend(diff.reaction_queue.iter().cloned());
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

    let mut raw_damage = 0.0;
    for (stat_name, &scaling) in &ability.scaling {
        let stat_val = match stat_name.as_str() {
            "STR" => attacker_stats.strength,
            "DEX" => attacker_stats.dexterity,
            "INT" => attacker_stats.intelligence,
            "WIS" => attacker_stats.wisdom,
            "CHA" => attacker_stats.charisma,
            "CON" => attacker_stats.constitution,
            "WITS" => attacker_stats.wits,
            "STA" => attacker_stats.stamina,
            _ => {
                logger.apply_command(LogCommand::Log(format!("Unknown attacker attribute: {}", stat_name)));
                0
            }
        };
        raw_damage += stat_val as f32 * scaling;
    }

    let modified_damage = modifier_card.apply(raw_damage as i32);

    let defender_stat = match defender_stat_name {
        "CON" => defender_stats.constitution,
        "AGI" | "DEX" => defender_stats.dexterity,
        _ => {
            logger.apply_command(LogCommand::Log(format!("Unknown defender attribute: {}", defender_stat_name)));
            0
        }
    };

    let mitigation = defender_stat * defender_stats.armor_class;
    
    (modified_damage - mitigation).max(0)
}
