pub mod tags;
pub mod jobs;
pub mod abilities;
pub mod scheduler;
pub mod rng;

use std::collections::{HashMap, BTreeSet, BTreeMap};
use serde::{Deserialize, Serialize};
use pystral_core::ui_log::{Logger, LogCommand};
pub use npc_engine_core::{Behavior, Context, ContextMut, Domain, StateDiffRef, Task, AgentValue, AgentId, TaskDuration, impl_task_boxed_methods};
use npc_engine_utils::GlobalDomain;
use std::hash::Hash;

pub use tags::*;
pub use jobs::*;
pub use abilities::*;
pub use scheduler::*;
pub use rng::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub struct GridCell {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitState {
    pub health: i32,
    pub mana: i32,
    pub action_points: i32,
    pub ct: i32,
    pub position: (i32, i32, i32),
    pub gender: Gender,
    pub class_id: ActorClassId,
    pub primary_job: JobId,
    pub secondary_jobs: Vec<JobId>,
    pub movement_ability: Option<AbilityId>,
    pub passive_abilities: Vec<AbilityId>,
    pub reaction_abilities: Vec<AbilityId>,
    pub stats: UnitStats,
    pub equipment: EquipmentSlots,
    pub status_effects: Vec<TagId>,
    pub modifier_deck: AbilityModifierDeck,
    pub derived_stats: crate::DerivedStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TacticalGrid {
    // Placeholder using hexx
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollisionMap {
    // Placeholder
}

#[derive(Debug, Default, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TacticalDiff {
    pub health_changes: BTreeMap<AgentId, i32>,
    pub position_changes: BTreeMap<AgentId, (i32, i32, i32)>,
    pub rng_update: Option<SeededRng>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TacticalDisplayAction {
    Move { to: GridCell },
    Ability { target: AgentId, ability: AbilityId },
    Wait,
}

impl Default for TacticalDisplayAction {
    fn default() -> Self {
        TacticalDisplayAction::Wait
    }
}

pub struct TacticalDomain;

impl Domain for TacticalDomain {
    type State = TacticalState;
    type Diff = TacticalDiff;
    type DisplayAction = TacticalDisplayAction;

    fn list_behaviors() -> &'static [&'static dyn Behavior<Self>] {
        &[&MoveBehavior, &AbilityBehavior, &WaitBehavior]
    }

    fn get_current_value(_tick: u64, state_diff: StateDiffRef<Self>, agent: AgentId) -> AgentValue {
        0.0f32.try_into().unwrap()
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
        for (agent_id, health_change) in &diff.health_changes {
            if let Some(agent) = global_state.agents.get_mut(agent_id) {
                agent.health += health_change;
            }
        }
        for (agent_id, pos_change) in &diff.position_changes {
            if let Some(agent) = global_state.agents.get_mut(agent_id) {
                agent.position = *pos_change;
            }
        }
    }
}

pub struct MoveBehavior;
impl Behavior<TacticalDomain> for MoveBehavior {
    fn is_valid(&self, _ctx: Context<TacticalDomain>) -> bool {
        true
    }

    fn add_own_tasks(&self, _ctx: Context<TacticalDomain>, _tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        // Add move tasks based on UnitState and MoveProgram
    }
}

pub struct AbilityBehavior;
impl Behavior<TacticalDomain> for AbilityBehavior {
    fn is_valid(&self, _ctx: Context<TacticalDomain>) -> bool {
        true
    }

    fn add_own_tasks(&self, _ctx: Context<TacticalDomain>, _tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        // Add ability tasks based on UnitState and AbilityDefs
    }
}

pub struct WaitBehavior;
impl Behavior<TacticalDomain> for WaitBehavior {
    fn is_valid(&self, _ctx: Context<TacticalDomain>) -> bool {
        true
    }

    fn add_own_tasks(&self, ctx: Context<TacticalDomain>, tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        tasks.push(Box::new(WaitTask { agent: ctx.agent }));
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct WaitTask {
    pub agent: AgentId,
}

impl Task<TacticalDomain> for WaitTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        10 // Duration until next CT fire
    }

    fn execute(&self, _ctx: ContextMut<TacticalDomain>) -> Option<Box<dyn Task<TacticalDomain>>> {
        None
    }

    fn is_valid(&self, _ctx: Context<TacticalDomain>) -> bool {
        true
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Wait
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[derive(Debug, Clone)]
pub struct TacticalState {
    pub agents: HashMap<AgentId, UnitState>,
    pub grid: TacticalGrid,
    pub collision: Option<CollisionMap>,
    pub logger: Logger,
    pub reaction_queue: Vec<(AgentId, ReactionId)>,
    pub rng: SeededRng,
}

impl TacticalState {
    pub fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            grid: self.grid.clone(),
            collision: self.collision.clone(),
            logger: self.logger.clone(),
            reaction_queue: self.reaction_queue.clone(),
            rng: self.rng.clone(),
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
    let mut raw_damage = 0.0;
    for (stat_name, &scaling) in &ability.scaling {
        let stat_val = match stat_name.as_str() {
            "STR" => attacker.stats.strength,
            "DEX" => attacker.stats.dexterity,
            "INT" => attacker.stats.intelligence,
            "WIS" => attacker.stats.wisdom,
            "CHA" => attacker.stats.charisma,
            "CON" => attacker.stats.constitution,
            "WITS" => attacker.stats.wits,
            "STA" => attacker.stats.stamina,
            _ => {
                logger.apply_command(LogCommand::Log(format!("Unknown attacker attribute: {}", stat_name)));
                0
            }
        };
        raw_damage += stat_val as f32 * scaling;
    }

    let modified_damage = modifier_card.apply(raw_damage as i32);

    let defender_stat = match defender_stat_name {
        "CON" => defender.stats.constitution,
        "AGI" | "DEX" => defender.stats.dexterity,
        _ => {
            logger.apply_command(LogCommand::Log(format!("Unknown defender attribute: {}", defender_stat_name)));
            0
        }
    };

    let mitigation = defender_stat * defender.stats.armor_class;
    
    (modified_damage - mitigation).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_system_initialization() {
        let job = JobDef {
            id: JobId(1),
            equipment_slots: vec![SlotType::MainHand, SlotType::OffHand, SlotType::Head, SlotType::Body, SlotType::Accessory],
            passive_slots_count: 2,
            reaction_slots_count: 1,
            secondary_job_slots_count: 1,
            innate_abilities: vec![],
            innate_passives: vec![],
            innate_reactions: vec![],
        };

        let unit = UnitState {
            health: 100,
            mana: 50,
            action_points: 4,
            ct: 0,
            position: (0, 0, 0),
            gender: Gender::Male,
            class_id: ActorClassId(1),
            primary_job: JobId(1),
            secondary_jobs: vec![],
            movement_ability: None,
            passive_abilities: vec![],
            reaction_abilities: vec![],
            stats: UnitStats {
                strength: 10,
                dexterity: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
                constitution: 10,
                wits: 10,
                stamina: 10,
                armor_class: 10,
                speed: 10,
            },
            derived_stats: DerivedStats {
                health_max: 100,
                mana_max: 50,
                action_points_max: 4,
            },
            equipment: EquipmentSlots {
                slots: HashMap::new(),
            },
            status_effects: vec![],
            modifier_deck: AbilityModifierDeck::default(),
        };

        assert_eq!(unit.health, 100);
        assert_eq!(unit.primary_job, JobId(1));
        assert_eq!(job.equipment_slots.len(), 5);
    }
}
