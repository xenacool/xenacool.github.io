use crate::{
    AbilityModifierDeck, ActorClassId, AgentId, CollisionMap, DerivedStats, EquipmentSlots, Gender,
    GridBounds, GridCell, JobHistoryEntry, JobId, Logger, Ruleset, ScriptAbilityDef, ScriptJobDef,
    ScriptMovementDef, ScriptPassiveDef, ScriptReactionDef, ScriptTagDef, SeededRng, SlotType,
    TacticalGrid, TacticalState, TagBag, TagRegistry, TileType, UnitState, UnitStats,
};
use hexx::Hex;
use pystral_physics::ProjectileCollider;
use skirmish_defs::{builtin_script_abilities, builtin_script_jobs};
use std::collections::HashMap;

#[path = "skirmish_defs.rs"]
mod skirmish_defs;

#[derive(Debug, Clone)]
pub struct UnitConfig {
    pub id: AgentId,
    pub team_id: u8,
    pub primary_job: String,
    pub secondary_jobs: Vec<String>,
    pub job_history: Vec<(String, u32)>,
    pub purchased_abilities: Vec<crate::AbilityId>,
    pub position: GridCell,
}

#[derive(Debug, Clone)]
pub struct SkirmishConfig {
    pub seed: u64,
    pub ct_threshold: i32,
    pub maximum_turn_count: u32,
    pub grid: TacticalGrid,
    pub units: Vec<UnitConfig>,
    pub script_jobs: Vec<ScriptJobDef>,
    pub script_abilities: Vec<ScriptAbilityDef>,
    pub script_tags: Vec<ScriptTagDef>,
    pub script_passives: Vec<ScriptPassiveDef>,
    pub script_reactions: Vec<ScriptReactionDef>,
    pub script_movements: Vec<ScriptMovementDef>,
    pub collision: CollisionMap,
}

impl SkirmishConfig {
    pub fn new(seed: u64) -> Self {
        Self::new_empty(seed).with_builtin_script_registry()
    }

    pub fn new_empty(seed: u64) -> Self {
        let mut grid = TacticalGrid {
            bounds: GridBounds {
                horizontal: hexx::HexBounds::from_radius(6),
                min_layer: 0,
                max_layer: 0,
            },
            tiles: HashMap::new(),
        };
        for hex in Hex::ZERO.range(6) {
            grid.tiles.insert(GridCell::new(hex, 0), TileType::Grass);
        }

        Self {
            seed,
            ct_threshold: 100,
            // Zero means no scenario turn cap; games end only when one team
            // remains. Explicit positive values remain useful for tests.
            maximum_turn_count: 0,
            grid,
            units: Vec::new(),
            script_jobs: Vec::new(),
            script_abilities: Vec::new(),
            script_tags: Vec::new(),
            script_passives: Vec::new(),
            script_reactions: Vec::new(),
            script_movements: Vec::new(),
            collision: CollisionMap::default(),
        }
    }

