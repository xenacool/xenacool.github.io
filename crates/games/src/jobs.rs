use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::abilities::AbilityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorClassId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PassiveId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Nonbinary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentSlots {
    pub slots: HashMap<SlotType, Option<ItemId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDef {
    pub id: JobId,
    pub equipment_slots: Vec<SlotType>,
    pub passive_slots_count: u8,
    pub reaction_slots_count: u8,
    pub secondary_job_slots_count: u8,
    pub innate_abilities: Vec<AbilityId>,
    pub innate_passives: Vec<PassiveId>,
    pub innate_reactions: Vec<ReactionId>,
}
