use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    AbilityDef, AbilityDelivery, AbilityId, AbilityTargetRule, DefinitionIdAllocator, DerivedStat,
    FacingRelation, JobDef, JobId, MoveProgram, MovementId, PassiveDef, PassiveId, RPGPrograms,
    ReactionDef, ReactionId, ScriptJobDef, TagDef, TagId, validate_rpg_programs,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptPassiveDef {
    pub name: String,
    pub stat_modifiers: Vec<(DerivedStat, i32)>,
    pub damage_multiplier: Option<(u16, u16)>,
    pub on_kill_heal: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptTagDef {
    pub name: String,
    pub max_stacks: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptReactionDef {
    pub name: String,
    pub ap_cost: Option<u8>,
    pub mana_cost: u16,
    pub target_damage: i32,
    pub self_heal: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptMovementDef {
    pub name: String,
    pub steps_ap_cost: Vec<(u8, u8)>,
    pub vertical_deltas: Vec<i32>,
    pub crosses_holes: bool,
    pub crosses_occupied: bool,
    pub teleport_range: Option<u32>,
    pub emit_tags: Vec<(String, u8)>,
    pub consume_tags: Vec<(String, u8, u8)>,
    pub mana_gain: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptAbilityDef {
    pub name: String,
    pub ap_cost: Option<u8>,
    pub health_cost: Option<u16>,
    pub mana_cost: Option<u16>,
    pub health_floor: Option<i32>,
    pub range: Option<u8>,
    pub delivery: Option<String>,
    pub target_rule: Option<String>,
    pub projectile_profile: Option<String>,
    pub area_radius: Option<u8>,
    pub scaling: Vec<(String, f32)>,
    pub emit_tags: Vec<(String, u8)>,
    pub consume_tags: Vec<(String, u8, u8)>,
    pub programs: RPGPrograms,
    pub facing_rule: Option<String>,
    pub front_damage_multiplier: Option<(u16, u16)>,
    pub side_damage_multiplier: Option<(u16, u16)>,
    pub rear_damage_multiplier: Option<(u16, u16)>,
}

impl ScriptAbilityDef {
    pub fn resolve(
        &self,
        id: AbilityId,
        tags: &HashMap<String, TagId>,
    ) -> Result<AbilityDef, String> {
        let ap_cost = self
            .ap_cost
            .ok_or_else(|| format!("Ability {} is missing AP cost", self.name))?;
        if self.health_floor.unwrap_or(0) < 0 {
            return Err(format!("Ability {} has a negative health floor", self.name));
        }
        if self
            .consume_tags
            .iter()
            .any(|(_, stacks, discount)| *stacks == 0 || *discount > ap_cost)
        {
            return Err(format!(
                "Ability {} has an invalid tag consumption or AP discount",
                self.name
            ));
        }
        for (label, multiplier) in [
            ("front", self.front_damage_multiplier),
            ("side", self.side_damage_multiplier),
            ("rear", self.rear_damage_multiplier),
        ]
        .into_iter()
        .filter_map(|(label, multiplier)| multiplier.map(|value| (label, value)))
        {
            if multiplier.1 == 0 {
                return Err(format!(
                    "Ability {} has a zero denominator in its {label} damage multiplier",
                    self.name
                ));
            }
        }
        let delivery = match self
            .delivery
            .as_deref()
            .ok_or_else(|| format!("Ability {} is missing delivery", self.name))?
        {
            "Melee" => AbilityDelivery::Melee,
            "StraightProjectile" => AbilityDelivery::StraightProjectile,
            "ArcProjectile" => AbilityDelivery::ArcProjectile,
            "SelfTarget" => AbilityDelivery::SelfTarget,
            "Area" => AbilityDelivery::Area,
            value => {
                return Err(format!(
                    "Unknown delivery mode {value} for ability {}",
                    self.name
                ));
            }
        };
        let area_radius =
            self.area_radius
                .unwrap_or(if matches!(&delivery, AbilityDelivery::Area) {
                    1
                } else {
                    0
                });
        let target_rule = match self.target_rule.as_deref() {
            Some("EnemyUnit") => AbilityTargetRule::EnemyUnit,
            Some("SelfUnit") => AbilityTargetRule::SelfUnit,
            Some("AreaCell") => AbilityTargetRule::AreaCell,
            Some("EmptyCell") => AbilityTargetRule::EmptyCell,
            Some("OwnedSummon") => AbilityTargetRule::OwnedSummon,
            Some(value) => {
                return Err(format!(
                    "Unknown target rule {value} for ability {}",
                    self.name
                ));
            }
            None if matches!(delivery, AbilityDelivery::SelfTarget) => AbilityTargetRule::SelfUnit,
            None if matches!(delivery, AbilityDelivery::Area) => AbilityTargetRule::AreaCell,
            None => AbilityTargetRule::EnemyUnit,
        };
        let facing_rule = match self.facing_rule.as_deref() {
            None | Some("Any") => None,
            Some("Front") => Some(FacingRelation::Front),
            Some("Side") => Some(FacingRelation::Side),
            Some("Rear") => Some(FacingRelation::Rear),
            Some(value) => {
                return Err(format!(
                    "Unknown facing rule {value} for ability {}",
                    self.name
                ));
            }
        };
        Ok(AbilityDef {
            id,
            name: self.name.clone(),
            ap_cost,
            health_cost: self.health_cost.unwrap_or(0),
            mana_cost: self.mana_cost.unwrap_or(0),
            health_floor: self.health_floor.unwrap_or(0),
            range: self
                .range
                .ok_or_else(|| format!("Ability {} is missing range", self.name))?,
            delivery,
            target_rule,
            projectile_profile: self.projectile_profile.clone(),
            area_radius,
            scaling: self.scaling.iter().cloned().collect(),
            emit_tags: self
                .emit_tags
                .iter()
                .map(|(name, stacks)| {
                    Ok((
                        *tags
                            .get(name)
                            .ok_or_else(|| format!("Unknown tag: {name}"))?,
                        *stacks,
                    ))
                })
                .collect::<Result<_, String>>()?,
            consume_tags: self
                .consume_tags
                .iter()
                .map(|(name, stacks, discount)| {
                    Ok((
                        *tags
                            .get(name)
                            .ok_or_else(|| format!("Unknown tag: {name}"))?,
                        *stacks,
                        *discount,
                    ))
                })
                .collect::<Result<_, String>>()?,
            programs: {
                validate_rpg_programs(&self.programs)
                    .map_err(|error| format!("Ability {}: {error}", self.name))?;
                self.programs.clone()
            },
            facing_rule,
            front_damage_multiplier: self.front_damage_multiplier.unwrap_or((1, 1)),
            side_damage_multiplier: self.side_damage_multiplier.unwrap_or((1, 1)),
            rear_damage_multiplier: self.rear_damage_multiplier.unwrap_or((1, 1)),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Ruleset {
    pub jobs: HashMap<JobId, JobDef>,
    pub abilities: HashMap<AbilityId, AbilityDef>,
    pub passives: HashMap<PassiveId, PassiveDef>,
    pub reactions: HashMap<ReactionId, ReactionDef>,
    pub movements: HashMap<crate::MovementId, MoveProgram>,
    pub tags: HashMap<TagId, TagDef>,
    pub tag_names: HashMap<String, TagId>,
}

impl Ruleset {
    pub fn with_global_script_ids(
        tags: &[ScriptTagDef],
        abilities: &[ScriptAbilityDef],
        passives: &[ScriptPassiveDef],
        reactions: &[ScriptReactionDef],
        movements: &[ScriptMovementDef],
        jobs: &[ScriptJobDef],
    ) -> Result<Self, String> {
        let mut allocator = DefinitionIdAllocator::new(0);
        let mut tag_names = HashMap::new();
        for definition in tags {
            if tag_names.contains_key(&definition.name) {
                return Err(format!("Duplicate script tag: {}", definition.name));
            }
            let id = TagId(allocator.allocate()?);
            let max_stacks = definition
                .max_stacks
                .ok_or_else(|| format!("Tag {} is missing max stacks", definition.name))?;
            if max_stacks == 0 {
                return Err(format!(
                    "Tag {} must allow at least one stack",
                    definition.name
                ));
            }
            tag_names.insert(definition.name.clone(), id);
        }
        let tags = tags
            .iter()
            .map(|definition| {
                let id = tag_names[&definition.name];
                (
                    id,
                    TagDef {
                        id,
                        max_stacks: definition.max_stacks.unwrap(),
                    },
                )
            })
            .collect();

        let mut ability_map = HashMap::new();
        for definition in abilities {
            let id = AbilityId(allocator.allocate()?);
            ability_map.insert(id, definition.resolve(id, &tag_names)?);
        }
        let mut passive_map = HashMap::new();
        for definition in passives {
            if definition.name.trim().is_empty() {
                return Err("Script passive name must not be empty".to_string());
            }
            if definition
                .damage_multiplier
                .is_some_and(|(numerator, denominator)| numerator == 0 || denominator == 0)
            {
                return Err(format!(
                    "Passive {} has an invalid damage multiplier",
                    definition.name
                ));
            }
            if definition.on_kill_heal < 0 {
                return Err(format!(
                    "Passive {} on-kill healing must not be negative",
                    definition.name
                ));
            }
            let id = PassiveId(allocator.allocate()?);
            passive_map.insert(
                id,
                PassiveDef {
                    id,
                    name: definition.name.clone(),
                    stat_modifiers: definition.stat_modifiers.clone(),
                    damage_multiplier: definition.damage_multiplier.unwrap_or((1, 1)),
                    on_kill_heal: definition.on_kill_heal,
                },
            );
        }
        let mut reaction_map = HashMap::new();
        for definition in reactions {
            let id = ReactionId(allocator.allocate()?);
            let ap_cost = definition
                .ap_cost
                .ok_or_else(|| format!("Reaction {} is missing AP cost", definition.name))?;
            if definition.target_damage < 0 || definition.self_heal < 0 {
                return Err(format!(
                    "Reaction {} damage and healing must not be negative",
                    definition.name
                ));
            }
            reaction_map.insert(
                id,
                ReactionDef {
                    id,
                    name: definition.name.clone(),
                    ap_cost,
                    mana_cost: definition.mana_cost,
                    target_damage: definition.target_damage,
                    self_heal: definition.self_heal,
                },
            );
        }
        let mut movement_map = HashMap::new();
        for definition in movements {
            if definition.mana_gain < 0 {
                return Err(format!(
                    "Movement {} mana gain must not be negative",
                    definition.name
                ));
            }
            let id = MovementId(allocator.allocate()?);
            let resolve_tags = |values: &[(String, u8, u8)]| {
                values
                    .iter()
                    .map(|(tag, stacks, discount)| {
                        Ok((
                            *tag_names
                                .get(tag)
                                .ok_or_else(|| format!("Unknown tag: {tag}"))?,
                            *stacks,
                            *discount,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()
            };
            let emit_tags = definition
                .emit_tags
                .iter()
                .map(|(tag, stacks)| {
                    Ok((
                        *tag_names
                            .get(tag)
                            .ok_or_else(|| format!("Unknown tag: {tag}"))?,
                        *stacks,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            movement_map.insert(
                id,
                MoveProgram {
                    id,
                    name: definition.name.clone(),
                    steps_ap_cost: definition.steps_ap_cost.clone(),
                    vertical_deltas: definition.vertical_deltas.clone(),
                    crosses_holes: definition.crosses_holes,
                    crosses_occupied: definition.crosses_occupied,
                    teleport_range: definition.teleport_range,
                    emit_tags,
                    consume_tags: resolve_tags(&definition.consume_tags)?,
                    mana_gain: definition.mana_gain,
                },
            );
        }
        let mut job_map = HashMap::new();
        for definition in jobs {
            let id = JobId(allocator.allocate()?);
            job_map.insert(
                id,
                definition.resolve(id, &ability_map, &passive_map, &reaction_map, &movement_map)?,
            );
        }
        for ability in ability_map.values() {
            let projectile_delivery = matches!(
                ability.delivery,
                AbilityDelivery::StraightProjectile | AbilityDelivery::ArcProjectile
            );
            if projectile_delivery != ability.projectile_profile.is_some() {
                return Err(format!(
                    "Ability {} projectile delivery/profile must be specified together",
                    ability.name
                ));
            }
            if ability
                .projectile_profile
                .as_ref()
                .is_some_and(|profile| profile.trim().is_empty())
            {
                return Err(format!(
                    "Ability {} has an empty projectile profile",
                    ability.name
                ));
            }
            let ops = ability
                .programs
                .get(&crate::RPGHook::OnAbilityResolve)
                .and_then(|programs| programs.first())
                .map(|program| program.ops.as_slice())
                .unwrap_or_default();
            let spawn_count = ops
                .iter()
                .filter(|op| matches!(op, crate::RPGBytecode::SpawnOwnedUnit { .. }))
                .count();
            if matches!(ability.target_rule, AbilityTargetRule::EmptyCell) && spawn_count != 1 {
                return Err(format!(
                    "Empty-cell ability {} must contain exactly one spawn operation",
                    ability.name
                ));
            }
            if !matches!(ability.target_rule, AbilityTargetRule::EmptyCell) && spawn_count != 0 {
                return Err(format!(
                    "Only empty-cell abilities may spawn units: {}",
                    ability.name
                ));
            }
            if matches!(ability.target_rule, AbilityTargetRule::OwnedSummon)
                && !ops
                    .iter()
                    .any(|op| matches!(op, crate::RPGBytecode::DestroyOwnedSummon))
            {
                return Err(format!(
                    "Owned-summon ability {} must destroy its target",
                    ability.name
                ));
            }
            for op in ops {
                if let crate::RPGBytecode::SpawnOwnedUnit { job, .. } = op
                    && !job_map.values().any(|definition| definition.name == *job)
                {
                    return Err(format!(
                        "Ability {} references unknown summoned job {job}",
                        ability.name
                    ));
                }
            }
        }
        Ok(Self {
            jobs: job_map,
            abilities: ability_map,
            passives: passive_map,
            reactions: reaction_map,
            movements: movement_map,
            tags,
            tag_names,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement(name: &str, mana_gain: i32) -> ScriptMovementDef {
        ScriptMovementDef {
            name: name.into(),
            steps_ap_cost: vec![(1, 1)],
            vertical_deltas: vec![],
            crosses_holes: false,
            crosses_occupied: false,
            teleport_range: None,
            emit_tags: vec![],
            consume_tags: vec![],
            mana_gain,
        }
    }

    #[test]
    fn generic_effect_records_preserve_arbitrary_authored_names_and_values() {
        let passives = [ScriptPassiveDef {
            name: "modded-passive".into(),
            stat_modifiers: vec![(DerivedStat::Constitution, 7)],
            damage_multiplier: Some((3, 2)),
            on_kill_heal: 11,
        }];
        let reactions = [ScriptReactionDef {
            name: "modded-reaction".into(),
            ap_cost: Some(2),
            mana_cost: 13,
            target_damage: 17,
            self_heal: 19,
        }];
        let movements = [movement("modded-movement", 23)];
        let ruleset =
            Ruleset::with_global_script_ids(&[], &[], &passives, &reactions, &movements, &[])
                .unwrap();

        let passive = ruleset.passives.values().next().unwrap();
        assert_eq!(
            (
                passive.name.as_str(),
                passive.stat_modifiers.as_slice(),
                passive.damage_multiplier,
                passive.on_kill_heal,
            ),
            (
                "modded-passive",
                [(DerivedStat::Constitution, 7)].as_slice(),
                (3, 2),
                11,
            )
        );
        let reaction = ruleset.reactions.values().next().unwrap();
        assert_eq!(
            (
                reaction.name.as_str(),
                reaction.ap_cost,
                reaction.mana_cost,
                reaction.target_damage,
                reaction.self_heal,
            ),
            ("modded-reaction", 2, 13, 17, 19)
        );
        let movement = ruleset.movements.values().next().unwrap();
        assert_eq!(
            (movement.name.as_str(), movement.mana_gain),
            ("modded-movement", 23)
        );
    }

    #[test]
    fn generic_effect_records_reject_negative_effect_amounts() {
        let passive = ScriptPassiveDef {
            name: "passive".into(),
            stat_modifiers: vec![],
            damage_multiplier: None,
            on_kill_heal: -1,
        };
        assert!(
            Ruleset::with_global_script_ids(&[], &[], &[passive], &[], &[], &[])
                .unwrap_err()
                .contains("must not be negative")
        );

        let reaction = ScriptReactionDef {
            name: "reaction".into(),
            ap_cost: Some(0),
            mana_cost: 0,
            target_damage: -1,
            self_heal: 0,
        };
        assert!(
            Ruleset::with_global_script_ids(&[], &[], &[], &[reaction], &[], &[])
                .unwrap_err()
                .contains("must not be negative")
        );

        assert!(
            Ruleset::with_global_script_ids(&[], &[], &[], &[], &[movement("movement", -1)], &[],)
                .unwrap_err()
                .contains("must not be negative")
        );
    }
}
