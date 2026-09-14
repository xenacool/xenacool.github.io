use super::*;

#[test]
fn reaction_interrupt_does_not_complete_the_responders_ordinary_turn() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let responder = AgentId(2);
    let reaction = simulation.state.agents[&responder].reaction_abilities[0];
    simulation
        .state
        .reaction_queue
        .push((responder, reaction, AgentId(1)));

    simulation
        .apply_npc_action(
            responder,
            TacticalDisplayAction::Reaction {
                reaction,
                target: AgentId(1),
            },
        )
        .expect("queued reaction is legal");

    assert!(simulation.state.reaction_queue.is_empty());
    assert!(!simulation.completed_turns.contains(&responder));
    assert_eq!(simulation.completed_rounds, 0);
}

#[test]
fn tactical_npc_adapter_rejects_a_poor_mcts_root_choice() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            visits: 5_000,
            depth: 10,
            seed: Some(42),
            ..Default::default()
        },
    );
    simulation.state.agents.get_mut(&AgentId(2)).unwrap().health = 1;
    simulation
        .state
        .agents
        .get_mut(&AgentId(2))
        .unwrap()
        .stats
        .armor_class = 0;

    assert!(matches!(
        simulation.request_npc_decision(AgentId(1)),
        Some(TacticalDisplayAction::Ability {
            target: AgentId(2),
            ..
        })
    ));
}
