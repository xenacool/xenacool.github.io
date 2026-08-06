use pystral_games::*;
use std::collections::{HashMap, BTreeMap};
use pystral_core::ui_log::Logger;

#[test]
fn test_mcts_sampling_determinism() {
    // 1. Setup initial state with a seeded RNG
    let mut state = TacticalState {
        agents: HashMap::new(),
        grid: TacticalGrid::default(),
        collision: None,
        logger: Logger::new(),
        reaction_queue: Vec::new(),
        rng: SeededRng::new(42),
    };

    let agent_id = AgentId(1);
    let mut unit = UnitState {
        health: 100,
        mana: 100,
        action_points: 4,
        ct: 0,
        position: (0, 0, 0),
        gender: Gender::Nonbinary,
        class_id: ActorClassId(1),
        primary_job: JobId(1),
        secondary_jobs: vec![],
        movement_ability: None,
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
            armor_class: 1,
            speed: 10,
        },
        derived_stats: DerivedStats {
            health_max: 100,
            mana_max: 100,
            action_points_max: 4,
        },
        equipment: EquipmentSlots {
            slots: HashMap::new(),
        },
        status_effects: Vec::new(),
        modifier_deck: AbilityModifierDeck {
            draw_pile: vec![ModifierCard::Plus1, ModifierCard::Minus1, ModifierCard::Critical],
            discard_pile: vec![],
            needs_reshuffle: false,
        },
    };
    state.agents.insert(agent_id, unit);

    // 2. Clone the state to simulate MCTS branching
    let mut state_branch_a = state.clone();
    let mut state_branch_b = state.clone();

    // 3. Perform a "random" draw in both branches
    // We'll simulate what a Task::execute would do
    let mut draw_random_card = |s: &mut TacticalState| {
        let agent = s.agents.get_mut(&agent_id).unwrap();
        // Force a situation where RNG MUST be used: empty draw pile, cards in discard pile
        if agent.modifier_deck.draw_pile.is_empty() && agent.modifier_deck.discard_pile.is_empty() {
             agent.modifier_deck.discard_pile = vec![ModifierCard::Plus1, ModifierCard::Minus1, ModifierCard::Critical];
        } else if !agent.modifier_deck.draw_pile.is_empty() {
             let mut cards = agent.modifier_deck.draw_pile.drain(..).collect::<Vec<_>>();
             agent.modifier_deck.discard_pile.append(&mut cards);
        }
        
        let _card = agent.modifier_deck.draw(&mut s.rng);
        
        // Return a diff to satisfy the pattern
        TacticalDiff {
            health_changes: BTreeMap::new(),
            position_changes: BTreeMap::new(),
            rng_update: Some(s.rng.clone()),
        }
    };

    let diff_a = draw_random_card(&mut state_branch_a);
    let diff_b = draw_random_card(&mut state_branch_b);

    // 4. Verify that both branches got the SAME result because they started with the same seed
    // This is the core of "preventing agents from cheating" - the outcome is fixed for a given state.
    assert_eq!(diff_a.rng_update, diff_b.rng_update);
    
    let card_a = state_branch_a.agents[&agent_id].modifier_deck.discard_pile.last().unwrap();
    let card_b = state_branch_b.agents[&agent_id].modifier_deck.discard_pile.last().unwrap();
    assert_eq!(card_a, card_b);

    // 5. Verify that subsequent draws are also deterministic but different from the first
    let diff_a2 = draw_random_card(&mut state_branch_a);
    let diff_b2 = draw_random_card(&mut state_branch_b);
    assert_eq!(diff_a2.rng_update, diff_b2.rng_update);
    assert_ne!(diff_a.rng_update, diff_a2.rng_update);
}
