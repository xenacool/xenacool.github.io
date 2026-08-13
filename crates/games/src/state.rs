use crate::{
    AbilityDef, AbilityId, AbilityModifierDeck, ActorClassId, CollisionMap, DerivedStat,
    DerivedStats, EquipmentSlots, Gender, JobDef, JobId, ModifierStacking, MoveProgram, MovementId,
    PassiveId, ReactionDef, ReactionId, SeededRng, TacticalDomain, TagBag, TagId, TagRegistry,
    TimedModifier, UnitStats,
};
use hexx::{Hex, HexBounds};
pub use npc_engine_core::{AgentId, StateDiffRef, StateDiffRefMut};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct JobHistoryEntry {
    pub job_id: JobId,
    pub consecutive_levels: u32,
}
use pystral_core::ui_log::Logger;

#[derive(Clone)]
struct ReachabilityNode {
    cell: GridCell,
    ap: u8,
    steps: u8,
    tags: TagBag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct GridCell {
    pub hex: Hex,
    pub layer: i32,
}

impl GridCell {
    pub const fn new(hex: Hex, layer: i32) -> Self {
        Self { hex, layer }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileType {
    Grass,
    Dirt,
    Rock,
}

impl TileType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "grass" => Some(Self::Grass),
            "dirt" => Some(Self::Dirt),
            "rock" => Some(Self::Rock),
            _ => None,
        }
    }

