use super::*;
use crate::pg_rpg::simulation::TacticalSimulation;
use proptest::prelude::*;
use pystral_core::log::{Event, GameOutcome};
use pystral_games::{GridCell, SkirmishConfig};

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

#[test]
fn commit_move_returns_history_delta_but_rejection_does_not() {
    let mut runtime = runtime_with_unit();
    let initial_ct =
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)].ct;
    let accepted = runtime
        .process_request(RuntimeRequest::CommitMove {
            request_id: 11,
            preview_request_id: 10,
            unit_id: 1,
            hex: hexx::Hex::new(1, 0),
            layer: 0,
        })
        .0;
    let (barrier_id, accepted_history) = match accepted {
        RuntimeResponse::ActionCommitted {
            request_id: 11,
            barrier_id,
            history,
            ..
        } => (barrier_id, history),
        other => panic!("expected committed move, got {other:?}"),
    };
    assert_eq!(accepted_history.log.len(), 3);
    assert!(
        matches!(accepted_history.log[0], Event::MoveSprite { id: 1, destination, transition: Some(ref transition) } if destination == hexx::Hex::new(1, 0) && transition.duration_ms == 500 && transition.delta_time_ms == 16.0 && transition.tween == pystral_core::log::TweenKind::SineInOut)
    );
    assert!(
        matches!(accepted_history.log[1], Event::UpdateProperty { id: 1, ref property, value: pystral_core::log::PropertyValue::Float(layer) } if property == "layer" && layer == 0.0)
    );
    assert_eq!(runtime.pg_rpg_history.as_ref().unwrap().log.len(), 3);
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)].ct,
        initial_ct
    );
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)]
            .action_points,
        3
    );
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
    ));

    let rejected = runtime
        .process_request(RuntimeRequest::CommitMove {
            request_id: 12,
            preview_request_id: 10,
            unit_id: 1,
            hex: hexx::Hex::new(20, 20),
            layer: 0,
        })
        .0;
    assert!(matches!(
        rejected,
        RuntimeResponse::ActionRejected { request_id: 12, .. }
    ));
    assert_eq!(runtime.pg_rpg_history.as_ref().unwrap().log.len(), 3);
}

#[test]
fn wait_commits_turn_boundary_without_preview_state() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::CommitWait {
            request_id: 41,
            unit_id: 1,
        })
        .0;
    let history = match response {
        RuntimeResponse::ActionCommitted {
            request_id: 41,
            history,
            ..
        } => history,
        other => panic!("expected committed wait, got {other:?}"),
    };
    assert!(matches!(
        history.log.first(),
        Some(Event::TurnCompleted { unit_id: 1 })
    ));
    assert!(
        matches!(history.log.get(1), Some(Event::UnitStateChanged { unit_id: 1, action_points, .. }) if *action_points > 0)
    );
    assert!(matches!(history.log.get(2), Some(Event::SequenceNumber(_))));
    assert_eq!(runtime.pg_rpg_history.as_ref().unwrap().log.len(), 3);
}

#[test]
fn typed_wait_decision_uses_the_existing_commit_path() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 51,
            decision: RuntimeDecision {
                unit_id: 1,
                action: RuntimeDecisionAction::Wait,
            },
            provenance: None,
        })
        .0;
    assert!(
        matches!(response, RuntimeResponse::ActionCommitted { request_id: 51, action, .. } if action == "wait")
    );
    assert!(matches!(
        runtime.continuation,
        RuntimeContinuation::AwaitAnimationAck { .. }
    ));
}

#[test]
fn animation_acknowledgment_advances_the_continuation_exactly_once() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::CommitWait {
            request_id: 61,
            unit_id: 1,
        })
        .0;
    let barrier_id = match response {
        RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
        other => panic!("expected committed wait, got {other:?}"),
    };

    let acknowledged = runtime
        .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
        .0;
    assert!(matches!(
        acknowledged,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
    ));

    let duplicate = runtime
        .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
        .0;
    assert!(
        matches!(duplicate, RuntimeResponse::Error(message) if message.contains("without a pending barrier"))
    );
}

#[test]
fn stale_animation_acknowledgment_does_not_advance_or_mutate_state() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::CommitWait {
            request_id: 62,
            unit_id: 1,
        })
        .0;
    let barrier_id = match response {
        RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
        other => panic!("expected committed wait, got {other:?}"),
    };

    let stale = runtime
        .process_request(RuntimeRequest::AcknowledgeAnimation {
            barrier_id: barrier_id.saturating_sub(1),
        })
        .0;
    assert!(matches!(stale, RuntimeResponse::Error(message) if message.contains("expected")));
    assert!(matches!(
        runtime.continuation,
        RuntimeContinuation::AwaitAnimationAck {
            barrier_id: expected,
            ..
        } if expected == barrier_id
    ));
    assert_eq!(runtime.pg_rpg_history.as_ref().unwrap().log.len(), 3);
}

