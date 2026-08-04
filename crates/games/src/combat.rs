use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivedStat {
    ArmorClass,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierStacking {
    AdditiveIndependent,
    RefreshReplace,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedModifier {
    pub stat: DerivedStat,
    pub amount: i32,
    pub remaining_turns: u16,
    pub stacking: ModifierStacking,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct CombatVector {
    pub strength: f32,
    pub dexterity: f32,
    pub intelligence: f32,
    pub wisdom: f32,
    pub charisma: f32,
    pub constitution: f32,
    pub wits: f32,
    pub stamina: f32,
}

impl CombatVector {
    pub fn from_stats(stats: &crate::UnitStats) -> Self {
        Self {
            strength: stats.strength as f32,
            dexterity: stats.dexterity as f32,
            intelligence: stats.intelligence as f32,
            wisdom: stats.wisdom as f32,
            charisma: stats.charisma as f32,
            constitution: stats.constitution as f32,
            wits: stats.wits as f32,
            stamina: stats.stamina as f32,
        }
    }

    pub fn from_scaling(scaling: &HashMap<String, f32>) -> Self {
        let mut vector = Self::default();
        for (name, value) in scaling {
            match name.as_str() {
                "STR" => vector.strength = *value,
                "DEX" => vector.dexterity = *value,
                "INT" => vector.intelligence = *value,
                "WIS" => vector.wisdom = *value,
                "CHA" => vector.charisma = *value,
                "CON" => vector.constitution = *value,
                "WITS" => vector.wits = *value,
                "STA" => vector.stamina = *value,
                _ => {}
            }
        }
        vector
    }

    pub fn component_mul(self, other: Self) -> Self {
        Self {
            strength: self.strength * other.strength,
            dexterity: self.dexterity * other.dexterity,
            intelligence: self.intelligence * other.intelligence,
            wisdom: self.wisdom * other.wisdom,
            charisma: self.charisma * other.charisma,
            constitution: self.constitution * other.constitution,
            wits: self.wits * other.wits,
            stamina: self.stamina * other.stamina,
        }
    }

    pub fn component_sub_clamped(self, other: Self) -> Self {
        Self {
            strength: (self.strength - other.strength).max(0.0),
            dexterity: (self.dexterity - other.dexterity).max(0.0),
            intelligence: (self.intelligence - other.intelligence).max(0.0),
            wisdom: (self.wisdom - other.wisdom).max(0.0),
            charisma: (self.charisma - other.charisma).max(0.0),
            constitution: (self.constitution - other.constitution).max(0.0),
            wits: (self.wits - other.wits).max(0.0),
            stamina: (self.stamina - other.stamina).max(0.0),
        }
    }

    pub fn scale(self, factor: f32) -> Self {
        Self {
            strength: self.strength * factor,
            dexterity: self.dexterity * factor,
            intelligence: self.intelligence * factor,
            wisdom: self.wisdom * factor,
            charisma: self.charisma * factor,
            constitution: self.constitution * factor,
            wits: self.wits * factor,
            stamina: self.stamina * factor,
        }
    }

    pub fn sum(self) -> f32 {
        self.strength
            + self.dexterity
            + self.intelligence
            + self.wisdom
            + self.charisma
            + self.constitution
            + self.wits
            + self.stamina
    }
}