    pub fn with_builtin_script_registry(mut self) -> Self {
        for tag in [
            ("Stun", 1),
            ("Slow", 1),
            ("Shield", 2),
            ("Armor", 2),
            ("Fear", 1),
            ("Mana", 1),
        ] {
            self.script_tags.push(ScriptTagDef {
                name: tag.0.to_string(),
                max_stacks: Some(tag.1),
            });
        }
        for passive in [
            "Thick Skin",
            "Spell Echo",
            "Death's Embrace",
            "Undead Resilience",
        ] {
            self.script_passives.push(ScriptPassiveDef {
                name: passive.to_string(),
            });
        }
        for (name, ap_cost) in [
            ("Counter-Swing", 1),
            ("Mana Shield", 0),
            ("Vengeful Spirit", 1),
            ("Bone Splinter", 0),
        ] {
            self.script_reactions.push(ScriptReactionDef {
                name: name.to_string(),
                ap_cost: Some(ap_cost),
            });
        }
        self.script_movements = vec![
            ScriptMovementDef {
                name: "Plain Move".into(),
                steps_ap_cost: vec![(1, 1)],
                vertical_deltas: vec![],
                crosses_holes: false,
                crosses_occupied: false,
                teleport_range: None,
                emit_tags: vec![],
                consume_tags: vec![],
            },
            ScriptMovementDef {
                name: "Manafeet".into(),
                steps_ap_cost: vec![(1, 1)],
                vertical_deltas: vec![],
                crosses_holes: false,
                crosses_occupied: false,
                teleport_range: None,
                emit_tags: vec![("Mana".into(), 1)],
                consume_tags: vec![],
            },
            ScriptMovementDef {
                name: "Shadow Step".into(),
                steps_ap_cost: vec![(1, 2)],
                vertical_deltas: vec![],
                crosses_holes: true,
                crosses_occupied: false,
                teleport_range: Some(2),
                emit_tags: vec![],
                consume_tags: vec![],
            },
            ScriptMovementDef {
                name: "Rattle Dash".into(),
                steps_ap_cost: vec![(1, 1), (4, 2)],
                vertical_deltas: vec![],
                crosses_holes: false,
                crosses_occupied: false,
                teleport_range: None,
                emit_tags: vec![],
                consume_tags: vec![],
            },
        ];
        self.script_abilities = builtin_script_abilities();
        self.script_jobs = builtin_script_jobs();
        self
    }

    pub fn set_ct_threshold(&mut self, value: i64) -> Result<(), String> {
        let threshold = i32::try_from(value)
            .map_err(|_| "CT threshold must be an integer in the range 1..=1000000".to_string())?;
        if !(1..=1_000_000).contains(&threshold) {
            return Err("CT threshold must be in the range 1..=1000000".to_string());
        }
        self.ct_threshold = threshold;
        Ok(())
    }

    pub fn set_maximum_turn_count(&mut self, value: i64) -> Result<(), String> {
        self.maximum_turn_count = value.max(0) as u32;
        Ok(())
    }

    pub fn set_projectile_speed_min(&mut self, value: f32) {
        self.collision.trajectory.speed_min = value;
    }
    pub fn set_projectile_speed_max(&mut self, value: f32) {
        self.collision.trajectory.speed_max = value;
    }
    pub fn set_projectile_speed_step(&mut self, value: f32) {
        self.collision.trajectory.speed_step = value;
    }
    pub fn set_projectile_angle_min(&mut self, value: f32) {
        self.collision.trajectory.angle_min_degrees = value;
    }
    pub fn set_projectile_angle_max(&mut self, value: f32) {
        self.collision.trajectory.angle_max_degrees = value;
    }
    pub fn set_projectile_angle_step(&mut self, value: f32) {
        self.collision.trajectory.angle_step_degrees = value;
    }
    pub fn set_projectile_gravity(&mut self, value: f32) {
        self.collision.trajectory.gravity = value;
    }
    pub fn set_projectile_time_step(&mut self, value: f32) {
        self.collision.trajectory.time_step = value;
    }
    pub fn set_projectile_max_steps(&mut self, value: u32) {
        self.collision.trajectory.max_steps = value;
    }
    pub fn set_projectile_ground_cutoff(&mut self, value: f32) {
        self.collision.trajectory.ground_cutoff = value;
    }
    pub fn set_projectile_collider(&mut self, collider: ProjectileCollider) {
        self.collision.trajectory.collider = collider;
    }

    pub fn set_grid(&mut self, grid: TacticalGrid) {
        self.grid = grid;
    }

    pub fn add_script_job(&mut self, job: ScriptJobDef) -> Result<(), String> {
        if self
            .script_jobs
            .iter()
            .any(|existing| existing.name == job.name)
        {
            return Err(format!("Duplicate script job: {}", job.name));
        }
        self.script_jobs.push(job);
        Ok(())
    }