#[test]
fn duplicate_boundary_resume_is_rejected_without_advancing_again() {
    let mut runtime = runtime_with_unit();
    let response = runtime
        .process_request(RuntimeRequest::CommitWait {
            request_id: 63,
            unit_id: 1,
        })
        .0;
    let barrier_id = match response {
        RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
        other => panic!("expected committed wait, got {other:?}"),
    };
    runtime.process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id });

    let first = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
    assert!(!matches!(first, RuntimeResponse::Error(_)));
    let second = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
    assert!(matches!(second, RuntimeResponse::Error(_)));
}

#[test]
fn completion_emits_typed_draw_once_at_the_configured_round_limit() {
    let mut scenario = SkirmishConfig::new(42);
    scenario.set_maximum_turn_count(1).unwrap();
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(4, 0), 0))
        .unwrap();
    let mut runtime = Runtime::new();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    simulation.record_completed_turn(npc_engine_core::AgentId(1));
    simulation.record_completed_turn(npc_engine_core::AgentId(2));
    runtime.pg_rpg_sim = Some(simulation);
    runtime.pg_rpg_history = Some(HistoryManager::new());
    attach_rhai_session(&mut runtime);

    let response = runtime
        .process_request(RuntimeRequest::StepPgRpgSimulation)
        .0;
    let history = match response {
        RuntimeResponse::GameCompleted { outcome, history } => {
            assert_eq!(outcome, GameOutcome::Draw);
            history
        }
        other => panic!("expected typed completion, got {other:?}"),
    };
    assert_eq!(
        history
            .log
            .iter()
            .filter(|event| matches!(event, Event::GameCompleted { .. }))
            .count(),
        1
    );

    let (response, logs) = runtime.process_request(RuntimeRequest::StepPgRpgSimulation);
    assert!(
        matches!(response, RuntimeResponse::Error(message) if message.contains("after game completion"))
    );
    assert_eq!(logs.len(), 1);
}

fn terminal_ability_fixture(
    attacker_job: &str,
    defender_job: &str,
    attacker_id: u64,
    defender_id: u64,
) -> (Runtime, u64) {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(
            attacker_id as i64,
            if attacker_id == 1 { 1 } else { 2 },
            attacker_job,
            GridCell::new(hexx::Hex::ZERO, 0),
        )
        .unwrap();
    scenario
        .add_unit(
            defender_id as i64,
            if defender_id == 1 { 1 } else { 2 },
            defender_job,
            GridCell::new(hexx::Hex::new(1, 0), 0),
        )
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
    runtime
        .pg_rpg_sim
        .as_mut()
        .unwrap()
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(defender_id as u32))
        .unwrap()
        .health = 1;
    attach_rhai_session(&mut runtime);
    let ability_name = if attacker_job == "Mage" {
        "Fireball"
    } else {
        "Club Smash"
    };
    let ability_id = runtime
        .pg_rpg_sim
        .as_ref()
        .unwrap()
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == ability_name)
        .map(|ability| ability.id.0 as u64)
        .expect("fixture attacker ability");
    runtime.continuation = RuntimeContinuation::AwaitMctsDecision {
        unit_id: attacker_id,
        request_id: 1,
        state_version: 0,
    };
    (runtime, ability_id)
}

#[test]
fn terminal_ability_proves_victory_through_commit_ack_and_boundary() {
    let (mut runtime, ability_id) = terminal_ability_fixture("Caveman", "Mage", 1, 2);
    let committed = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 1,
            state_version: 0,
            decision: RuntimeDecision {
                unit_id: 1,
                action: RuntimeDecisionAction::Ability {
                    ability_id,
                    target: RuntimeAbilityTarget::Unit { unit_id: 2 },
                },
            },
        })
        .0;
    let barrier_id = match committed {
        RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
        other => panic!("expected terminal ability commit, got {other:?}"),
    };
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
    ));
    let completed = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
    let history = match completed {
        RuntimeResponse::GameCompleted { outcome, history } => {
            assert_eq!(outcome, GameOutcome::Victory { winning_team: 1 });
            assert_eq!(
                runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(2)]
                    .health,
                0
            );
            history
        }
        other => panic!("expected victory, got {other:?}"),
    };
    assert_eq!(
        history
            .log
            .iter()
            .filter(|event| matches!(event, Event::GameCompleted { .. }))
            .count(),
        1
    );
    assert!(matches!(
        runtime.process_request(RuntimeRequest::StepPgRpgSimulation).0,
        RuntimeResponse::Error(message) if message.contains("after game completion")
    ));
    assert!(matches!(
        runtime.process_request(RuntimeRequest::ResumeBoundary).0,
        RuntimeResponse::Error(message) if message.contains("after game completion")
    ));
}

