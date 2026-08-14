use pystral_core::ui_log::Logger;
use pystral_games::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;

fn setup_unit(_id: u64, speed: i32, wits: i32) -> UnitState {
    UnitState {
        team_id: 1,
        health: 100,
        mana: 100,
        action_points: 4,
        ct: 0,
        position: GridCell::new(hexx::Hex::new(0, 0), 0),
        gender: Gender::Nonbinary,
        class_id: ActorClassId(1),
        primary_job: JobId(1),
        secondary_jobs: vec![],
        job_history: vec![],
        purchased_abilities: vec![],
        movement_ability: MovementId(0),
        passive_abilities: Vec::new(),
        reaction_abilities: vec![],
        stats: UnitStats {
            strength: 10,
            dexterity: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
            constitution: 10,
            wits,
            stamina: 10,
            armor_class: 1,
            speed,
        },
        equipment: EquipmentSlots {
            slots: HashMap::new(),
        },
        status_effects: Vec::new(),
        turn_tags: TagBag::default(),
        modifier_deck: AbilityModifierDeck::default(),
        timed_modifiers: vec![],
        derived_stats: DerivedStats {
            health_max: 100,
            mana_max: 100,
            action_points_max: 4,
        },
    }
}

#[test]
fn test_ct_scheduler() {
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::new(),
        reaction_queue: Vec::new(),
        rng: SeededRng::new(42),
        ability_registry: HashMap::new(),
        job_registry: HashMap::new(),
        movement_registry: HashMap::new(),
        reaction_registry: HashMap::new(),
        tag_registry: TagRegistry {
            defs: HashMap::new(),
        },
    };

    let id1 = AgentId(1);
    let id2 = AgentId(2);

    state.agents.insert(id1, setup_unit(1, 100, 10)); // Ready in 10 ticks
    state.agents.insert(id2, setup_unit(2, 50, 20)); // Ready in 20 ticks

    let scheduler = CTScheduler::new(1000);

    let ready = scheduler.tick_until_ready(&mut state);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0], id1);
    assert_eq!(state.agents[&id1].ct, 1000);
    assert_eq!(state.agents[&id2].ct, 500);

    // After acting, deduct CT
    state.agents.get_mut(&id1).unwrap().ct -= scheduler.calculate_deduction(4, 4);
    assert_eq!(state.agents[&id1].ct, 0);

    let ready = scheduler.tick_until_ready(&mut state);
    assert_eq!(ready[0], id2);
    assert_eq!(state.agents[&id2].ct, 1000);
    assert_eq!(state.agents[&id1].ct, 1000); // Also ready now because 10 more ticks passed

    // Check sorting by CT (both 1000) then WITS (id2 has 20, id1 has 10)
    assert_eq!(ready, vec![id2, id1]);
}

#[test]
fn test_ct_priority_and_initialization() {
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::new(),
        reaction_queue: Vec::new(),
        rng: SeededRng::new(42),
        ability_registry: HashMap::new(),
        job_registry: HashMap::new(),
        movement_registry: HashMap::new(),
        reaction_registry: HashMap::new(),
        tag_registry: TagRegistry {
            defs: HashMap::new(),
        },
    };

    let id1 = AgentId(1); // Higher Speed, Lower Wits
    let id2 = AgentId(2); // Lower Speed, Higher Wits

    state.agents.insert(id1, setup_unit(1, 100, 10));
    state.agents.insert(id2, setup_unit(2, 90, 20));

    let scheduler = CTScheduler::new(1000);

    // Test initialization
    scheduler.initialize_ct(&mut state);
    assert_eq!(state.agents[&id1].ct, 10);
    assert_eq!(state.agents[&id2].ct, 20);

    // After 10 ticks:
    // id1: 10 + 100 * 10 = 1010 (Ready)
    // id2: 20 + 90 * 10 = 920 (Not Ready)
    let ready = scheduler.tick_until_ready(&mut state);
    assert_eq!(ready, vec![id1]);
    assert_eq!(state.agents[&id1].ct, 1010);
    assert_eq!(state.agents[&id2].ct, 920);

    // Reset id1 and tick again until id2 is ready
    state.agents.get_mut(&id1).unwrap().ct = 0;

    // Next tick:
    // id1: 0 + 100 = 100
    // id2: 920 + 90 = 1010 (Ready)
    let ready = scheduler.tick_until_ready(&mut state);
    assert_eq!(ready, vec![id2]);
}

