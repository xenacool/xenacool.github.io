use games::*;
use std::collections::HashMap;
use rand::SeedableRng;
use rand::rngs::StdRng;
use pystral_core::ui_log::Logger;

fn setup_unit(_id: u64, speed: i32, wits: i32) -> UnitState {
    UnitState {
        health: 100,
        mana: 100,
        action_points: 4,
        ct: 0,
        position: (0, 0, 0),
        gender: Gender::Nonbinary,
        class_id: ActorClassId(1),
        primary_job: JobId(1),
        secondary_job: None,
        movement_ability: None,
        passive_abilities: Vec::new(),
        reaction_ability: None,
        stats: UnitStats {
            health_max: 100,
            mana_max: 100,
            action_points_max: 4,
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
        modifier_deck: AbilityModifierDeck::default(),
    }
}

#[test]
fn test_ct_scheduler() {
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::new(),
    };

    let id1 = AgentId(1);
    let id2 = AgentId(2);

    state.agents.insert(id1, setup_unit(1, 100, 10)); // Ready in 10 ticks
    state.agents.insert(id2, setup_unit(2, 50, 20));  // Ready in 20 ticks

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
    
    // Check sorting by WITS (id2 has 20, id1 has 10)
    assert_eq!(ready, vec![id2, id1]);
}

#[test]
fn test_ap_movement_costs() {
    let mut tag_bag = TagBag::default();
    let move_prog = MoveProgram {
        steps_ap_cost: vec![(3, 2), (5, 3)], // 1-2: 1AP, 3-4: 2AP, 5+: 3AP
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
            m.insert(tag1, TagDef { id: tag1, max_stacks: 1 });
            m
        }
    };

    let move_prog = MoveProgram {
        steps_ap_cost: vec![(3, 2), (5, 3)],
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
    assert!(matches!(card1, ModifierCard::Critical | ModifierCard::Plus1));
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
        ap_cost: 2,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: {
            let mut m = HashMap::new();
            m.insert("STR".to_string(), 1.5);
            m
        },
    };

    // STR is 10. 10 * 1.5 = 15.0
    // Modifier Plus1 -> 15 + 1 = 16
    // Defender CON is 10, Armor Class is 1. Mitigation = 10 * 1 = 10
    // Final damage = 16 - 10 = 6
    let mut logger = Logger::new();
    let damage = calculate_damage(&attacker, &defender, &ability, ModifierCard::Plus1, "CON", &mut logger);
    assert_eq!(damage, 6);

    // Modifier Critical -> 15 * 2 = 30
    // Final damage = 30 - 10 = 20
    let damage = calculate_damage(&attacker, &defender, &ability, ModifierCard::Critical, "CON", &mut logger);
    assert_eq!(damage, 20);

    // Modifier Null -> 0
    let damage = calculate_damage(&attacker, &defender, &ability, ModifierCard::Null, "CON", &mut logger);
    assert_eq!(damage, 0);
}

#[test]
fn test_unknown_stat_logging() {
    let attacker = setup_unit(1, 100, 10);
    let defender = setup_unit(2, 100, 10);
    let mut logger = Logger::new();

    let ability = AbilityDef {
        ap_cost: 2,
        emit_tags: vec![],
        consume_tags: vec![],
        scaling: {
            let mut m = HashMap::new();
            m.insert("NONEXISTENT".to_string(), 1.0);
            m
        },
    };

    let damage = calculate_damage(&attacker, &defender, &ability, ModifierCard::Plus0, "CON", &mut logger);
    assert_eq!(damage, 0);
    assert_eq!(logger.total_errors, 1);
    assert!(logger.get_messages()[0].contains("Unknown attacker attribute: NONEXISTENT"));
}