#[test]
fn terminal_ability_proves_defeat_through_commit_ack_and_boundary() {
    let (mut runtime, ability_id) = terminal_ability_fixture("Caveman", "Mage", 2, 1);
    let committed = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 1,
            state_version: 0,
            decision: RuntimeDecision {
                unit_id: 2,
                action: RuntimeDecisionAction::Ability {
                    ability_id,
                    target: RuntimeAbilityTarget::Unit { unit_id: 1 },
                },
            },
        })
        .0;
    let barrier_id = match committed {
        RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
        other => panic!("expected terminal ability commit, got {other:?}"),
    };
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
    ));
    let completed = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
    match completed {
        RuntimeResponse::GameCompleted { outcome, .. } => {
            assert_eq!(outcome, GameOutcome::Defeat { winning_team: 2 });
            assert_eq!(
                runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)]
                    .health,
                0
            );
        }
        other => panic!("expected defeat, got {other:?}"),
    }
}

#[test]
fn turn_limit_contract_preserves_move_wait_barriers_and_completion() {
    let mut scenario = SkirmishConfig::new(42);
    scenario.set_maximum_turn_count(1).unwrap();
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

    let mut initial = runtime
        .process_request(RuntimeRequest::StepPgRpgSimulation)
        .0;
    loop {
        while matches!(initial, RuntimeResponse::SimulationProgress { .. }) {
            initial = runtime
                .process_request(RuntimeRequest::StepPgRpgSimulation)
                .0;
        }
        let RuntimeContinuation::AwaitMctsDecision {
            request_id,
            unit_id,
            state_version,
        } = runtime.continuation.clone()
        else {
            break;
        };
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
            panic!("expected typed MCTS candidate");
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
        initial = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
    }
    assert!(matches!(
        initial,
        RuntimeResponse::PgRpgSimulationStepped(ref history)
            if history.log.iter().any(|event| matches!(
                event,
                Event::TurnStarted { unit_id: 1 }
            ))
    ));
    assert!(matches!(
        runtime.continuation,
        RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 }
    ));

    let move_response = runtime
        .process_request(RuntimeRequest::CommitMove {
            request_id: 701,
            preview_request_id: 700,
            unit_id: 1,
            hex: hexx::Hex::new(1, 0),
            layer: 0,
        })
        .0;
    let move_barrier = match move_response {
        RuntimeResponse::ActionCommitted {
            request_id: 701,
            action,
            barrier_id,
            history,
            ..
        } => {
            assert_eq!(action, "move");
            assert!(matches!(
                history.log.first(),
                Some(Event::MoveSprite { .. })
            ));
            barrier_id
        }
        other => panic!("expected committed move, got {other:?}"),
    };
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)].position,
        GridCell::new(hexx::Hex::new(1, 0), 0)
    );
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation {
                barrier_id: move_barrier
            })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 })
    ));

    let wait_response = runtime
        .process_request(RuntimeRequest::CommitWait {
            request_id: 702,
            unit_id: 1,
        })
        .0;
    let wait_barrier = match wait_response {
        RuntimeResponse::ActionCommitted {
            request_id: 702,
            action,
            barrier_id,
            history,
            ..
        } => {
            assert_eq!(action, "wait");
            assert!(matches!(
                history.log.first(),
                Some(Event::TurnCompleted { unit_id: 1 })
            ));
            barrier_id
        }
        other => panic!("expected committed wait, got {other:?}"),
    };
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation {
                barrier_id: wait_barrier
            })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
    ));

    let completed = runtime.process_request(RuntimeRequest::ResumeBoundary).0;
    let completion_history = match completed {
        RuntimeResponse::GameCompleted { outcome, history } => {
            assert_eq!(outcome, GameOutcome::Draw);
            history
        }
        other => panic!("expected turn-limit completion, got {other:?}"),
    };
    assert_eq!(
        completion_history
            .log
            .iter()
            .filter(|event| matches!(
                event,
                Event::GameCompleted {
                    completed_rounds: 1,
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(runtime.pg_rpg_sim.as_ref().unwrap().completed_rounds, 1);
    assert_eq!(runtime.continuation, RuntimeContinuation::Completed);
}

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

#[path = "action_preview_tests.rs"]
mod action_preview_tests;

#[path = "generated_protocol_tests.rs"]
mod generated_protocol_tests;
