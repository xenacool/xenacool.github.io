use crate::tags::{TagBag, TagId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::combat::CombatVector;
use crate::jobs::{DefinitionId, MovementId, PassiveId, ReactionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityId(pub DefinitionId);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
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
pub enum AbilityDelivery {
    Melee,
    StraightProjectile,
    ArcProjectile,
    SelfTarget,
    Area,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDef {
    pub id: AbilityId,
    pub name: String,
    pub ap_cost: u8,
    pub emit_tags: Vec<(TagId, u8)>,
    pub consume_tags: Vec<(TagId, u8, u8)>, // (tag, stacks, discount)
    pub scaling: HashMap<String, f32>,      // Attribute name -> scaling factor
    pub range: u8,
    pub delivery: AbilityDelivery,
    pub area_radius: u8,
}

impl AbilityDef {
    pub fn attack_scaling_vector(&self) -> CombatVector {
        CombatVector::from_scaling(&self.scaling)
    }
}

impl AbilityDef {
    pub fn get_ap_cost(&self, tag_bag: &mut TagBag) -> u8 {
        let mut discount = 0;
        for &(tag, stacks, d) in &self.consume_tags {
            if tag_bag.consume(tag, stacks) == stacks {
                discount += d;
            }
        }
        (self.ap_cost as i32 - discount as i32).max(0) as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveDef {
    pub id: PassiveId,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionDef {
    pub id: ReactionId,
    pub name: String,
    pub ap_cost: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveProgram {
    pub id: MovementId,
    pub name: String,
    pub steps_ap_cost: Vec<(u8, u8)>, // (step-threshold, AP cost)
    pub vertical_deltas: Vec<i32>,
    pub crosses_holes: bool,
    pub crosses_occupied: bool,
    pub teleport_range: Option<u32>,
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