#[test]
fn test_ct_tie_breaker_wits() {
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::new(),
        reaction_queue: Vec::new(),
        rng: SeededRng::new(42),
        ability_registry: HashMap::new(),
        job_registry: HashMap::new(),
        movement_registry: HashMap::new(),
        reaction_registry: HashMap::new(),
        tag_registry: TagRegistry {
            defs: HashMap::new(),
        },
    };

    let id1 = AgentId(1);
    let id2 = AgentId(2);

    // Same CT, different Wits
    let mut unit1 = setup_unit(1, 100, 10);
    unit1.ct = 1000;
    let mut unit2 = setup_unit(2, 100, 20);
    unit2.ct = 1000;

    state.agents.insert(id1, unit1);
    state.agents.insert(id2, unit2);

    let scheduler = CTScheduler::new(1000);
    let ready = scheduler.tick_until_ready(&mut state);

    // Both have 1000 CT, so id2 should be first due to higher Wits (20 > 10)
    assert_eq!(ready, vec![id2, id1]);
}

#[test]
fn test_ct_higher_priority_than_wits() {
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::new(),
        reaction_queue: Vec::new(),
        rng: SeededRng::new(42),
        ability_registry: HashMap::new(),
        job_registry: HashMap::new(),
        movement_registry: HashMap::new(),
        reaction_registry: HashMap::new(),
        tag_registry: TagRegistry {
            defs: HashMap::new(),
        },
    };

    let id1 = AgentId(1);
    let id2 = AgentId(2);

    // id1 has higher CT but lower Wits
    let mut unit1 = setup_unit(1, 100, 10);
    unit1.ct = 1100;
    let mut unit2 = setup_unit(2, 100, 20);
    unit2.ct = 1050;

    state.agents.insert(id1, unit1);
    state.agents.insert(id2, unit2);

    let scheduler = CTScheduler::new(1000);
    let ready = scheduler.tick_until_ready(&mut state);

    // id1 has higher CT (1100 > 1050), so it should be first
    assert_eq!(ready, vec![id1, id2]);
}

#[test]
fn test_ap_movement_costs() {
    let mut tag_bag = TagBag::default();
    let move_prog = MoveProgram {
        id: MovementId(0),
        name: "Test Move".to_string(),
        steps_ap_cost: vec![(3, 2), (5, 3)], // 1-2: 1AP, 3-4: 2AP, 5+: 3AP
        vertical_deltas: vec![],
        crosses_holes: false,
        crosses_occupied: false,
        teleport_range: None,
        emit_tags: vec![],
        consume_tags: vec![],
    };

    assert_eq!(move_prog.get_ap_cost(0, &mut tag_bag), 1);
    assert_eq!(move_prog.get_ap_cost(1, &mut tag_bag), 1);
    assert_eq!(move_prog.get_ap_cost(2, &mut tag_bag), 2);
    assert_eq!(move_prog.get_ap_cost(3, &mut tag_bag), 2);
    assert_eq!(move_prog.get_ap_cost(4, &mut tag_bag), 3);
}

#[test]
fn test_tag_discount() {
    let tag1 = TagId(1);
    let mut tag_bag = TagBag::default();
    let registry = TagRegistry {
        defs: {
            let mut m = HashMap::new();
            m.insert(
                tag1,
                TagDef {
                    id: tag1,
                    max_stacks: 1,
                },
            );
            m
        },
    };

    let move_prog = MoveProgram {
        id: MovementId(0),
        name: "Test Move".to_string(),
        steps_ap_cost: vec![(3, 2), (5, 3)],
        vertical_deltas: vec![],
        crosses_holes: false,
        crosses_occupied: false,
        teleport_range: None,
        emit_tags: vec![],
        consume_tags: vec![(tag1, 1, 1)], // Consume 1 stack of tag1 for 1 AP discount
    };

    // No tag, cost should be normal
    assert_eq!(move_prog.get_ap_cost(0, &mut tag_bag), 1);

    // Emit tag
    let mut logger = Logger::new();
    tag_bag.emit(tag1, 1, &registry, &mut logger);
    assert_eq!(tag_bag.counts[&tag1], 1);

    // Now cost should be discounted
    assert_eq!(move_prog.get_ap_cost(0, &mut tag_bag), 0);
    assert_eq!(tag_bag.counts.get(&tag1).copied().unwrap_or(0), 0);
}

