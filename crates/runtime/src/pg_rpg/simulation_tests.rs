use super::*;

#[test]
fn available_actions_include_job_names_and_all_abilities() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(4, 0), 0))
        .unwrap();
    scenario.add_secondary_job(1, "Mage").unwrap();
    let simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );

    let actions = simulation.get_available_actions(1).unwrap();
    assert!(!actions.movement.is_empty());
    assert_eq!(actions.primary_job.name, "Caveman");
    assert!(
        actions
            .primary_job
            .abilities
            .iter()
            .any(|ability| ability.name == "Club Smash")
    );
    assert_eq!(actions.secondary_jobs[0].name, "Mage");
    assert!(
        actions.secondary_jobs[0]
            .abilities
            .iter()
            .any(|ability| ability.name == "Fireball")
    );
}

#[test]
fn boundary_exposes_npc_before_its_typed_decision() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 1, "Mage", GridCell::new(hexx::Hex::new(1, -1), 0))
        .unwrap();
    scenario
        .add_unit(3, 2, "Necromancer", GridCell::new(hexx::Hex::new(5, -5), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            visits: 1,
            depth: 1,
            seed: Some(42),
            ..Default::default()
        },
    );

    let first = simulation.advance_to_boundary().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(simulation.state.agents[&first[0]].team_id, 1);
    simulation.commit_wait(first[0].0 as u64).unwrap();

    let second = simulation.advance_to_boundary().unwrap();
    assert_eq!(second.len(), 1);
    assert_ne!(first[0], second[0]);
    assert_eq!(simulation.state.agents[&second[0]].team_id, 2);
    let decision = simulation.wait_decision(second[0]);
    simulation.apply_npc_action(second[0], decision).unwrap();
    let third = simulation.advance_to_boundary().unwrap();
    assert_eq!(simulation.state.agents[&third[0]].team_id, 1);
}

#[test]
fn budgeted_boundary_preserves_progress_between_slices() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(4, 0), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );

    assert_eq!(simulation.advance_to_boundary_budgeted(0).unwrap(), None);
    let mut boundary = None;
    for _ in 0..128 {
        if let Some(ready) = simulation.advance_to_boundary_budgeted(1).unwrap() {
            boundary = Some(ready);
            break;
        }
    }
    let boundary = boundary.expect("bounded scheduler must eventually reach a boundary");
    assert!(!boundary.is_empty());
    assert!(
        boundary
            .iter()
            .all(|agent| simulation.state.agents.contains_key(agent))
    );
}

#[test]
fn npc_action_is_planned_and_revalidated_before_application() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(4, 0), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            visits: 1,
            depth: 1,
            seed: Some(42),
            ..Default::default()
        },
    );

    let action = simulation
        .request_npc_decision(AgentId(2))
        .unwrap_or_else(|| simulation.wait_decision(AgentId(2)));
    let applied = simulation
        .apply_npc_action(AgentId(2), action.clone())
        .unwrap();

    assert_eq!(applied, action);
    assert!(simulation.state.agents[&AgentId(2)].action_points <= 4);
}

#[test]
fn npc_planning_never_selects_an_ordinary_action_over_a_pending_reaction() {
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
            visits: 24,
            depth: 6,
            seed: Some(42),
            ..Default::default()
        },
    );
    simulation
        .state
        .reaction_queue
        .push((AgentId(2), ReactionId(101), AgentId(1)));

    assert_eq!(
        simulation.request_npc_decision(AgentId(2)),
        Some(TacticalDisplayAction::Reaction {
            reaction: ReactionId(101),
            target: AgentId(1),
        })
    );
}

