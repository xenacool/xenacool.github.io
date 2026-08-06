use std::collections::{HashMap, BTreeSet, BTreeMap};
use serde::{Deserialize, Serialize};
use pystral_core::ui_log::{Logger, LogCommand};
pub use npc_engine_core::{Behavior, Context, ContextMut, Domain, StateDiffRef, Task, AgentValue, AgentId, TaskDuration, impl_task_boxed_methods};
use npc_engine_utils::GlobalDomain;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorClassId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub struct GridCell {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Nonbinary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitStats {
    // Derived Stats
    pub health_max: i32,
    pub mana_max: i32,
    pub action_points_max: i32,

    // Base Attributes
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
    pub constitution: i32,
    pub wits: i32,
    pub stamina: i32,
    pub armor_class: i32,
    pub speed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SlotType {
    MainHand,
    OffHand,
    Head,
    Body,
    Accessory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentSlots {
    pub slots: HashMap<SlotType, Option<u64>>, // ItemId placeholder
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDef {
    pub id: JobId,
    pub equipment_slots: Vec<SlotType>,
    pub movement_slots_count: u8,
    pub passive_slots_count: u8,
    pub reaction_slots_count: u8,
    pub ability_slots_count: u8, // Typically 1 for secondary job ability
    pub innate_abilities: Vec<AbilityId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModifierCard {
    Plus0,
    Plus1,
    Minus1,
    Plus2,
    Minus2,
    Critical, // 2x
    Null,     // 0x
}

impl ModifierCard {
    pub fn apply(&self, value: i32) -> i32 {
        match self {
            ModifierCard::Plus0 => value,
            ModifierCard::Plus1 => value + 1,
            ModifierCard::Minus1 => value - 1,
            ModifierCard::Plus2 => value + 2,
            ModifierCard::Minus2 => value - 2,
            ModifierCard::Critical => value * 2,
            ModifierCard::Null => 0,
        }
    }

    pub fn is_reshuffle(&self) -> bool {
        matches!(self, ModifierCard::Critical | ModifierCard::Null)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilityModifierDeck {
    pub draw_pile: Vec<ModifierCard>,
    pub discard_pile: Vec<ModifierCard>,
    pub needs_reshuffle: bool,
}

impl AbilityModifierDeck {
    pub fn draw(&mut self, rng: &mut impl rand::Rng) -> ModifierCard {
        if self.draw_pile.is_empty() {
            self.reshuffle(rng);
        }
        
        // If still empty, return a default +0 (should not happen with a proper deck)
        if self.draw_pile.is_empty() {
            return ModifierCard::Plus0;
        }

        let card = self.draw_pile.remove(0);
        if card.is_reshuffle() {
            self.needs_reshuffle = true;
        }
        self.discard_pile.push(card);
        card
    }

    pub fn reshuffle(&mut self, rng: &mut impl rand::Rng) {
        use rand::seq::SliceRandom;
        self.draw_pile.append(&mut self.discard_pile);
        self.draw_pile.shuffle(rng);
        self.needs_reshuffle = false;
    }

    pub fn end_of_action(&mut self, rng: &mut impl rand::Rng) {
        if self.needs_reshuffle {
            self.reshuffle(rng);
        }
    }
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
    pub secondary_job: Option<JobId>,
    pub movement_ability: Option<AbilityId>,
    pub passive_abilities: Vec<AbilityId>,
    pub reaction_ability: Option<AbilityId>,
    pub stats: UnitStats,
    pub equipment: EquipmentSlots,
    pub status_effects: Vec<TagId>,
    pub modifier_deck: AbilityModifierDeck,
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

    fn get_current_value(_tick: u64, _state_diff: StateDiffRef<Self>, _agent: AgentId) -> AgentValue {
        // w_hp * (hp / hp_max)  +  w_pos * positional_score  -  w_opp * enemy_hp_sum
        0.0f32.try_into().unwrap()
    }

    fn update_visible_agents(_start_tick: u64, _ctx: Context<Self>, _agents: &mut BTreeSet<AgentId>) {
        // Implementation for visibility/fog of war
    }
}

impl GlobalDomain for TacticalDomain {
    type GlobalState = TacticalState;

    fn derive_local_state(global_state: &Self::GlobalState, _agent: AgentId) -> Self::State {
        // For local planning, we could prune the state to only include nearby agents
        global_state.clone()
    }

    fn apply(global_state: &mut Self::GlobalState, _local_state: &Self::State, diff: &Self::Diff) {
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
}

impl TacticalState {
    pub fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            grid: self.grid.clone(),
            collision: self.collision.clone(),
            logger: self.logger.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TagDef {
    pub id: TagId,
    pub max_stacks: u8,
}

pub struct TagRegistry {
    pub defs: HashMap<TagId, TagDef>,
}

#[derive(Default, Debug, Clone)]
pub struct TagBag {
    pub counts: HashMap<TagId, u8>,
}

impl TagBag {
    pub fn emit(&mut self, tag: TagId, n: u8, defs: &TagRegistry, logger: &mut Logger) {
        if let Some(def) = defs.defs.get(&tag) {
            let current = self.counts.entry(tag).or_insert(0);
            *current = (*current).saturating_add(n).min(def.max_stacks);
        } else {
            logger.apply_command(LogCommand::Log(format!("Attempted to emit undefined tag: {:?}", tag)));
        }
    }

    pub fn consume(&mut self, tag: TagId, n: u8) -> u8 {
        if let Some(current) = self.counts.get_mut(&tag) {
            let consumed = (*current).min(n);
            *current -= consumed;
            consumed
        } else {
            0
        }
    }
}

pub struct CTScheduler {
    pub agents: Vec<AgentId>,
    pub ct_threshold: i32,
}

impl CTScheduler {
    pub fn new(ct_threshold: i32) -> Self {
        Self {
            agents: Vec::new(),
            ct_threshold,
        }
    }

    pub fn tick_until_ready(&self, state: &mut TacticalState) -> Vec<AgentId> {
        loop {
            let mut ready = Vec::new();
            for (&id, agent) in state.agents.iter() {
                if agent.ct >= self.ct_threshold {
                    ready.push(id);
                }
            }

            if !ready.is_empty() {
                // Sort by WITS (descending) then by CT (descending) for stability
                ready.sort_by(|a, b| {
                    let agent_a = &state.agents[a];
                    let agent_b = &state.agents[b];
                    agent_b.stats.wits.cmp(&agent_a.stats.wits)
                        .then(agent_b.ct.cmp(&agent_a.ct))
                });
                return ready;
            }

            // Tick
            for agent in state.agents.values_mut() {
                agent.ct += agent.stats.speed;
            }
        }
    }

    pub fn calculate_deduction(&self, ap_spent: i32, ap_max: i32) -> i32 {
        if ap_max == 0 { return self.ct_threshold; }
        (self.ct_threshold * ap_spent) / ap_max
    }
}

#[derive(Debug, Clone)]
pub struct AbilityDef {
    pub ap_cost: u8,
    pub emit_tags: Vec<(TagId, u8)>,
    pub consume_tags: Vec<(TagId, u8, u8)>, // (tag, stacks, discount)
    pub scaling: HashMap<String, f32>,       // Attribute name -> scaling factor
}

#[derive(Debug, Clone)]
pub struct MoveProgram {
    pub steps_ap_cost: Vec<(u8, u8)>, // (step-threshold, AP cost)
    pub emit_tags: Vec<(TagId, u8)>,
    pub consume_tags: Vec<(TagId, u8, u8)>,
}

impl MoveProgram {
    pub fn get_ap_cost(&self, total_steps_so_far: u8, tag_bag: &mut TagBag) -> u8 {
        let current_step = total_steps_so_far + 1;
        let mut base_cost = 1;
        for &(threshold, c) in &self.steps_ap_cost {
            if current_step >= threshold {
                base_cost = c;
            } else {
                break;
            }
        }

        let mut discount = 0;
        for &(tag, stacks, d) in &self.consume_tags {
            if tag_bag.consume(tag, stacks) == stacks {
                discount += d;
            }
        }

        (base_cost as i32 - discount as i32).max(0) as u8
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
            movement_slots_count: 1,
            passive_slots_count: 2,
            reaction_slots_count: 1,
            ability_slots_count: 1,
            innate_abilities: vec![],
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
            secondary_job: None,
            movement_ability: None,
            passive_abilities: vec![],
            reaction_ability: None,
            stats: UnitStats {
                health_max: 100,
                mana_max: 50,
                action_points_max: 4,
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
