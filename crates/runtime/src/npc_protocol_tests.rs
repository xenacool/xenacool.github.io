use super::*;
use crate::pg_rpg::simulation::TacticalSimulation;
use pystral_core::log::Event;
use pystral_games::{GridCell, SkirmishConfig};

fn test_runtime(scenario: SkirmishConfig) -> Runtime {
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
    let history = runtime.pg_rpg_history.clone().unwrap();
    let simulation = runtime.pg_rpg_sim.clone().unwrap();
    runtime.rhai_session = Some(
        crate::rhai_session::RhaiSession::from_simulation(history, simulation)
            .expect("test Rhai session"),
    );
    runtime
}

#[test]
fn npc_ability_uses_typed_candidate_revalidation_and_barrier() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let mut runtime = test_runtime(scenario);
    let target_health_before =
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)].health;
    let ability_id = runtime
        .pg_rpg_sim
        .as_ref()
        .unwrap()
        .get_available_actions(2)
        .unwrap()
        .primary_job
        .abilities
        .first()
        .unwrap()
        .id;
    runtime.continuation = RuntimeContinuation::AwaitMctsDecision {
        unit_id: 2,
        request_id: 90,
        state_version: 0,
    };
    let response = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 90,
            state_version: 0,
            decision: RuntimeDecision {
                unit_id: 2,
                action: RuntimeDecisionAction::Ability {
                    ability_id: u64::from(ability_id),
                    target: RuntimeAbilityTarget::Unit { unit_id: 1 },
                },
            },
        })
        .0;
    let barrier_id = match response {
        RuntimeResponse::ActionCommitted {
            action,
            barrier_id,
            history,
            ..
        } => {
            assert_eq!(action, "ability");
            assert!(history.log.iter().any(|event| matches!(
                event, Event::Log { msg } if msg.contains("NPC unit 2 used")
            )));
            assert!(history.log.iter().any(|event| matches!(
                event,
                Event::UnitStateChanged { unit_id: 1, health, .. }
                    if *health < target_health_before
            )));
            assert!(
                runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)]
                    .health
                    < target_health_before
            );
            barrier_id
        }
        other => panic!("expected committed NPC ability, got {other:?}"),
    };
    assert!(matches!(
        runtime.continuation,
        RuntimeContinuation::AwaitAnimationAck {
            npc: true,
            ends_turn: false,
            ..
        }
    ));
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitMctsDecision { unit_id: 2, .. })
    ));
}

#[test]
fn rejected_npc_candidate_is_visible_before_fallback_wait() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let mut runtime = test_runtime(scenario);
    runtime
        .pg_rpg_sim
        .as_mut()
        .unwrap()
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(2))
        .unwrap()
        .action_points = 0;
    let fireball = runtime
        .pg_rpg_sim
        .as_ref()
        .unwrap()
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Fireball")
        .unwrap()
        .id
        .0 as u64;
    runtime.continuation = RuntimeContinuation::AwaitMctsDecision {
        unit_id: 2,
        request_id: 92,
        state_version: 0,
    };
    let response = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 92,
            state_version: 0,
            decision: RuntimeDecision {
                unit_id: 2,
                action: RuntimeDecisionAction::Ability {
                    ability_id: fireball,
                    target: RuntimeAbilityTarget::Unit { unit_id: 2 },
                },
            },
        })
        .0;
    let history = match response {
        RuntimeResponse::ActionCommitted {
            action, history, ..
        } => {
            assert_eq!(action, "wait");
            history
        }
        other => panic!("expected fallback wait, got {other:?}"),
    };
    assert!(history.log.iter().any(|event| matches!(
        event,
        Event::Log { msg } if msg.contains("candidate rejected; fallback Wait")
    )));
}

#[test]
fn rejected_npc_candidate_resolves_pending_reaction_before_fallback() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let mut runtime = test_runtime(scenario);
    runtime
        .pg_rpg_sim
        .as_mut()
        .unwrap()
        .state
        .reaction_queue
        .push((
            npc_engine_core::AgentId(2),
            pystral_games::ReactionId(101),
            npc_engine_core::AgentId(1),
        ));
    runtime.continuation = RuntimeContinuation::AwaitMctsDecision {
        unit_id: 2,
        request_id: 93,
        state_version: 0,
    };
    let response = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 93,
            state_version: 0,
            decision: RuntimeDecision {
                unit_id: 2,
                action: RuntimeDecisionAction::Ability {
                    ability_id: 3,
                    target: RuntimeAbilityTarget::Unit { unit_id: 1 },
                },
            },
        })
        .0;

    assert!(matches!(
        response,
        RuntimeResponse::ActionCommitted { action, .. } if action == "reaction"
    ));
    assert!(
        runtime
            .pg_rpg_sim
            .as_ref()
            .unwrap()
            .state
            .reaction_queue
            .is_empty()
    );
}

#[test]
fn stale_and_duplicate_mcts_results_cannot_commit() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    let mut runtime = test_runtime(scenario);
    runtime.continuation = RuntimeContinuation::AwaitMctsDecision {
        unit_id: 1,
        request_id: 91,
        state_version: 0,
    };
    let decision = RuntimeDecision {
        unit_id: 1,
        action: RuntimeDecisionAction::Wait,
    };
    let stale = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 90,
            decision: decision.clone(),
            state_version: 0,
        })
        .0;
    assert!(matches!(stale, RuntimeResponse::Error(message)
        if message.contains("Stale MCTS result")));
    assert!(runtime.pg_rpg_history.as_ref().unwrap().log.is_empty());
    let committed = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 91,
            decision: decision.clone(),
            state_version: 0,
        })
        .0;
    let barrier_id = match committed {
        RuntimeResponse::ActionCommitted { barrier_id, .. } => barrier_id,
        other => panic!("expected committed NPC wait, got {other:?}"),
    };
    let duplicate = runtime
        .process_request(RuntimeRequest::MctsDecisionReady {
            request_id: 91,
            decision,
            state_version: 0,
        })
        .0;
    assert!(matches!(duplicate, RuntimeResponse::Error(message)
        if message.contains("outside MCTS boundary")));
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitBoundary)
    ));
}
