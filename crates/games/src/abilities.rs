use crate::FacingRelation;
use crate::effects::RPGPrograms;
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AbilityTargetRule {
    EnemyUnit,
    SelfUnit,
    AreaCell,
    EmptyCell,
    OwnedSummon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDef {
    pub id: AbilityId,
    pub name: String,
    pub ap_cost: u8,
    pub health_cost: u16,
    pub mana_cost: u16,
    pub health_floor: i32,
    pub emit_tags: Vec<(TagId, u8)>,
    pub consume_tags: Vec<(TagId, u8, u8)>, // (tag, stacks, discount)
    pub scaling: HashMap<String, f32>,      // Attribute name -> scaling factor
    pub range: u8,
    pub delivery: AbilityDelivery,
    pub target_rule: AbilityTargetRule,
    pub projectile_profile: Option<String>,
    pub area_radius: u8,
    pub programs: RPGPrograms,
    pub facing_rule: Option<FacingRelation>,
    pub front_damage_multiplier: (u16, u16),
    pub side_damage_multiplier: (u16, u16),
    pub rear_damage_multiplier: (u16, u16),
}

impl AbilityDef {
    pub fn facing_damage_multiplier(&self, relation: FacingRelation) -> (u16, u16) {
        match relation {
            FacingRelation::Front => self.front_damage_multiplier,
            FacingRelation::Side => self.side_damage_multiplier,
            FacingRelation::Rear => self.rear_damage_multiplier,
        }
    }
}

impl AbilityDef {
    pub fn attack_scaling_vector(&self) -> CombatVector {
        CombatVector::from_scaling(&self.scaling)
    }
}

impl AbilityDef {
    pub fn resolve_cost(&self, tag_bag: &TagBag) -> ResolvedAbilityCost {
        let mut available = tag_bag.clone();
        let mut discount = 0;
        let mut consumed_tags = Vec::new();
        for &(tag, stacks, d) in &self.consume_tags {
            if available.count(tag) >= stacks {
                available.consume(tag, stacks);
                discount += d;
                consumed_tags.push((tag, stacks));
            }
        }
        ResolvedAbilityCost {
            base_ap: self.ap_cost,
            ap: (self.ap_cost as i32 - discount as i32).max(0) as u8,
            health: self.health_cost,
            mana: self.mana_cost,
            health_floor: self.health_floor,
            consumed_tags,
        }
    }

    pub fn get_ap_cost(&self, tag_bag: &mut TagBag) -> u8 {
        let resolved = self.resolve_cost(tag_bag);
        for &(tag, stacks) in &resolved.consumed_tags {
            tag_bag.consume(tag, stacks);
        }
        resolved.ap
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ResolvedAbilityCost {
    pub base_ap: u8,
    pub ap: u8,
    pub health: u16,
    pub mana: u16,
    pub health_floor: i32,
    pub consumed_tags: Vec<(TagId, u8)>,
}

impl ResolvedAbilityCost {
    pub fn can_pay(&self, unit: &crate::UnitState) -> bool {
        unit.action_points >= i32::from(self.ap)
            && unit.mana >= i32::from(self.mana)
            && i64::from(unit.health) - i64::from(self.health) >= i64::from(self.health_floor)
            && self
                .consumed_tags
                .iter()
                .all(|(tag, stacks)| unit.turn_tags.count(*tag) >= *stacks)
    }

    pub fn apply_to(&self, unit: &mut crate::UnitState) -> Result<(), String> {
        if !self.can_pay(unit) {
            return Err("Insufficient resources for ability cost".to_string());
        }
        let mut updated = unit.clone();
        updated.action_points -= i32::from(self.ap);
        updated.health -= i32::from(self.health);
        updated.mana -= i32::from(self.mana);
        for &(tag, stacks) in &self.consumed_tags {
            if updated.turn_tags.consume(tag, stacks) != stacks {
                return Err("Ability discount tag changed before commit".to_string());
            }
        }
        *unit = updated;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveDef {
    pub id: PassiveId,
    pub name: String,
    pub stat_modifiers: Vec<(crate::DerivedStat, i32)>,
    pub damage_multiplier: (u16, u16),
    pub on_kill_heal: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionDef {
    pub id: ReactionId,
    pub name: String,
    pub ap_cost: u8,
    pub mana_cost: u16,
    pub target_damage: i32,
    pub self_heal: i32,
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
    pub mana_gain: i32,
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

#[cfg(test)]
mod cost_tests {
    use crate::{AgentId, GridCell, SkirmishConfig};

    #[test]
    fn resolved_cost_is_atomic_and_consumes_discount_once() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario.set_unit_mana(1, 0).unwrap();
        let state = scenario.build_state().unwrap();
        let drain = state
            .ability_registry
            .values()
            .find(|ability| ability.name == "Soul Drain")
            .unwrap();
        let fresh_soul = drain.consume_tags[0].0;
        let mut unit = state.agents[&AgentId(1)].clone();
        unit.mana = 10;
        unit.turn_tags.counts.insert(fresh_soul, 1);

        let discounted = drain.resolve_cost(&unit.turn_tags);
        assert_eq!(
            (discounted.base_ap, discounted.ap, discounted.mana),
            (3, 2, 10)
        );
        discounted.apply_to(&mut unit).unwrap();
        assert_eq!((unit.action_points, unit.mana), (2, 0));
        assert_eq!(unit.turn_tags.count(fresh_soul), 0);

        let second = drain.resolve_cost(&unit.turn_tags);
        assert_eq!(second.ap, 3);
        let snapshot = unit.clone();
        assert!(second.apply_to(&mut unit).is_err());
        assert_eq!(unit, snapshot);
    }

    #[test]
    fn initial_mana_override_is_checked_against_derived_maximum() {
        let mut valid = SkirmishConfig::new(42);
        valid
            .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        valid.set_unit_mana(1, 0).unwrap();
        assert_eq!(valid.build_state().unwrap().agents[&AgentId(1)].mana, 0);

        let mut invalid = valid.clone();
        invalid.set_unit_mana(1, 71).unwrap();
        assert!(
            invalid
                .build_state()
                .unwrap_err()
                .contains("exceeds maximum")
        );
    }
}
