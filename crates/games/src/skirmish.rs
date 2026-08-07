use std::collections::HashMap;
use crate::{TacticalState, TacticalGrid, Logger, SeededRng, get_ability_defs, get_job_defs, get_movement_defs, get_reaction_defs, TagRegistry, JobId, UnitState, Gender, ActorClassId, EquipmentSlots, TagBag, AbilityModifierDeck, DerivedStats, AgentId, JOB_CAVEMAN, JOB_MAGE, JOB_NECROMANCER, JOB_SKELETON_MINION};

pub fn setup_2v2_skirmish() -> TacticalState {
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::default(),
        reaction_queue: vec![],
        rng: SeededRng::new(42),
        ability_registry: get_ability_defs(),
        job_registry: get_job_defs(),
        movement_registry: get_movement_defs(),
        reaction_registry: get_reaction_defs(),
        tag_registry: TagRegistry { defs: HashMap::new() },
    };

    let job_defs = state.job_registry.clone();

    // Helper to create unit from job
    let create_unit = |job_id: JobId, team_id: u8, pos: (i32, i32, i32)| -> UnitState {
        let job = &job_defs[&job_id];
        UnitState {
            team_id,
            health: job.base_stats.constitution * 10,
            mana: job.base_stats.intelligence * 5,
            action_points: 4,
            ct: 0,
            position: pos,
            gender: Gender::Male,
            class_id: ActorClassId(1),
            primary_job: job_id,
            secondary_jobs: vec![],
            movement_ability: job.movement,
            passive_abilities: job.passives.clone(),
            reaction_abilities: job.reactions.clone(),
            stats: job.base_stats.clone(),
            equipment: EquipmentSlots { slots: HashMap::new() },
            status_effects: vec![],
            turn_tags: TagBag::default(),
            modifier_deck: AbilityModifierDeck::default(),
            derived_stats: DerivedStats {
                health_max: job.base_stats.constitution * 10,
                mana_max: job.base_stats.intelligence * 5,
                action_points_max: 4,
            },
        }
    };

    state.agents.insert(AgentId(1), create_unit(JOB_CAVEMAN, 1, (0, 0, 0)));
    state.agents.insert(AgentId(2), create_unit(JOB_MAGE, 1, (1, -1, 0)));
    state.agents.insert(AgentId(3), create_unit(JOB_NECROMANCER, 2, (5, -5, 0)));
    state.agents.insert(AgentId(4), create_unit(JOB_SKELETON_MINION, 2, (4, -4, 0)));

    state
}
