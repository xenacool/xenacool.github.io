use super::*;
use crate::demo::simulation::TacticalSimulation;
use pystral_core::history::HistoryManager;
use pystral_core::log::Event;
use pystral_games::{GridCell, SkirmishConfig};

fn attach_rhai_session(runtime: &mut Runtime) {
    let history = runtime.demo_history.clone().unwrap_or_default();
    let simulation = runtime.demo_sim.clone().expect("test simulation");
    runtime.rhai_session = Some(
        crate::rhai_session::RhaiSession::new(
            include_str!("../../../web/scripts/actions/demo_loop.rhai"),
            history,
            String::new(),
            Vec::new(),
            1,
        )
        .map(|mut session| {
            session.set_simulation(simulation);
            session
        })
        .expect("test Rhai session"),
    );
}

#[test]
fn production_shaped_four_unit_playout_reaches_completion_past_history_379() {
    let mut scenario = SkirmishConfig::new(42);
    scenario.set_maximum_turn_count(12).unwrap();
    for (id, team, job, hex) in [
        (1, 1, "Caveman", hexx::Hex::new(0, 0)),
        (2, 1, "Mage", hexx::Hex::new(1, -1)),
        (3, 2, "Necromancer", hexx::Hex::new(5, -5)),
        (4, 2, "Skeleton_Minion", hexx::Hex::new(4, -4)),
    ] {
        scenario
            .add_unit(id, team, job, GridCell::new(hex, 0))
            .unwrap();
    }
    scenario.add_secondary_job(1, "Mage").unwrap();

    let mut runtime = Runtime::new();
    runtime.demo_sim = Some(TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            visits: 4,
            depth: 2,
            seed: Some(42),
            ..Default::default()
        },
    ));
    runtime.demo_history = Some(HistoryManager::new());
    attach_rhai_session(&mut runtime);

    let mut response = runtime
        .process_request(RuntimeRequest::StepDemoSimulation)
        .0;
    let mut max_history_len = 0;
    for step in 0..512u64 {
        max_history_len = max_history_len.max(runtime.demo_history.as_ref().unwrap().log.len());
        while matches!(response, RuntimeResponse::SimulationProgress { .. }) {
            response = runtime
                .process_request(RuntimeRequest::StepDemoSimulation)
                .0;
        }
        match runtime.continuation.clone() {
            RuntimeContinuation::AwaitPlayerDecision { unit_id } => {
                let committed = runtime
                    .process_request(RuntimeRequest::CommitWait {
                        request_id: 10_000 + step,
                        unit_id,
                    })
                    .0;
                let barrier_id = match committed {
                    RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
                    other => panic!("player wait failed at step {step}: {other:?}"),
                };
                runtime.process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id });
                response = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
            }
            RuntimeContinuation::AwaitMctsDecision {
                request_id,
                unit_id,
                state_version,
            } => {
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
                    panic!("MCTS request failed at step {step}: {ready:?}");
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
                    other => panic!("NPC action failed at step {step}: {other:?}"),
                };
                runtime.process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id });
                response = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
            }
            RuntimeContinuation::Completed => break,
            continuation => panic!("stalled at step {step} with {continuation:?}"),
        }
        if matches!(response, RuntimeResponse::GameCompleted { .. }) {
            break;
        }
    }

    assert!(
        max_history_len > 379,
        "playout did not exercise history 379"
    );
    assert_eq!(runtime.continuation, RuntimeContinuation::Completed);
    let history = runtime.demo_history.as_ref().expect("demo history");
    assert_eq!(
        history
            .log
            .iter()
            .filter(|event| matches!(event, Event::GameCompleted { .. }))
            .count(),
        1
    );
    assert!(matches!(response, RuntimeResponse::GameCompleted { .. }));
}
