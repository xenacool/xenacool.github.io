#[test]
fn deterministic_startup_step_produces_continuation() {
    // This is the offline equivalent of the first two simulation-worker
    // messages: StartPgRpgSimulation has established the session, then one
    // StepPgRpgSimulation crosses the script boundary. It isolates runtime
    // state progression from Gloo worker startup and bridge delivery.
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
    assert_eq!(runtime.continuation(), RuntimeContinuation::AwaitBoundary);

    let response = (0..16)
        .map(|_| {
            runtime
                .process_request(RuntimeRequest::StepPgRpgSimulation)
                .0
        })
        .find(|response| !matches!(response, RuntimeResponse::SimulationProgress { .. }))
        .expect("budgeted startup should reach a boundary within 16 slices");

    assert!(matches!(
        response,
        RuntimeResponse::PgRpgSimulationStepped(_)
    ));
    assert!(
        matches!(
            runtime.continuation(),
            RuntimeContinuation::AwaitPlayerDecision { .. }
                | RuntimeContinuation::AwaitMctsDecision { .. }
        ),
        "response={response:?}, continuation={:?}",
        runtime.continuation()
    );
}
