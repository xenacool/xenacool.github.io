use super::*;

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
