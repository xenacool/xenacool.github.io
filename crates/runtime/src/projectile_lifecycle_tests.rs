use super::*;
use crate::pg_rpg::simulation::TacticalSimulation;
use pystral_games::{GridCell, SkirmishConfig};

#[test]
fn soul_drain_projectile_commits_before_ack_and_despawns_exactly_once() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Caveman", GridCell::new(hexx::Hex::new(3, 0), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let drain = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Soul Drain")
        .unwrap()
        .id;
    let fresh_soul = simulation.state.ability_registry[&drain].consume_tags[0].0;
    let caster = simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(1))
        .unwrap();
    caster.mana = 10;
    caster.turn_tags.counts.insert(fresh_soul, 1);

    let mut runtime = Runtime::new();
    runtime.pg_rpg_sim = Some(simulation);
    runtime.pg_rpg_history = Some(HistoryManager::new());
    runtime.continuation = RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 };
    let target_response = runtime
        .process_request(RuntimeRequest::OpenAbilityTargets {
            request_id: 1,
            unit_id: 1,
            ability_id: drain.0 as u64,
        })
        .0;
    let provenance = match target_response {
        RuntimeResponse::AbilityTargets {
            target_session_id,
            state_version,
            snapshot_fingerprint,
            ..
        } => DecisionProvenance {
            target_session_id,
            state_version,
            snapshot_fingerprint,
        },
        other => panic!("expected Soul Drain targets, got {other:?}"),
    };
    let target_health_before =
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(2)].health;

    let committed = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 2,
            decision: RuntimeDecision {
                unit_id: 1,
                action: RuntimeDecisionAction::Ability {
                    ability_id: drain.0 as u64,
                    target: RuntimeAbilityTarget::Unit { unit_id: 2 },
                },
            },
            provenance: Some(provenance),
        })
        .0;
    let (barrier_id, projectile_id) =
        match committed {
            RuntimeResponse::ActionCommitted {
                barrier_id,
                history,
                ..
            } => {
                let projectile_id = history
                    .log
                    .iter()
                    .find_map(|event| match event {
                        Event::SpawnEntity { id, kind, .. } if kind == "projectile" => Some(*id),
                        _ => None,
                    })
                    .expect("Soul Drain should spawn a presentation projectile");
                assert!(history.log.iter().any(|event| matches!(
                    event,
                    Event::UpdateProperty {
                        id,
                        property,
                        value: pystral_core::log::PropertyValue::AssetRef(profile),
                    } if *id == projectile_id && property == "asset" && profile == "SoulOrb"
                )));
                assert!(history.log.iter().any(|event| matches!(
                    event,
                    Event::MoveSprite { id, destination, .. }
                        if *id == projectile_id && *destination == hexx::Hex::new(3, 0)
                )));
                assert!(!history.log.iter().any(
                    |event| matches!(event, Event::DespawnEntity { id } if *id == projectile_id)
                ));
                (barrier_id, projectile_id)
            }
            other => panic!("expected committed Soul Drain, got {other:?}"),
        };

    let caster_after_commit =
        &runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(1)];
    let target_health_after =
        runtime.pg_rpg_sim.as_ref().unwrap().state.agents[&npc_engine_core::AgentId(2)].health;
    assert!(target_health_after < target_health_before);
    assert_eq!(caster_after_commit.mana, 0);
    assert_eq!(caster_after_commit.action_points, 2);
    assert_eq!(caster_after_commit.turn_tags.count(fresh_soul), 0);
    let committed_fingerprint = runtime.pg_rpg_sim.as_ref().unwrap().snapshot_fingerprint();
    let history_len_before_ack = runtime.pg_rpg_history.as_ref().unwrap().log.len();

    let stale = runtime
        .process_request(RuntimeRequest::AcknowledgeAnimation {
            barrier_id: barrier_id.saturating_sub(1),
        })
        .0;
    assert!(matches!(stale, RuntimeResponse::Error(message) if message.contains("expected")));
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().snapshot_fingerprint(),
        committed_fingerprint
    );
    assert_eq!(
        runtime.pg_rpg_history.as_ref().unwrap().log.len(),
        history_len_before_ack
    );

    let acknowledged = runtime
        .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
        .0;
    match acknowledged {
        RuntimeResponse::AnimationAcknowledged {
            continuation,
            history,
        } => {
            assert_eq!(
                continuation,
                RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 }
            );
            assert!(matches!(
                history.log.as_slice(),
                [Event::DespawnEntity { id }] if *id == projectile_id
            ));
        }
        other => panic!("expected projectile acknowledgment, got {other:?}"),
    }
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().snapshot_fingerprint(),
        committed_fingerprint
    );
    assert_eq!(
        runtime.pg_rpg_history.as_ref().unwrap().log.len(),
        history_len_before_ack + 1
    );

    let duplicate = runtime
        .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
        .0;
    assert!(
        matches!(duplicate, RuntimeResponse::Error(message) if message.contains("without a pending barrier"))
    );
    assert_eq!(
        runtime.pg_rpg_sim.as_ref().unwrap().snapshot_fingerprint(),
        committed_fingerprint
    );
    assert_eq!(
        runtime.pg_rpg_history.as_ref().unwrap().log.len(),
        history_len_before_ack + 1
    );
}

#[test]
fn melee_ability_emits_no_projectile_lifecycle() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    let simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let smash = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Club Smash")
        .unwrap()
        .id;
    let mut runtime = Runtime::new();
    runtime.pg_rpg_sim = Some(simulation);
    runtime.pg_rpg_history = Some(HistoryManager::new());
    runtime.continuation = RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 };

    let target_response = runtime
        .process_request(RuntimeRequest::OpenAbilityTargets {
            request_id: 1,
            unit_id: 1,
            ability_id: smash.0 as u64,
        })
        .0;
    let provenance = match target_response {
        RuntimeResponse::AbilityTargets {
            target_session_id,
            state_version,
            snapshot_fingerprint,
            ..
        } => DecisionProvenance {
            target_session_id,
            state_version,
            snapshot_fingerprint,
        },
        other => panic!("expected Club Smash targets, got {other:?}"),
    };
    let committed = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 2,
            decision: RuntimeDecision {
                unit_id: 1,
                action: RuntimeDecisionAction::Ability {
                    ability_id: smash.0 as u64,
                    target: RuntimeAbilityTarget::Unit { unit_id: 2 },
                },
            },
            provenance: Some(provenance),
        })
        .0;
    let barrier_id = match committed {
        RuntimeResponse::ActionCommitted {
            barrier_id,
            history,
            ..
        } => {
            assert!(!history.log.iter().any(|event| matches!(
                event,
                Event::SpawnEntity {
                    kind,
                    ..
                } if kind == "projectile"
            )));
            barrier_id
        }
        other => panic!("expected committed Club Smash, got {other:?}"),
    };
    assert!(matches!(
        runtime
            .process_request(RuntimeRequest::AcknowledgeAnimation { barrier_id })
            .0,
        RuntimeResponse::Continuation(RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 })
    ));
    assert!(runtime.pending_projectile_despawns.is_empty());
}