#[test]
fn authoritative_action_boundary_resolves_reaction_before_ordinary_candidate() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, -1), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            seed: Some(42),
            visits: 1,
            depth: 1,
            ..Default::default()
        },
    );
    let agent = AgentId(2);
    let target = AgentId(1);
    let fireball = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Fireball")
        .unwrap()
        .id;
    let reaction = simulation.state.agents[&agent].reaction_abilities[0];
    simulation
        .state
        .reaction_queue
        .push((agent, reaction, target));

    let action = TacticalDisplayAction::Ability {
        target,
        ability: fireball,
    };
    assert_eq!(
        simulation.apply_npc_action(agent, action.clone()),
        Ok(action)
    );
    assert!(
        !simulation
            .state
            .reaction_queue
            .iter()
            .any(|(reaction_agent, _, _)| *reaction_agent == agent)
    );
}

#[test]
fn adjacent_npc_attack_is_legal_revalidated_and_damages_player() {
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
            visits: 50,
            depth: 5,
            seed: Some(42),
            ..Default::default()
        },
    );
    simulation
        .state
        .agents
        .get_mut(&AgentId(1))
        .unwrap()
        .stats
        .armor_class = 0;
    simulation.state.agents.get_mut(&AgentId(1)).unwrap().health = 1;
    let health_before = simulation.state.agents[&AgentId(1)].health;

    let action = simulation
        .request_npc_decision(AgentId(2))
        .expect("adjacent NPC should have a legal action");
    assert!(
        matches!(
            action,
            TacticalDisplayAction::Ability {
                target: AgentId(1),
                ..
            }
        ),
        "unexpected adjacent NPC action: {action:?}"
    );
    simulation
        .apply_npc_action(AgentId(2), action)
        .expect("the root candidate must revalidate");

    assert!(simulation.state.agents[&AgentId(1)].health < health_before);
}

#[test]
fn secondary_job_ability_is_generated_and_revalidated() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario.add_secondary_job(1, "Mage").unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let fireball = scenario
        .build_state()
        .unwrap()
        .ability_registry
        .values()
        .find(|ability| ability.name == "Fireball")
        .map(|ability| ability.id)
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            visits: 1,
            depth: 1,
            seed: Some(42),
            ..Default::default()
        },
    );
    simulation
        .state
        .agents
        .get_mut(&AgentId(2))
        .unwrap()
        .stats
        .armor_class = 0;
    let action = TacticalDisplayAction::Ability {
        target: AgentId(2),
        ability: fireball,
    };
    let health_before = simulation.state.agents[&AgentId(2)].health;

    simulation
        .apply_npc_action(AgentId(1), action.clone())
        .expect("secondary-job ability should be a legal root task");

    assert_eq!(simulation.state.agents[&AgentId(1)].action_points, 1);
    assert!(simulation.state.agents[&AgentId(2)].health < health_before);
}

#[test]
fn rejected_npc_action_reports_snapshot_actor_and_legal_actions_without_mutation() {
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
            visits: 1,
            depth: 1,
            seed: Some(42),
            ..Default::default()
        },
    );
    let before = simulation.state.clone();
    let error = simulation
        .apply_npc_action(
            AgentId(2),
            TacticalDisplayAction::Ability {
                target: AgentId(2),
                ability: AbilityId(9),
            },
        )
        .expect_err("self-targeted melee candidate must be rejected");

    assert!(error.contains("agent 2"));
    assert!(error.contains("snapshot="));
    assert!(error.contains("legal_actions="));
    assert_eq!(simulation.state.agents, before.agents);
}