    pub fn add_script_ability(&mut self, ability: ScriptAbilityDef) -> Result<(), String> {
        if self
            .script_abilities
            .iter()
            .any(|existing| existing.name == ability.name)
        {
            return Err(format!("Duplicate script ability: {}", ability.name));
        }
        self.script_abilities.push(ability);
        Ok(())
    }

    pub fn add_script_tag(&mut self, tag: ScriptTagDef) -> Result<(), String> {
        if self
            .script_tags
            .iter()
            .any(|existing| existing.name == tag.name)
        {
            return Err(format!("Duplicate script tag: {}", tag.name));
        }
        self.script_tags.push(tag);
        Ok(())
    }

    pub fn add_script_passive(&mut self, passive: ScriptPassiveDef) -> Result<(), String> {
        if self
            .script_passives
            .iter()
            .any(|existing| existing.name == passive.name)
        {
            return Err(format!("Duplicate script passive: {}", passive.name));
        }
        self.script_passives.push(passive);
        Ok(())
    }

    pub fn add_script_reaction(&mut self, reaction: ScriptReactionDef) -> Result<(), String> {
        if self
            .script_reactions
            .iter()
            .any(|existing| existing.name == reaction.name)
        {
            return Err(format!("Duplicate script reaction: {}", reaction.name));
        }
        self.script_reactions.push(reaction);
        Ok(())
    }

    pub fn add_script_movement(&mut self, movement: ScriptMovementDef) -> Result<(), String> {
        if self
            .script_movements
            .iter()
            .any(|existing| existing.name == movement.name)
        {
            return Err(format!("Duplicate script movement: {}", movement.name));
        }
        self.script_movements.push(movement);
        Ok(())
    }

    pub fn add_unit(
        &mut self,
        id: i64,
        team_id: i64,
        primary_job: &str,
        position: GridCell,
    ) -> Result<(), String> {
        let id = u32::try_from(id).map_err(|_| "Unit ID must be non-negative".to_string())?;
        let team_id = u8::try_from(team_id).map_err(|_| "Team ID is out of range".to_string())?;
        if primary_job.trim().is_empty() {
            return Err("Primary job name must not be empty".to_string());
        }
        if self.units.iter().any(|unit| unit.id == AgentId(id)) {
            return Err(format!("Duplicate unit ID: {id}"));
        }
        if !self.grid.contains(position) {
            return Err(format!(
                "Unit position is not an occupied grid cell: {position:?}"
            ));
        }
        self.units.push(UnitConfig {
            id: AgentId(id),
            team_id,
            primary_job: primary_job.to_string(),
            secondary_jobs: Vec::new(),
            job_history: Vec::new(),
            purchased_abilities: Vec::new(),
            position,
        });
        Ok(())
    }

    pub fn add_secondary_job(&mut self, id: i64, job_name: &str) -> Result<(), String> {
        let id = u32::try_from(id).map_err(|_| "Unit ID must be non-negative".to_string())?;
        let unit = self
            .units
            .iter_mut()
            .find(|unit| unit.id == AgentId(id))
            .ok_or_else(|| format!("Unknown unit ID: {id}"))?;
        unit.secondary_jobs.push(job_name.to_string());
        Ok(())
    }

    pub fn add_job_history(
        &mut self,
        id: i64,
        job_name: &str,
        consecutive_levels: i64,
    ) -> Result<(), String> {
        let id = u32::try_from(id).map_err(|_| "Unit ID must be non-negative".to_string())?;
        let consecutive_levels = u32::try_from(consecutive_levels)
            .map_err(|_| "Consecutive job levels must be positive".to_string())?;
        if consecutive_levels == 0 {
            return Err("Consecutive job levels must be positive".to_string());
        }
        let unit = self
            .units
            .iter_mut()
            .find(|unit| unit.id == AgentId(id))
            .ok_or_else(|| format!("Unknown unit ID: {id}"))?;
        if unit
            .job_history
            .last()
            .is_some_and(|(name, _)| name == job_name)
        {
            return Err("Job history must use compressed consecutive runs".to_string());
        }
        unit.job_history
            .push((job_name.to_string(), consecutive_levels));
        Ok(())
    }

