use std::collections::{HashMap, BTreeSet, BTreeMap};
use serde::{Deserialize, Serialize};
pub use npc_engine_core::{AgentId, StateDiffRef, StateDiffRefMut};
use crate::{TagId, TagBag, AbilityModifierDeck, DerivedStats, Gender, ActorClassId, JobId, MovementId, PassiveId, ReactionId, UnitStats, EquipmentSlots, TagRegistry, AbilityId, AbilityDef, JobDef, MoveProgram, ReactionDef, SeededRng, TacticalDomain};
use pystral_core::ui_log::Logger;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub struct GridCell {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UnitState {
    pub team_id: u8,
    pub health: i32,
    pub mana: i32,
    pub action_points: i32,
    pub ct: i32,
    pub position: (i32, i32, i32),
    pub gender: Gender,
    pub class_id: ActorClassId,
    pub primary_job: JobId,
    pub secondary_jobs: Vec<JobId>,
    pub movement_ability: MovementId,
    pub passive_abilities: Vec<PassiveId>,
    pub reaction_abilities: Vec<ReactionId>,
    pub stats: UnitStats,
    pub equipment: EquipmentSlots,
    pub status_effects: Vec<TagId>,
    pub turn_tags: TagBag,
    pub modifier_deck: AbilityModifierDeck,
    pub derived_stats: DerivedStats,
}

impl UnitState {
    pub fn stats_with_passives(&self) -> UnitStats {
        let mut stats = self.stats.clone();
        for passive in &self.passive_abilities {
            match passive.0 {
                101 => { // Thick Skin (Caveman)
                    stats.armor_class += 5;
                },
                401 => { // Undead Resilience (Skeleton)
                    stats.armor_class += 2;
                    stats.constitution += 2;
                },
                _ => {}
            }
        }
        stats
    }
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
    pub agents: BTreeMap<AgentId, UnitState>,
    pub rng_update: Option<SeededRng>,
    pub reaction_queue: Vec<(AgentId, ReactionId, AgentId)>,
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

#[derive(Debug, Clone)]
pub struct TacticalState {
    pub agents: HashMap<AgentId, UnitState>,
    pub grid: TacticalGrid,
    pub collision: Option<CollisionMap>,
    pub logger: Logger,
    pub reaction_queue: Vec<(AgentId, ReactionId, AgentId)>,
    pub rng: SeededRng,
    pub ability_registry: HashMap<AbilityId, AbilityDef>,
    pub job_registry: HashMap<JobId, JobDef>,
    pub movement_registry: HashMap<MovementId, MoveProgram>,
    pub reaction_registry: HashMap<ReactionId, ReactionDef>,
    pub tag_registry: TagRegistry,
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
            ability_registry: self.ability_registry.clone(),
            job_registry: self.job_registry.clone(),
            movement_registry: self.movement_registry.clone(),
            reaction_registry: self.reaction_registry.clone(),
            tag_registry: self.tag_registry.clone(),
        }
    }
}

pub trait TacticalAccess {
    fn get_agent(&self, id: AgentId) -> Option<&UnitState>;
    fn list_agents(&self) -> Vec<AgentId>;
}

impl TacticalAccess for StateDiffRef<'_, TacticalDomain> {
    fn get_agent(&self, id: AgentId) -> Option<&UnitState> {
        self.diff.agents.get(&id).or_else(|| self.initial_state.agents.get(&id))
    }
    fn list_agents(&self) -> Vec<AgentId> {
        let mut ids: BTreeSet<_> = self.initial_state.agents.keys().cloned().collect();
        ids.extend(self.diff.agents.keys().cloned());
        ids.into_iter().collect()
    }
}

impl TacticalAccess for StateDiffRefMut<'_, TacticalDomain> {
    fn get_agent(&self, id: AgentId) -> Option<&UnitState> {
        self.diff.agents.get(&id).or_else(|| self.initial_state.agents.get(&id))
    }
    fn list_agents(&self) -> Vec<AgentId> {
        let mut ids: BTreeSet<_> = self.initial_state.agents.keys().cloned().collect();
        ids.extend(self.diff.agents.keys().cloned());
        ids.into_iter().collect()
    }
}

pub trait TacticalAccessMut: TacticalAccess {
    fn get_agent_mut(&mut self, id: AgentId) -> Option<&mut UnitState>;
}

impl TacticalAccessMut for StateDiffRefMut<'_, TacticalDomain> {
    fn get_agent_mut(&mut self, id: AgentId) -> Option<&mut UnitState> {
        if self.diff.agents.contains_key(&id) {
            self.diff.agents.get_mut(&id)
        } else {
            self.initial_state.agents.get(&id).map(|unit| {
                self.diff.agents.insert(id, unit.clone());
                self.diff.agents.get_mut(&id).unwrap()
            })
        }
    }
}