#[test]
fn diverse_headless_choices_preserve_board_invariants_and_complete() {
    let positions = [
        (hexx::Hex::ZERO, hexx::Hex::new(1, 0)),
        (hexx::Hex::ZERO, hexx::Hex::new(4, 0)),
        (hexx::Hex::new(1, -1), hexx::Hex::new(5, -5)),
        (hexx::Hex::new(-2, 1), hexx::Hex::new(2, -2)),
    ];

    for (case, (player_position, npc_position)) in positions.into_iter().enumerate() {
        let mut scenario = SkirmishConfig::new(100 + case as u64);
        scenario.set_maximum_turn_count(3).unwrap();
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(player_position, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(npc_position, 0))
            .unwrap();
        let mut simulation = TacticalSimulation::from_scenario(
            scenario,
            MCTSConfiguration {
                visits: 4,
                depth: 2,
                seed: Some(100 + case as u64),
                ..Default::default()
            },
        );

        for _ in 0..64 {
            if simulation.is_complete() {
                break;
            }
            let ready = simulation.advance_to_boundary().unwrap();
            assert_eq!(ready.len(), 1);
            let agent = ready[0];
            if simulation.state.agents[&agent].team_id == 1 {
                if case % 2 == 0 {
                    if let Some((destination, _)) = reachable_cells(&simulation.state, agent)
                        .unwrap()
                        .into_iter()
                        .next()
                    {
                        simulation.commit_move(agent.0 as u64, destination).unwrap();
                    }
                }
                simulation.commit_wait(agent.0 as u64).unwrap();
            } else {
                for _ in 0..8 {
                    if simulation.is_complete() {
                        break;
                    }
                    let action = simulation
                        .request_npc_decision(agent)
                        .unwrap_or(TacticalDisplayAction::Wait);
                    let applied = simulation.apply_npc_action(agent, action).unwrap();
                    if matches!(applied, TacticalDisplayAction::Wait) {
                        break;
                    }
                }
            }

            let occupied = simulation
                .state
                .agents
                .values()
                .filter(|unit| unit.health > 0)
                .map(|unit| unit.position)
                .collect::<std::collections::HashSet<_>>();
            let living_units = simulation
                .state
                .agents
                .values()
                .filter(|unit| unit.health > 0)
                .collect::<Vec<_>>();
            assert_eq!(occupied.len(), living_units.len(), "case {case}");
            assert!(
                living_units
                    .iter()
                    .all(|unit| simulation.state.grid.contains(unit.position))
            );
        }

        assert!(simulation.is_complete(), "case {case} did not complete");
    }
}

#[test]
fn npc_off_board_move_is_rejected_without_mutating_position() {
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
            visits: 1,
            depth: 1,
            seed: Some(42),
            ..Default::default()
        },
    );
    let before = simulation.state.agents[&AgentId(2)].position;
    assert!(
        simulation
            .apply_npc_action(
                AgentId(2),
                TacticalDisplayAction::Move {
                    to: GridCell::new(hexx::Hex::new(99, 99), 0),
                },
            )
            .is_err()
    );
    assert_eq!(simulation.state.agents[&AgentId(2)].position, before);
}

#[test]
fn commit_move_validates_destination_before_mutating_state() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let destination = GridCell::new(hexx::Hex::new(1, 0), 0);

    let validated = simulation.commit_move(1, destination).unwrap();
    assert_eq!(validated.destination, destination);
    assert_eq!(simulation.state.agents[&AgentId(1)].position, destination);
    assert_eq!(simulation.state.agents[&AgentId(1)].action_points, 3);

    let rejected = simulation.commit_move(1, GridCell::new(hexx::Hex::new(20, 20), 0));
    assert_eq!(
        rejected,
        Err(ActionError::IllegalDestination(GridCell::new(
            hexx::Hex::new(20, 20),
            0
        )))
    );
}

#[test]
fn completion_is_detected_and_stops_future_steps() {
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
    simulation.state.agents.get_mut(&AgentId(2)).unwrap().health = 0;

    assert!(simulation.is_complete());
    assert_eq!(simulation.winning_team(), Some(1));
    assert!(simulation.advance_to_boundary().unwrap().is_empty());
}

#[test]
fn npc_wait_remains_legal_when_another_unit_has_a_pending_reaction() {
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
    simulation
        .state
        .reaction_queue
        .push((AgentId(1), ReactionId(101), AgentId(2)));

    assert_eq!(
        simulation.apply_npc_action(AgentId(2), TacticalDisplayAction::Wait),
        Ok(TacticalDisplayAction::Wait)
    );
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
