use super::*;

#[test]
fn ability_decision_requires_strict_target_provenance() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 52,
            decision: RuntimeDecision {
                unit_id: 1,
                action: RuntimeDecisionAction::Ability {
                    ability_id: 101,
                    target: RuntimeAbilityTarget::Unit { unit_id: 2 },
                },
            },
            provenance: None,
        })
        .0;
    assert!(
        matches!(response, RuntimeResponse::Error(message) if message.contains("Missing ability target provenance"))
    );
}

#[test]
fn action_input_is_routed_without_history_mutation() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::ActionInput {
            direction: "confirm".into(),
        })
        .0;
    assert!(
        matches!(response, RuntimeResponse::ActionInputRouted(direction) if direction == "confirm")
    );
    assert!(runtime.pg_rpg_history.as_ref().unwrap().log.is_empty());
}

#[test]
fn move_preview_returns_reachable_cells_without_history_mutation() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::OpenMovePreview {
            request_id: 21,
            unit_id: 1,
        })
        .0;
    assert!(
        matches!(response, RuntimeResponse::MovePreview { request_id: 21, unit_id: 1, source, reachable, selected_destination: Some(_) } if source.hex == hexx::Hex::ZERO && !reachable.is_empty())
    );
    assert!(runtime.pg_rpg_history.as_ref().unwrap().log.is_empty());
}

#[test]
fn occupied_destination_rejects_stale_preview_without_history_mutation() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(4, 0), 0))
        .unwrap();
    let mut runtime = Runtime::new();
    runtime.pg_rpg_sim = Some(TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    ));
    runtime.pg_rpg_history = Some(HistoryManager::new());

    let preview = runtime
        .process_request(RuntimeRequest::OpenMovePreview {
            request_id: 30,
            unit_id: 1,
        })
        .0;
    let destination = match preview {
        RuntimeResponse::MovePreview { reachable, .. } => reachable
            .into_iter()
            .find(|cell| cell.hex == hexx::Hex::new(1, 0))
            .expect("expected adjacent destination"),
        other => panic!("expected preview, got {other:?}"),
    };
    runtime
        .pg_rpg_sim
        .as_mut()
        .unwrap()
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(2))
        .unwrap()
        .position = GridCell::new(destination.hex, destination.layer);

    let rejected = runtime
        .process_request(RuntimeRequest::CommitMove {
            request_id: 31,
            preview_request_id: 30,
            unit_id: 1,
            hex: destination.hex,
            layer: destination.layer,
        })
        .0;
    assert!(
        matches!(rejected, RuntimeResponse::ActionRejected { request_id: 31, reason: ActionError::IllegalDestination(cell) } if cell == GridCell::new(destination.hex, destination.layer))
    );
    assert!(runtime.pg_rpg_history.as_ref().unwrap().log.is_empty());
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)].position,
        GridCell::new(hexx::Hex::ZERO, 0)
    );
}