#[test]
fn test_modifier_deck() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut deck = AbilityModifierDeck {
        draw_pile: vec![ModifierCard::Critical, ModifierCard::Plus1],
        discard_pile: vec![],
        needs_reshuffle: false,
    };

    let card1 = deck.draw(&mut rng);
    assert!(matches!(
        card1,
        ModifierCard::Critical | ModifierCard::Plus1
    ));
    assert!(!deck.draw_pile.is_empty());

    let _card2 = deck.draw(&mut rng);
    assert!(deck.draw_pile.is_empty());
    assert!(deck.needs_reshuffle);

    deck.end_of_action(&mut rng);
    assert!(!deck.draw_pile.is_empty());
    assert!(!deck.needs_reshuffle);
}

#[test]
fn test_damage_calculation() {
    let attacker = setup_unit(1, 100, 10);
    let defender = setup_unit(2, 100, 10);

    let ability = AbilityDef {
        id: AbilityId(0),
        name: "Test Ability".to_string(),
        ap_cost: 2,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: {
            let mut m = HashMap::new();
            m.insert("STR".to_string(), 1.5);
            m
        },
        range: 1,
        delivery: AbilityDelivery::Melee,
        area_radius: 0,
        programs: RPGPrograms::new(),
    };

    // STR is 10. 10 * 1.5 = 15.0. Plus1 raises total attack power to 16.
    // Only the active STR component remains after component-wise mitigation;
    // AC 1 is normalized to 0.1, so STR mitigation is 1.
    let mut logger = Logger::new();
    let damage = calculate_damage(
        &attacker,
        &defender,
        &ability,
        ModifierCard::Plus1,
        "CON",
        &mut logger,
    );
    assert_eq!(damage, 15);

    // Modifier Critical -> 15 * 2 = 30; final damage is 30 - 1 = 29.
    let damage = calculate_damage(
        &attacker,
        &defender,
        &ability,
        ModifierCard::Critical,
        "CON",
        &mut logger,
    );
    assert_eq!(damage, 29);

    // Modifier Null -> 0
    let damage = calculate_damage(
        &attacker,
        &defender,
        &ability,
        ModifierCard::Null,
        "CON",
        &mut logger,
    );
    assert_eq!(damage, 0);
}

#[test]
fn test_unknown_stat_logging() {
    let attacker = setup_unit(1, 100, 10);
    let defender = setup_unit(2, 100, 10);
    let mut logger = Logger::new();

    let ability = AbilityDef {
        id: AbilityId(0),
        name: "Test Ability".to_string(),
        ap_cost: 2,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: {
            let mut m = HashMap::new();
            m.insert("NONEXISTENT".to_string(), 1.0);
            m
        },
        range: 1,
        delivery: AbilityDelivery::Melee,
        area_radius: 0,
        programs: RPGPrograms::new(),
    };

    let damage = calculate_damage(
        &attacker,
        &defender,
        &ability,
        ModifierCard::Plus0,
        "CON",
        &mut logger,
    );
    assert_eq!(damage, 0);
    assert_eq!(logger.total_errors, 1);
    assert!(logger.get_messages()[0].contains("Unknown attacker attribute: NONEXISTENT"));
}

#[test]
fn timed_modifiers_stack_independently_or_replace_by_policy() {
    let mut unit = setup_unit(1, 100, 10);
    unit.add_timed_modifier(TimedModifier {
        stat: DerivedStat::ArmorClass,
        amount: 3,
        remaining_turns: 2,
        stacking: ModifierStacking::AdditiveIndependent,
    })
    .unwrap();
    unit.add_timed_modifier(TimedModifier {
        stat: DerivedStat::ArmorClass,
        amount: 4,
        remaining_turns: 1,
        stacking: ModifierStacking::AdditiveIndependent,
    })
    .unwrap();
    assert_eq!(unit.stats_with_passives().armor_class, 8);
    unit.advance_owner_turn();
    assert_eq!(unit.stats_with_passives().armor_class, 4);
    unit.add_timed_modifier(TimedModifier {
        stat: DerivedStat::ArmorClass,
        amount: 9,
        remaining_turns: 1,
        stacking: ModifierStacking::RefreshReplace,
    })
    .unwrap();
    assert_eq!(unit.stats_with_passives().armor_class, 10);
}
