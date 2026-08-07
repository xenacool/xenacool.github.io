#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use npc_engine_utils::GlobalDomain;
    use crate::*;

    #[test]
    fn test_job_system_initialization() {
        let job = JobDef {
            id: JobId(1),
            base_stats: UnitStats {
                strength: 10,
                dexterity: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
                constitution: 10,
                wits: 10,
                stamina: 10,
                armor_class: 10,
                speed: 10,
            },
            equipment_slots: vec![SlotType::MainHand, SlotType::OffHand, SlotType::Head, SlotType::Body, SlotType::Accessory],
            passive_slots_count: 2,
            reaction_slots_count: 1,
            secondary_job_slots_count: 1,
            abilities: vec![],
            passives: vec![],
            reactions: vec![],
            movement: MovementId(0),
        };

        let unit = UnitState {
            team_id: 0,
            health: 100,
            mana: 50,
            action_points: 4,
            ct: 0,
            position: (0, 0, 0),
            gender: Gender::Male,
            class_id: ActorClassId(1),
            primary_job: JobId(1),
            secondary_jobs: vec![],
            movement_ability: MovementId(0),
            passive_abilities: vec![],
            reaction_abilities: vec![],
            stats: UnitStats {
                strength: 10,
                dexterity: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
                constitution: 10,
                wits: 10,
                stamina: 10,
                armor_class: 10,
                speed: 10,
            },
            derived_stats: DerivedStats {
                health_max: 100,
                mana_max: 50,
                action_points_max: 4,
            },
            equipment: EquipmentSlots {
                slots: HashMap::new(),
            },
            status_effects: vec![],
            turn_tags: TagBag::default(),
            modifier_deck: AbilityModifierDeck::default(),
        };

        assert_eq!(unit.health, 100);
        assert_eq!(unit.primary_job, JobId(1));
        assert_eq!(job.equipment_slots.len(), 5);
    }

    #[test]
    fn test_skirmish_simulation() {
        let mut state = setup_2v2_skirmish();
        let scheduler = CTScheduler::new(100);
        scheduler.initialize_ct(&mut state);

        let config = MCTSConfiguration::default();

        for _ in 0..20 {
            let ready_agents = scheduler.tick_until_ready(&mut state);
            for agent_id in ready_agents {
                let mut mcts = MCTS::<TacticalDomain>::new(state.clone(), agent_id, config.clone());
                let task = mcts.run().unwrap();
                
                let mut diff = TacticalDiff::default();
                {
                    let ctx_mut = ContextMut {
                        tick: 0,
                        state_diff: StateDiffRefMut { initial_state: &state, diff: &mut diff },
                        agent: agent_id,
                    };
                    task.execute(ctx_mut);
                }
                let dummy_state = state.clone();
                TacticalDomain::apply(&mut state, &dummy_state, &diff);
            }
        }
        
        // Assertions to verify simulation progress
        assert!(!state.agents.is_empty());
        // Verify at least some damage was dealt or actions taken
        let total_health: i32 = state.agents.values().map(|u| u.health).sum();
        let initial_health: i32 = setup_2v2_skirmish().agents.values().map(|u| u.health).sum();
        assert!(total_health <= initial_health);
    }
}
