use crate::abilities::AbilityId;
use crate::abilities::{AbilityDef, MoveProgram, PassiveDef, ReactionDef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// The single backing type for every generated gameplay-definition ID.
pub type DefinitionId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionIdAllocator {
    next: DefinitionId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptJobDef {
    pub name: String,
    pub base_stats: Option<UnitStats>,
    pub equipment_slots: Vec<SlotType>,
    pub passive_slots_count: Option<u8>,
    pub reaction_slots_count: Option<u8>,
    pub secondary_job_slots_count: Option<u8>,
    pub ability_names: Vec<String>,
    pub passive_names: Vec<String>,
    pub reaction_names: Vec<String>,
    pub movement_name: Option<String>,
}

impl ScriptJobDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_stats: None,
            equipment_slots: Vec::new(),
            passive_slots_count: None,
            reaction_slots_count: None,
            secondary_job_slots_count: None,
            ability_names: Vec::new(),
            passive_names: Vec::new(),
            reaction_names: Vec::new(),
            movement_name: None,
        }
    }

    pub fn resolve(
        &self,
        id: JobId,
        abilities: &HashMap<AbilityId, AbilityDef>,
        passives: &HashMap<PassiveId, PassiveDef>,
        reactions: &HashMap<ReactionId, ReactionDef>,
        movements: &HashMap<MovementId, MoveProgram>,
    ) -> Result<JobDef, String> {
        if self.name.trim().is_empty() {
            return Err("Script job name must not be empty".to_string());
        }
        let base_stats = self
            .base_stats
            .clone()
            .ok_or_else(|| format!("Job {} is missing base stats", self.name))?;
        let passive_slots_count = self
            .passive_slots_count
            .ok_or_else(|| format!("Job {} is missing passive slot count", self.name))?;
        let reaction_slots_count = self
            .reaction_slots_count
            .ok_or_else(|| format!("Job {} is missing reaction slot count", self.name))?;
        let secondary_job_slots_count = self
            .secondary_job_slots_count
            .ok_or_else(|| format!("Job {} is missing secondary slot count", self.name))?;

        let abilities = self
            .ability_names
            .iter()
            .map(|name| find_named(abilities, name, "ability"))
            .collect::<Result<Vec<_>, _>>()?;
        let passives = self
            .passive_names
            .iter()
            .map(|name| find_named(passives, name, "passive"))
            .collect::<Result<Vec<_>, _>>()?;
        let reactions = self
            .reaction_names
            .iter()
            .map(|name| find_named(reactions, name, "reaction"))
            .collect::<Result<Vec<_>, _>>()?;
        let movement_name = self
            .movement_name
            .as_ref()
            .ok_or_else(|| format!("Job {} is missing movement", self.name))?;
        let movement = movements
            .iter()
            .find(|(_, definition)| definition.name == *movement_name)
            .map(|(id, _)| *id)
            .ok_or_else(|| format!("Unknown movement {movement_name} for job {}", self.name))?;

        Ok(JobDef {
            id,
            name: self.name.clone(),
            base_stats,
            equipment_slots: self.equipment_slots.clone(),
            passive_slots_count,
            reaction_slots_count,
            secondary_job_slots_count,
            abilities,
            passives,
            reactions,
            movement,
        })
    }
}

fn find_named<K: Copy, T: AsRefName>(
    definitions: &HashMap<K, T>,
    name: &str,
    kind: &str,
) -> Result<K, String>
where
    T: AsRefName,
{
    definitions
        .iter()
        .find(|(_, definition)| definition.name_ref() == name)
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("Unknown {kind} {name}"))
}

trait AsRefName {
    fn name_ref(&self) -> &str;
}

impl AsRefName for AbilityDef {
    fn name_ref(&self) -> &str {
        &self.name
    }
}
impl AsRefName for PassiveDef {
    fn name_ref(&self) -> &str {
        &self.name
    }
}
impl AsRefName for ReactionDef {
    fn name_ref(&self) -> &str {
        &self.name
    }
}

impl DefinitionIdAllocator {
    pub const fn new(first: DefinitionId) -> Self {
        Self { next: first }
    }

    pub fn allocate(&mut self) -> Result<DefinitionId, String> {
        let id = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| "Definition ID allocator exhausted".to_string())?;
        Ok(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorClassId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub DefinitionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PassiveId(pub DefinitionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionId(pub DefinitionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MovementId(pub DefinitionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Gender {
    Male,
    Female,
    Nonbinary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UnitStats {
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
pub struct DerivedStats {
    pub health_max: i32,
    pub mana_max: i32,
    pub action_points_max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SlotType {
    MainHand,
    OffHand,
    Head,
    Body,
    Accessory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquipmentSlots {
    pub slots: HashMap<SlotType, Option<ItemId>>,
}

impl Hash for EquipmentSlots {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut keys: Vec<_> = self.slots.keys().collect();
        keys.sort_by_key(|&k| format!("{:?}", k));
        for k in keys {
            k.hash(state);
            self.slots.get(k).hash(state);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDef {
    pub id: JobId,
    pub name: String,
    pub base_stats: UnitStats,
    pub equipment_slots: Vec<SlotType>,
    pub passive_slots_count: u8,
    pub reaction_slots_count: u8,
    pub secondary_job_slots_count: u8,
    pub abilities: Vec<AbilityId>,
    pub passives: Vec<PassiveId>,
    pub reactions: Vec<ReactionId>,
    pub movement: MovementId,
}
