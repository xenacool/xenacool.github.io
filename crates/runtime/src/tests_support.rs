fn attach_rhai_session(runtime: &mut Runtime) {
    let history = runtime.pg_rpg_history.clone().unwrap_or_default();
    let simulation = runtime.pg_rpg_sim.clone().expect("test simulation");
    runtime.rhai_session = Some(
        crate::rhai_session::RhaiSession::from_simulation(history, simulation)
            .expect("test Rhai session"),
    );
}

fn runtime_with_unit() -> Runtime {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
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
    attach_rhai_session(&mut runtime);
    runtime
}