    pub fn build_state(&self) -> Result<TacticalState, String> {
        if !(1..=1_000_000).contains(&self.ct_threshold) {
            return Err("CT threshold must be in the range 1..=1000000".to_string());
        }
        let ruleset = Ruleset::with_global_script_ids(
            &self.script_tags,
            &self.script_abilities,
            &self.script_passives,
            &self.script_reactions,
            &self.script_movements,
            &self.script_jobs,
        )?;
        let mut state = TacticalState {
            agents: HashMap::new(),
            grid: self.grid.clone(),
            collision: Some(self.collision.clone()),
            logger: Logger::default(),
            reaction_queue: vec![],
            rng: SeededRng::new(self.seed),
            ability_registry: ruleset.abilities.clone(),
            job_registry: ruleset.jobs.clone(),
            movement_registry: ruleset.movements.clone(),
            reaction_registry: ruleset.reactions.clone(),
            tag_registry: TagRegistry {
                defs: ruleset.tags.clone(),
            },
        };

        let job_defs = state.job_registry.clone();
        for unit_config in &self.units {
            let resolve_job = |name: &str| -> Result<JobId, String> {
                job_defs
                    .iter()
                    .find(|(_, job)| job.name == name)
                    .map(|(id, _)| *id)
                    .ok_or_else(|| format!("Missing job definition: {name}"))
            };
            let primary_job_id = resolve_job(&unit_config.primary_job)?;
            let secondary_jobs = unit_config
                .secondary_jobs
                .iter()
                .map(|name| resolve_job(name))
                .collect::<Result<Vec<_>, _>>()?;
            let job = job_defs
                .get(&primary_job_id)
                .ok_or_else(|| format!("Missing job definition: {}", unit_config.primary_job))?;
            if secondary_jobs.len() > usize::from(job.secondary_job_slots_count) {
                return Err(format!(
                    "Unit {:?} exceeds secondary job slots",
                    unit_config.id
                ));
            }
            let mut unit = UnitState {
                team_id: unit_config.team_id,
                health: job.base_stats.constitution * 10,
                mana: job.base_stats.intelligence * 5,
                action_points: 4,
                ct: 0,
                position: unit_config.position,
                gender: Gender::Male,
                class_id: ActorClassId(1),
                primary_job: primary_job_id,
                secondary_jobs,
                job_history: Vec::new(),
                purchased_abilities: unit_config.purchased_abilities.clone(),
                movement_ability: job.movement,
                passive_abilities: job.passives.clone(),
                reaction_abilities: job.reactions.clone(),
                stats: job.base_stats.clone(),
                equipment: EquipmentSlots {
                    slots: HashMap::new(),
                },
                status_effects: vec![],
                turn_tags: TagBag::default(),
                modifier_deck: AbilityModifierDeck::default(),
                timed_modifiers: vec![],
                derived_stats: DerivedStats {
                    health_max: job.base_stats.constitution * 10,
                    mana_max: job.base_stats.intelligence * 5,
                    action_points_max: 4,
                },
            };
            let history = if unit_config.job_history.is_empty() {
                vec![JobHistoryEntry {
                    job_id: primary_job_id,
                    consecutive_levels: 1,
                }]
            } else {
                unit_config
                    .job_history
                    .iter()
                    .map(|(name, consecutive_levels)| {
                        Ok(JobHistoryEntry {
                            job_id: resolve_job(name)?,
                            consecutive_levels: *consecutive_levels,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?
            };
            for entry in history {
                let history_job = job_defs
                    .get(&entry.job_id)
                    .ok_or_else(|| format!("Missing job definition: {:?}", entry.job_id))?;
                for _ in 0..entry.consecutive_levels {
                    unit.apply_job_level(history_job)?;
                }
            }
            state.agents.insert(unit_config.id, unit);
        }
        Ok(state)
    }
}
