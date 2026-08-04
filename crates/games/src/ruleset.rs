use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    AbilityDef, AbilityDelivery, AbilityId, DefinitionIdAllocator, JobDef, JobId, MoveProgram,
    MovementId, PassiveDef, PassiveId, ReactionDef, ReactionId, ScriptJobDef, TagDef, TagId,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptPassiveDef {
    pub name: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptAbilityDef {
    pub name: String,
    pub ap_cost: Option<u8>,
    pub range: Option<u8>,
    pub delivery: Option<String>,
    pub area_radius: Option<u8>,
    pub scaling: Vec<(String, f32)>,
    pub emit_tags: Vec<(String, u8)>,
    pub consume_tags: Vec<(String, u8, u8)>,
}

impl ScriptAbilityDef {
    pub fn resolve(
        &self,
        id: AbilityId,
        tags: &HashMap<String, TagId>,
    ) -> Result<AbilityDef, String> {
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
        Ok(AbilityDef {
            id,
            name: self.name.clone(),
            ap_cost: self
                .ap_cost
                .ok_or_else(|| format!("Ability {} is missing AP cost", self.name))?,
            range: self
                .range
                .ok_or_else(|| format!("Ability {} is missing range", self.name))?,
            delivery,
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
            let id = PassiveId(allocator.allocate()?);
            passive_map.insert(
                id,
                PassiveDef {
                    id,
                    name: definition.name.clone(),
                },
            );
        }
        let mut reaction_map = HashMap::new();
        for definition in reactions {
            let id = ReactionId(allocator.allocate()?);
            let ap_cost = definition
                .ap_cost
                .ok_or_else(|| format!("Reaction {} is missing AP cost", definition.name))?;
            reaction_map.insert(
                id,
                ReactionDef {
                    id,
                    name: definition.name.clone(),
                    ap_cost,
                },
            );
        }
        let mut movement_map = HashMap::new();
        for definition in movements {
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