    pub const fn material_name(self) -> &'static str {
        match self {
            Self::Grass => "grass",
            Self::Dirt => "dirt",
            Self::Rock => "rock",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridBounds {
    pub horizontal: HexBounds,
    pub min_layer: i32,
    pub max_layer: i32,
}

impl Default for GridBounds {
    fn default() -> Self {
        Self {
            horizontal: HexBounds::from_radius(0),
            min_layer: 0,
            max_layer: 0,
        }
    }
}

impl GridBounds {
    pub fn contains(&self, cell: GridCell) -> bool {
        self.horizontal.is_in_bounds(cell.hex)
            && (self.min_layer..=self.max_layer).contains(&cell.layer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GridMap {
    pub bounds: GridBounds,
    pub tiles: HashMap<GridCell, TileType>,
}

impl GridMap {
    pub fn set_tile(&mut self, cell: GridCell, tile: TileType) -> Result<(), String> {
        if !self.bounds.contains(cell) {
            return Err(format!(
                "Grid cell {:?} is outside the configured bounds",
                cell
            ));
        }
        self.tiles.insert(cell, tile);
        Ok(())
    }

    pub fn contains(&self, cell: GridCell) -> bool {
        self.bounds.contains(cell) && self.tiles.contains_key(&cell)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UnitState {
    pub team_id: u8,
    pub health: i32,
    pub mana: i32,
    pub action_points: i32,
    pub ct: i32,
    pub position: GridCell,
    pub gender: Gender,
    pub class_id: ActorClassId,
    pub primary_job: JobId,
    pub secondary_jobs: Vec<JobId>,
    pub job_history: Vec<JobHistoryEntry>,
    pub purchased_abilities: Vec<AbilityId>,
    pub movement_ability: MovementId,
    pub passive_abilities: Vec<PassiveId>,
    pub reaction_abilities: Vec<ReactionId>,
    pub stats: UnitStats,
    pub equipment: EquipmentSlots,
    pub status_effects: Vec<TagId>,
    pub turn_tags: TagBag,
    pub modifier_deck: AbilityModifierDeck,
    pub timed_modifiers: Vec<TimedModifier>,
    pub derived_stats: DerivedStats,
}

impl UnitState {
    /// Applies one already-resolved job level. Resolution belongs to the
    /// ruleset/Rhai boundary; this method is the single tactical state update.
    pub fn apply_job_level(&mut self, job: &JobDef) -> Result<(), String> {
        self.stats = job.base_stats.clone();
        match self.job_history.last_mut() {
            Some(last) if last.job_id == job.id => {
                last.consecutive_levels = last
                    .consecutive_levels
                    .checked_add(1)
                    .ok_or_else(|| "Job history level count overflowed".to_string())?;
            }
            _ => self.job_history.push(JobHistoryEntry {
                job_id: job.id,
                consecutive_levels: 1,
            }),
        }
        self.derived_stats = DerivedStats {
            health_max: self.stats.constitution * 10,
            mana_max: self.stats.intelligence * 5,
            action_points_max: self.derived_stats.action_points_max,
        };
        self.health = self.health.min(self.derived_stats.health_max);
        self.mana = self.mana.min(self.derived_stats.mana_max);
        self.action_points = self.action_points.min(self.derived_stats.action_points_max);
        Ok(())
    }

    pub fn available_action_abilities(
        &self,
        job_registry: &HashMap<JobId, JobDef>,
    ) -> Result<Vec<AbilityId>, String> {
        let primary = job_registry
            .get(&self.primary_job)
            .ok_or_else(|| format!("Missing primary job definition: {:?}", self.primary_job))?;
        let mut abilities = primary.abilities.clone();
        for secondary_job in &self.secondary_jobs {
            let job = job_registry
                .get(secondary_job)
                .ok_or_else(|| format!("Missing secondary job definition: {secondary_job:?}"))?;
            for ability in &job.abilities {
                if !abilities.contains(ability) {
                    abilities.push(*ability);
                }
            }
        }
        for ability in &self.purchased_abilities {
            if !abilities.contains(ability) {
                abilities.push(*ability);
            }
        }
        Ok(abilities)
    }

    pub fn stats_with_passives(&self) -> UnitStats {
        let mut stats = self.stats.clone();
        for passive in &self.passive_abilities {
            match passive.0 {
                101 => {
                    // Thick Skin (Caveman)
                    stats.armor_class += 5;
                }
                401 => {
                    // Undead Resilience (Skeleton)
                    stats.armor_class += 2;
                    stats.constitution += 2;
                }
                _ => {}
            }
        }
        for modifier in &self.timed_modifiers {
            if modifier.stat == DerivedStat::ArmorClass {
                stats.armor_class += modifier.amount;
            }
        }
        stats
    }

    pub fn add_timed_modifier(&mut self, modifier: TimedModifier) -> Result<(), String> {
        if modifier.remaining_turns == 0 {
            return Err("Timed modifier must last at least one owner turn".to_string());
        }
        if modifier.stacking == ModifierStacking::RefreshReplace {
            self.timed_modifiers
                .retain(|existing| existing.stat != modifier.stat);
        }
        self.timed_modifiers.push(modifier);
        Ok(())
    }

    pub fn advance_owner_turn(&mut self) {
        for modifier in &mut self.timed_modifiers {
            modifier.remaining_turns = modifier.remaining_turns.saturating_sub(1);
        }
        self.timed_modifiers
            .retain(|modifier| modifier.remaining_turns > 0);
    }
}

pub type TacticalGrid = GridMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionError {
    UnknownAgent(AgentId),
    UnknownMovement(MovementId),
    IllegalDestination(GridCell),
    InsufficientActionPoints,
    IllegalAbility(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatedMove {
    pub agent: AgentId,
    pub destination: GridCell,
    pub ap_cost: u8,
}

pub fn validate_move(
    state: &TacticalState,
    agent: AgentId,
    destination: GridCell,
) -> Result<ValidatedMove, ActionError> {
    let unit = state
        .agents
        .get(&agent)
        .ok_or(ActionError::UnknownAgent(agent))?;
    let destinations = reachable_cells(state, agent)
        .map_err(|_| ActionError::UnknownMovement(unit.movement_ability))?;
    let ap_cost = destinations
        .get(&destination)
        .copied()
        .ok_or(ActionError::IllegalDestination(destination))?;
    if unit.action_points < i32::from(ap_cost) {
        return Err(ActionError::InsufficientActionPoints);
    }
    Ok(ValidatedMove {
        agent,
        destination,
        ap_cost,
    })
}

/// Computes destinations using the movement program rather than render or
/// caller-specific neighbor logic. Missing tiles can be traversed only by a
/// movement program that explicitly permits crossing holes; they are never
/// returned as destinations.
pub fn reachable_cells(
    state: &TacticalState,
    agent: AgentId,
) -> Result<HashMap<GridCell, u8>, String> {
    let unit = state
        .agents
        .get(&agent)
        .ok_or_else(|| format!("Unknown agent: {agent:?}"))?;
    let program = state
        .movement_registry
        .get(&unit.movement_ability)
        .ok_or_else(|| format!("Unknown movement ability: {:?}", unit.movement_ability))?;
    let occupied: std::collections::HashSet<GridCell> = state
        .agents
        .values()
        .map(|other| other.position)
        .filter(|&cell| cell != unit.position)
        .collect();
    if program.teleport_range.is_some() {
        let mut tags = unit.turn_tags.clone();
        let cost = program.get_ap_cost(0, &mut tags);
        let mut destinations = HashMap::new();
        for cell in movement_neighbors(unit.position, program) {
            if state.grid.bounds.contains(cell)
                && state.grid.contains(cell)
                && !occupied.contains(&cell)
                && i32::from(cost) <= unit.action_points
            {
                destinations.insert(cell, cost);
            }
        }
        return Ok(destinations);
    }
    let mut best = HashMap::from([(unit.position, 0)]);
    let mut frontier = vec![ReachabilityNode {
        cell: unit.position,
        ap: 0,
        steps: 0,
        tags: unit.turn_tags.clone(),
    }];

    while !frontier.is_empty() {
        let index = frontier
            .iter()
            .enumerate()
            .min_by_key(|(_, node)| node.ap)
            .map(|(index, _)| index)
            .unwrap();
        let node = frontier.swap_remove(index);
        if node.ap > best.get(&node.cell).copied().unwrap_or(u8::MAX) {
            continue;
        }

        let candidates = movement_neighbors(node.cell, program);
        for next in candidates {
            if !state.grid.bounds.contains(next)
                || occupied.contains(&next) && !program.crosses_occupied
            {
                continue;
            }
            let present = state.grid.contains(next);
            if !present && !program.crosses_holes {
                continue;
            }
            let mut tags = node.tags.clone();
            let cost = program.get_ap_cost(node.steps, &mut tags);
            let ap = node.ap.saturating_add(cost);
            if i32::from(ap) > unit.action_points {
                continue;
            }
            let steps = node.steps.saturating_add(1);
            if best.get(&next).is_some_and(|known| *known <= ap) {
                continue;
            }
            best.insert(next, ap);
            frontier.push(ReachabilityNode {
                cell: next,
                ap,
                steps,
                tags,
            });
        }
    }

    best.remove(&unit.position);
    best.retain(|cell, _| state.grid.contains(*cell) && !occupied.contains(cell));
    Ok(best)
}

fn movement_neighbors(cell: GridCell, program: &MoveProgram) -> Vec<GridCell> {
    if let Some(range) = program.teleport_range {
        return Hex::ZERO
            .range(range)
            .filter(|offset| *offset != Hex::ZERO)
            .map(|offset| {
                GridCell::new(
                    Hex::new(cell.hex.x + offset.x, cell.hex.y + offset.y),
                    cell.layer,
                )
            })
            .collect();
    }

    let mut neighbors = cell
        .hex
        .all_neighbors()
        .into_iter()
        .map(|hex| GridCell::new(hex, cell.layer))
        .collect::<Vec<_>>();
    for &delta in &program.vertical_deltas {
        if delta != 0 {
            neighbors.push(GridCell::new(cell.hex, cell.layer.saturating_add(delta)));
        }
    }
    neighbors
}

#[derive(Debug, Default, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TacticalDiff {
    pub agents: BTreeMap<AgentId, UnitState>,
    pub rng_update: Option<SeededRng>,
    pub reaction_queue: Vec<(AgentId, ReactionId, AgentId)>,
    pub reaction_queue_replace: Option<Vec<(AgentId, ReactionId, AgentId)>>,
    pub turn_completed: bool,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum TacticalDisplayAction {
    Move {
        to: GridCell,
    },
    Ability {
        target: AgentId,
        ability: AbilityId,
    },
    Reaction {
        reaction: ReactionId,
        target: AgentId,
    },
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
        self.diff
            .agents
            .get(&id)
            .or_else(|| self.initial_state.agents.get(&id))
    }
    fn list_agents(&self) -> Vec<AgentId> {
        let mut ids: BTreeSet<_> = self.initial_state.agents.keys().cloned().collect();
        ids.extend(self.diff.agents.keys().cloned());
        ids.into_iter().collect()
    }
}

impl TacticalAccess for StateDiffRefMut<'_, TacticalDomain> {
    fn get_agent(&self, id: AgentId) -> Option<&UnitState> {
        self.diff
            .agents
            .get(&id)
            .or_else(|| self.initial_state.agents.get(&id))
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
