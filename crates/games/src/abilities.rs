use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::tags::{TagId, TagBag};

use crate::jobs::{PassiveId, ReactionId, MovementId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityId(pub u64);

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
pub struct AbilityDef {
    pub id: AbilityId,
    pub name: String,
    pub ap_cost: u8,
    pub emit_tags: Vec<(TagId, u8)>,
    pub consume_tags: Vec<(TagId, u8, u8)>, // (tag, stacks, discount)
    pub scaling: HashMap<String, f32>,       // Attribute name -> scaling factor
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

pub fn get_ability_defs() -> HashMap<AbilityId, AbilityDef> {
    let mut abilities = HashMap::new();
    
    // Caveman
    abilities.insert(AbilityId(101), AbilityDef {
        id: AbilityId(101),
        name: "Club Smash".to_string(),
        ap_cost: 2,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: [("STR".to_string(), 1.5)].into(),
    });
    abilities.insert(AbilityId(102), AbilityDef {
        id: AbilityId(102),
        name: "Rock Throw".to_string(),
        ap_cost: 2,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: [("STR".to_string(), 1.0), ("DEX".to_string(), 0.5)].into(),
    });
    abilities.insert(AbilityId(103), AbilityDef {
        id: AbilityId(103),
        name: "Primal Roar".to_string(),
        ap_cost: 3,
        emit_tags: vec![(TagId(1), 1)], // Stun or something
        consume_tags: vec![],
        scaling: [("CHA".to_string(), 0.5)].into(),
    });

    // Mage
    abilities.insert(AbilityId(201), AbilityDef {
        id: AbilityId(201),
        name: "Fireball".to_string(),
        ap_cost: 3,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: [("INT".to_string(), 2.0)].into(),
    });
    abilities.insert(AbilityId(202), AbilityDef {
        id: AbilityId(202),
        name: "Frost Bolt".to_string(),
        ap_cost: 2,
        emit_tags: vec![(TagId(2), 1)], // Slow
        consume_tags: vec![],
        scaling: [("INT".to_string(), 1.0), ("WIS".to_string(), 0.5)].into(),
    });
    abilities.insert(AbilityId(203), AbilityDef {
        id: AbilityId(203),
        name: "Arcane Shield".to_string(),
        ap_cost: 2,
        emit_tags: vec![(TagId(3), 2)], // Shield
        consume_tags: vec![],
        scaling: [("INT".to_string(), 0.5), ("WIS".to_string(), 1.0)].into(),
    });

    // Necromancer
    abilities.insert(AbilityId(301), AbilityDef {
        id: AbilityId(301),
        name: "Raise Skeleton".to_string(),
        ap_cost: 4,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: [("INT".to_string(), 0.5)].into(),
    });
    abilities.insert(AbilityId(302), AbilityDef {
        id: AbilityId(302),
        name: "Soul Drain".to_string(),
        ap_cost: 3,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: [("INT".to_string(), 1.0), ("CHA".to_string(), 1.0)].into(),
    });
    abilities.insert(AbilityId(303), AbilityDef {
        id: AbilityId(303),
        name: "Bone Armor".to_string(),
        ap_cost: 2,
        emit_tags: vec![(TagId(4), 2)], // Armor
        consume_tags: vec![],
        scaling: [("INT".to_string(), 1.0)].into(),
    });

    // Skeleton Minion
    abilities.insert(AbilityId(401), AbilityDef {
        id: AbilityId(401),
        name: "Bony Strike".to_string(),
        ap_cost: 1,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: [("STR".to_string(), 1.0)].into(),
    });
    abilities.insert(AbilityId(402), AbilityDef {
        id: AbilityId(402),
        name: "Shield Bash".to_string(),
        ap_cost: 2,
        emit_tags: vec![(TagId(1), 1)], // Stun
        consume_tags: vec![],
        scaling: [("STR".to_string(), 0.5), ("CON".to_string(), 0.5)].into(),
    });
    abilities.insert(AbilityId(403), AbilityDef {
        id: AbilityId(403),
        name: "Screech".to_string(),
        ap_cost: 2,
        emit_tags: vec![(TagId(5), 1)], // Fear/Debuff
        consume_tags: vec![],
        scaling: [("CHA".to_string(), 0.2)].into(),
    });

    abilities
}

pub fn get_passive_defs() -> HashMap<PassiveId, PassiveDef> {
    let mut passives = HashMap::new();
    passives.insert(PassiveId(101), PassiveDef { id: PassiveId(101), name: "Thick Skin".to_string() });
    passives.insert(PassiveId(201), PassiveDef { id: PassiveId(201), name: "Spell Echo".to_string() });
    passives.insert(PassiveId(301), PassiveDef { id: PassiveId(301), name: "Death's Embrace".to_string() });
    passives.insert(PassiveId(401), PassiveDef { id: PassiveId(401), name: "Undead Resilience".to_string() });
    passives
}

pub fn get_reaction_defs() -> HashMap<ReactionId, ReactionDef> {
    let mut reactions = HashMap::new();
    reactions.insert(ReactionId(101), ReactionDef { id: ReactionId(101), name: "Counter-Swing".to_string(), ap_cost: 1 });
    reactions.insert(ReactionId(201), ReactionDef { id: ReactionId(201), name: "Mana Shield".to_string(), ap_cost: 0 });
    reactions.insert(ReactionId(301), ReactionDef { id: ReactionId(301), name: "Vengeful Spirit".to_string(), ap_cost: 1 });
    reactions.insert(ReactionId(401), ReactionDef { id: ReactionId(401), name: "Bone Splinter".to_string(), ap_cost: 0 });
    reactions
}

pub fn get_movement_defs() -> HashMap<MovementId, MoveProgram> {
    let mut movements = HashMap::new();
    
    // Caveman: Plain Move
    movements.insert(MovementId(101), MoveProgram {
        id: MovementId(101),
        name: "Plain Move".to_string(),
        steps_ap_cost: vec![(1, 1)],
        emit_tags: vec![],
        consume_tags: vec![],
    });

    // Mage: Manafeet
    movements.insert(MovementId(201), MoveProgram {
        id: MovementId(201),
        name: "Manafeet".to_string(),
        steps_ap_cost: vec![(1, 1)],
        emit_tags: vec![(TagId(10), 1)], // Special tag for Manafeet hook
        consume_tags: vec![],
    });

    // Necromancer: Shadow Step
    movements.insert(MovementId(301), MoveProgram {
        id: MovementId(301),
        name: "Shadow Step".to_string(),
        steps_ap_cost: vec![(1, 2)], // More expensive teleport
        emit_tags: vec![],
        consume_tags: vec![],
    });

    // Skeleton Minion: Rattle Dash
    movements.insert(MovementId(401), MoveProgram {
        id: MovementId(401),
        name: "Rattle Dash".to_string(),
        steps_ap_cost: vec![(1, 1), (4, 2)], // Long range, gets more expensive
        emit_tags: vec![],
        consume_tags: vec![],
    });

    movements
}
