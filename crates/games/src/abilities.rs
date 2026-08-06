use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::tags::{TagId, TagBag};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityId(pub u64);

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
