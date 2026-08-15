#[test]
fn repeated_player_npc_handoffs_reach_completion_exactly_once() {
    let mut scenario = SkirmishConfig::new(42);
    scenario.set_maximum_turn_count(2).unwrap();
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
            visits: 1,
            depth: 1,
            seed: Some(42),
            ..Default::default()
        },
    ));
    runtime.pg_rpg_history = Some(HistoryManager::new());
    attach_rhai_session(&mut runtime);

    let mut response = runtime
        .process_request(RuntimeRequest::StepPgRpgSimulation)
        .0;
    let mut player_turns = 0;
    let mut npc_turns = 0;
    let mut acknowledgments = 0;
    for _ in 0..128 {
        while matches!(runtime.continuation, RuntimeContinuation::AwaitBoundary) {
            response = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
            if !matches!(response, RuntimeResponse::SimulationProgress { .. }) {
                break;
            }
        }
        match runtime.continuation.clone() {
            RuntimeContinuation::AwaitPlayerDecision { unit_id } => {
                player_turns += 1;
                let committed = runtime
                    .process_request(RuntimeRequest::CommitWait {
                        request_id: 800 + player_turns,
                        unit_id,
                    })
                    .0;
                let barrier_id = match committed {
                    RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
                    other => panic!("expected player Wait commit, got {other:?}"),
                };
                assert!(matches!(
                    runtime
                        .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
                        .0,
                    RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
                ));
                acknowledgments += 1;
                response = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
            }
            RuntimeContinuation::AwaitMctsDecision {
                request_id,
                unit_id,
                state_version,
            } => {
                npc_turns += 1;
                let ready = runtime
                    .process_request(RuntimeRequest::RequestMctsDecision {
                        request_id,
                        unit_id,
                        state_version,
                    })
                    .0;
                let RuntimeResponse::MctsDecisionReady {
                    request_id,
                    decision,
                    state_version,
                } = ready
                else {
                    panic!("expected MCTS decision");
                };
                let committed = runtime
                    .process_request(RuntimeRequest::MctsDecisionReady {
                        request_id,
                        decision,
                        state_version,
                    })
                    .0;
                let barrier_id = match committed {
                    RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
                    other => panic!("expected NPC action commit, got {other:?}"),
                };
                runtime.process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id });
                acknowledgments += 1;
                response = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
            }
            RuntimeContinuation::Completed => break,
            continuation => panic!("unexpected handoff continuation: {continuation:?}"),
        }
    }

    assert!(player_turns >= 2);
    assert!(npc_turns >= 2);
    assert_eq!(acknowledgments, player_turns + npc_turns);
    assert!(matches!(response, RuntimeResponse::GameCompleted { .. }));
    let completion_count = runtime
        .pg_rpg_history
        .as_ref()
        .unwrap()
        .log
        .iter()
        .filter(|event| matches!(event, Event::GameCompleted { .. }))
        .count();
    assert_eq!(completion_count, 1);
    assert!(matches!(
        runtime.continuation,
        RuntimeContinuation::Completed
    ));
}

#[test]
fn empty_simulation_completes_as_draw() {
    let mut runtime = Runtime::new();
    runtime.pg_rpg_sim = Some(TacticalSimulation::from_scenario(
        SkirmishConfig::new(42),
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    ));
    runtime.pg_rpg_history = Some(HistoryManager::new());
    attach_rhai_session(&mut runtime);

    let response = runtime
        .process_request(RuntimeRequest::StepPgRpgSimulation)
        .0;
    assert!(matches!(
        response,
        RuntimeResponse::GameCompleted {
            outcome: GameOutcome::Draw,
            ..
        }
    ));
}
