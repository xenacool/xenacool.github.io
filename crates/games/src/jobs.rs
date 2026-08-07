use std::collections::HashMap;
use std::hash::Hash;
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
pub struct MovementId(pub u64);

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

pub const JOB_CAVEMAN: JobId = JobId(1);
pub const JOB_MAGE: JobId = JobId(2);
pub const JOB_NECROMANCER: JobId = JobId(3);
pub const JOB_SKELETON_MINION: JobId = JobId(4);

pub fn get_job_defs() -> HashMap<JobId, JobDef> {
    let mut jobs = HashMap::new();

    jobs.insert(
        JOB_CAVEMAN,
        JobDef {
            id: JOB_CAVEMAN,
            base_stats: UnitStats {
                strength: 15,
                dexterity: 8,
                intelligence: 4,
                wisdom: 4,
                charisma: 6,
                constitution: 14,
                wits: 8,
                stamina: 12,
                armor_class: 12,
                speed: 4,
            },
            equipment_slots: vec![SlotType::MainHand, SlotType::Body, SlotType::Accessory],
            passive_slots_count: 1,
            reaction_slots_count: 1,
            secondary_job_slots_count: 1,
            abilities: vec![AbilityId(101), AbilityId(102), AbilityId(103)],
            passives: vec![PassiveId(101)],
            reactions: vec![ReactionId(101)],
            movement: MovementId(101),
        },
    );

    jobs.insert(
        JOB_MAGE,
        JobDef {
            id: JOB_MAGE,
            base_stats: UnitStats {
                strength: 4,
                dexterity: 6,
                intelligence: 16,
                wisdom: 12,
                charisma: 8,
                constitution: 6,
                wits: 12,
                stamina: 8,
                armor_class: 8,
                speed: 5,
            },
            equipment_slots: vec![SlotType::MainHand, SlotType::Body, SlotType::Accessory],
            passive_slots_count: 1,
            reaction_slots_count: 1,
            secondary_job_slots_count: 1,
            abilities: vec![AbilityId(201), AbilityId(202), AbilityId(203)],
            passives: vec![PassiveId(201)],
            reactions: vec![ReactionId(201)],
            movement: MovementId(201),
        },
    );

    jobs.insert(
        JOB_NECROMANCER,
        JobDef {
            id: JOB_NECROMANCER,
            base_stats: UnitStats {
                strength: 6,
                dexterity: 8,
                intelligence: 14,
                wisdom: 10,
                charisma: 12,
                constitution: 10,
                wits: 10,
                stamina: 10,
                armor_class: 10,
                speed: 4,
            },
            equipment_slots: vec![SlotType::MainHand, SlotType::Body, SlotType::Accessory],
            passive_slots_count: 1,
            reaction_slots_count: 1,
            secondary_job_slots_count: 1,
            abilities: vec![AbilityId(301), AbilityId(302), AbilityId(303)],
            passives: vec![PassiveId(301)],
            reactions: vec![ReactionId(301)],
            movement: MovementId(301),
        },
    );

    jobs.insert(
        JOB_SKELETON_MINION,
        JobDef {
            id: JOB_SKELETON_MINION,
            base_stats: UnitStats {
                strength: 10,
                dexterity: 12,
                intelligence: 2,
                wisdom: 2,
                charisma: 2,
                constitution: 8,
                wits: 8,
                stamina: 8,
                armor_class: 10,
                speed: 6,
            },
            equipment_slots: vec![SlotType::MainHand, SlotType::OffHand, SlotType::Body],
            passive_slots_count: 1,
            reaction_slots_count: 1,
            secondary_job_slots_count: 1,
            abilities: vec![AbilityId(401), AbilityId(402), AbilityId(403)],
            passives: vec![PassiveId(401)],
            reactions: vec![ReactionId(401)],
            movement: MovementId(401),
        },
    );

    jobs
}