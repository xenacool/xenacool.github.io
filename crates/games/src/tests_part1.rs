    #[test]
    fn grid_map_keeps_in_bounds_holes_sparse() {
        let mut grid = GridMap {
            bounds: GridBounds {
                horizontal: hexx::HexBounds::from_radius(1),
                min_layer: 0,
                max_layer: 1,
            },
            tiles: HashMap::new(),
        };
        let ground = GridCell::new(hexx::Hex::new(0, 0), 0);
        let hole = GridCell::new(hexx::Hex::new(1, 0), 0);
        let upper = GridCell::new(hexx::Hex::new(0, 0), 1);

        grid.set_tile(ground, TileType::Grass).unwrap();
        grid.set_tile(upper, TileType::Rock).unwrap();

        assert!(grid.contains(ground));
        assert!(grid.contains(upper));
        assert!(!grid.contains(hole));
        assert_eq!(TileType::Rock.material_name(), "rock");
    }

    #[test]
    fn grid_map_rejects_out_of_bounds_cells() {
        let mut grid = GridMap::default();
        let outside = GridCell::new(hexx::Hex::new(1, 0), 0);

        assert!(grid.set_tile(outside, TileType::Dirt).is_err());
    }

    
    #[test]
    fn reachable_cells_respects_holes_and_occupancy() {
        let mut config = SkirmishConfig::new(42);
        config
            .add_unit(1, 1, "Caveman", GridCell::new(Hex::new(0, 0), 0))
            .expect("default unit config");
        config
            .add_unit(2, 1, "Mage", GridCell::new(Hex::new(1, -1), 0))
            .expect("default unit config");
        config
            .add_unit(3, 2, "Necromancer", GridCell::new(Hex::new(5, -5), 0))
            .expect("default unit config");
        config
            .add_unit(4, 2, "Skeleton_Minion", GridCell::new(Hex::new(4, -4), 0))
            .expect("default unit config");
        let mut state = config.build_state().expect("default skirmish config");
        let start = state.agents[&AgentId(1)].position;
        let occupied = state.agents[&AgentId(2)].position;
        state
            .grid
            .tiles
            .remove(&GridCell::new(hexx::Hex::new(1, 0), 0));

        let reachable = reachable_cells(&state, AgentId(1)).unwrap();
        assert!(!reachable.contains_key(&GridCell::new(hexx::Hex::new(1, 0), 0)));
        assert!(!reachable.contains_key(&occupied));
        assert!(!reachable.contains_key(&start));
    }

    #[test]
    fn reachable_cells_uses_movement_defined_vertical_transitions() {
        let mut config = SkirmishConfig::new(42);
        config
            .add_unit(1, 1, "Caveman", GridCell::new(Hex::new(0, 0), 0))
            .expect("default unit config");
        config
            .add_unit(2, 1, "Mage", GridCell::new(Hex::new(1, -1), 0))
            .expect("default unit config");
        config
            .add_unit(3, 2, "Necromancer", GridCell::new(Hex::new(5, -5), 0))
            .expect("default unit config");
        config
            .add_unit(4, 2, "Skeleton_Minion", GridCell::new(Hex::new(4, -4), 0))
            .expect("default unit config");
        let mut state = config.build_state().expect("default skirmish config");
        let upper = GridCell::new(hexx::Hex::new(0, 0), 1);
        state.grid.bounds.max_layer = 1;
        state.grid.set_tile(upper, TileType::Rock).unwrap();
        state.movement_registry.insert(
            MovementId(999),
            MoveProgram {
                id: MovementId(999),
                name: "Climb".to_string(),
                steps_ap_cost: vec![(1, 1)],
                vertical_deltas: vec![1],
                crosses_holes: false,
                crosses_occupied: false,
                teleport_range: None,
                emit_tags: vec![],
                consume_tags: vec![],
            },
        );
        let unit = state.agents.get_mut(&AgentId(1)).unwrap();
        unit.movement_ability = MovementId(999);
        unit.action_points = 1;

        let reachable = reachable_cells(&state, AgentId(1)).unwrap();
        assert_eq!(reachable.get(&upper), Some(&1));
    }

    #[test]
    fn test_job_system_initialization() {
        let job = JobDef {
            id: JobId(1),
            name: "Test Job".to_string(),
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
            equipment_slots: vec![
                SlotType::MainHand,
                SlotType::OffHand,
                SlotType::Head,
                SlotType::Body,
                SlotType::Accessory,
            ],
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
            position: GridCell::new(hexx::Hex::new(0, 0), 0),
            gender: Gender::Male,
            class_id: ActorClassId(1),
            primary_job: JobId(1),
            secondary_jobs: vec![],
            job_history: vec![],
            purchased_abilities: vec![],
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
            timed_modifiers: vec![],
        };

        assert_eq!(unit.health, 100);
        assert_eq!(unit.primary_job, JobId(1));
        assert_eq!(job.equipment_slots.len(), 5);
    }

    #[test]
    fn definition_ids_are_contiguous_and_reject_exhaustion() {
        let mut zero_based = DefinitionIdAllocator::new(0);
        assert_eq!(zero_based.allocate().unwrap(), 0);
        assert_eq!(zero_based.allocate().unwrap(), 1);
        let mut allocator = DefinitionIdAllocator::new(u32::MAX - 1);
        assert_eq!(allocator.allocate().unwrap(), u32::MAX - 1);
        assert!(allocator.allocate().is_err());
    }

    #[test]
    fn unit_progression_compresses_history_and_keeps_purchased_actions_separate() {
        let mut registry_scenario = SkirmishConfig::new(42);
        registry_scenario
            .add_unit(99, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        let registry_state = registry_scenario.build_state().unwrap();
        let jobs = &registry_state.job_registry;
        let caveman_id = jobs.values().find(|job| job.name == "Caveman").unwrap().id;
        let mage_id = jobs.values().find(|job| job.name == "Mage").unwrap().id;
        let caveman = &jobs[&caveman_id];
        let mage = &jobs[&mage_id];
        let mut unit = UnitState {
            team_id: 1,
            health: 100,
            mana: 50,
            action_points: 4,
            ct: 0,
            position: GridCell::new(hexx::Hex::ZERO, 0),
            gender: Gender::Male,
            class_id: ActorClassId(1),
            primary_job: caveman_id,
            secondary_jobs: vec![mage_id],
            job_history: vec![],
            purchased_abilities: vec![AbilityId(201)],
            movement_ability: MovementId(0),
            passive_abilities: vec![],
            reaction_abilities: vec![],
            stats: UnitStats {
                strength: 0,
                dexterity: 0,
                intelligence: 0,
                wisdom: 0,
                charisma: 0,
                constitution: 0,
                wits: 0,
                stamina: 0,
                armor_class: 0,
                speed: 0,
            },
            derived_stats: DerivedStats {
                health_max: 0,
                mana_max: 0,
                action_points_max: 4,
            },
            equipment: EquipmentSlots {
                slots: HashMap::new(),
            },
            status_effects: vec![],
            turn_tags: TagBag::default(),
            modifier_deck: AbilityModifierDeck::default(),
            timed_modifiers: vec![],
        };
        unit.apply_job_level(caveman).unwrap();
        unit.apply_job_level(caveman).unwrap();
        unit.apply_job_level(mage).unwrap();

        assert_eq!(
            unit.job_history,
            vec![
                JobHistoryEntry {
                    job_id: caveman_id,
                    consecutive_levels: 2
                },
                JobHistoryEntry {
                    job_id: mage_id,
                    consecutive_levels: 1
                },
            ]
        );
        assert_eq!(unit.stats, mage.base_stats);
        assert_eq!(unit.purchased_abilities, vec![AbilityId(201)]);
        let available = unit.available_action_abilities(&jobs).unwrap();
        assert!(available.contains(&AbilityId(201)));
    }

    #[test]
    fn skirmish_history_uses_the_same_unit_transition_as_runtime_progression() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario.add_secondary_job(1, "Mage").unwrap();
        scenario.add_job_history(1, "Mage", 1).unwrap();

        let state = scenario.build_state().unwrap();
        let unit = state.agents.get(&AgentId(1)).unwrap();
        let mage_id = state
            .job_registry
            .values()
            .find(|job| job.name == "Mage")
            .map(|job| job.id)
            .unwrap();
        assert_eq!(
            unit.job_history,
            vec![JobHistoryEntry {
                job_id: mage_id,
                consecutive_levels: 1,
            }]
        );
        assert_eq!(unit.stats, state.job_registry[&mage_id].base_stats);
    }

    #[test]
    fn builtin_script_registry_resolves_symbolic_job_names() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        let state = scenario.build_state().unwrap();
        let caveman = state
            .job_registry
            .values()
            .find(|job| job.name == "Caveman")
            .unwrap();
        assert_eq!(caveman.abilities.len(), 3);
        assert_eq!(caveman.passives.len(), 1);
        assert_eq!(caveman.reactions.len(), 1);
        assert_eq!(state.movement_registry[&caveman.movement].name, "Plain Move");
    }

    #[test]
    fn symbolic_job_definition_rejects_missing_references() {
        let mut spec = ScriptJobDef::new("Broken");
        spec.base_stats = Some(UnitStats {
            strength: 1,
            dexterity: 1,
            intelligence: 1,
            wisdom: 1,
            charisma: 1,
            constitution: 1,
            wits: 1,
            stamina: 1,
            armor_class: 1,
            speed: 1,
        });
        spec.passive_slots_count = Some(0);
        spec.reaction_slots_count = Some(0);
        spec.secondary_job_slots_count = Some(0);
        spec.ability_names.push("Missing Ability".to_string());
        spec.movement_name = Some("Plain Move".to_string());

        let error = spec
            .resolve(
                JobId(99),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
            )
            .unwrap_err();
        assert!(error.contains("Unknown ability Missing Ability"));
    }
