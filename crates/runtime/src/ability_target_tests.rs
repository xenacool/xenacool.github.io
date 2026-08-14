use super::*;
use crate::demo::simulation::TacticalSimulation;
use npc_engine_core::Domain;
use proptest::prelude::*;
use pystral_games::{
    GridCell, SkirmishConfig, TacticalDiff, TacticalDisplayAction, TacticalDomain,
};

#[test]
fn query_returns_authoritative_legal_targets() {
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
    let ability_id = simulation
        .state
        .ability_registry
        .values()
        .find(|a| a.name == "Club Smash")
        .unwrap()
        .id
        .0 as u64;
    let mut runtime = Runtime::new();
    runtime.demo_sim = Some(simulation);
    runtime.continuation = RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 };
    let response = runtime
        .process_request(RuntimeRequest::OpenAbilityTargets {
            request_id: 1,
            unit_id: 1,
            ability_id,
        })
        .0;
    assert!(
        matches!(response, RuntimeResponse::AbilityTargets { targets, disabled_reason: None, .. } if targets.iter().any(|target| matches!(target.kind, AbilityTargetKind::Unit { unit_id: 2 })))
    );
}

#[test]
fn demo_mage_fireball_target_revalidates() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::new(0, 0), 0))
        .unwrap();
    scenario
        .add_unit(2, 1, "Mage", GridCell::new(hexx::Hex::new(1, -1), 0))
        .unwrap();
    scenario
        .add_unit(3, 2, "Necromancer", GridCell::new(hexx::Hex::new(5, -5), 0))
        .unwrap();
    scenario
        .add_unit(
            4,
            2,
            "Skeleton_Minion",
            GridCell::new(hexx::Hex::new(4, -4), 0),
        )
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            visits: 40,
            depth: 20,
            seed: Some(42),
            ..Default::default()
        },
    );
    let fireball = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Fireball")
        .unwrap()
        .id;
    let (targets, reason) = simulation.ability_targets(2, fireball.0 as u64);
    assert!(reason.is_none(), "{reason:?}");
    assert!(
        targets
            .iter()
            .any(|target| { matches!(target.kind, AbilityTargetKind::Unit { unit_id: 3 }) })
    );
    simulation
        .apply_npc_action(
            npc_engine_core::AgentId(2),
            pystral_games::TacticalDisplayAction::Ability {
                target: npc_engine_core::AgentId(3),
                ability: fireball,
            },
        )
        .expect("fireball target should revalidate");
}

#[test]
fn player_ability_commit_resolves_pending_reaction_before_fireball() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, -1), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let mage = npc_engine_core::AgentId(2);
    let target = npc_engine_core::AgentId(1);
    let fireball = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Fireball")
        .unwrap()
        .id;
    let reaction = simulation.state.agents[&mage].reaction_abilities[0];
    let target_health_before = simulation.state.agents[&target].health;
    simulation
        .state
        .reaction_queue
        .push((mage, reaction, target));

    let mut runtime = Runtime::new();
    runtime.demo_sim = Some(simulation);
    runtime.demo_history = Some(HistoryManager::new());
    runtime.continuation = RuntimeContinuation::AwaitPlayerDecision { unit_id: 2 };
    let provenance = match runtime
        .process_request(RuntimeRequest::OpenAbilityTargets {
            request_id: 1,
            unit_id: 2,
            ability_id: fireball.0 as u64,
        })
        .0
    {
        RuntimeResponse::AbilityTargets {
            state_version,
            target_session_id,
            snapshot_fingerprint,
            ..
        } => DecisionProvenance {
            state_version,
            target_session_id,
            snapshot_fingerprint,
        },
        other => panic!("expected ability targets, got {other:?}"),
    };

    let response = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 2,
            decision: RuntimeDecision {
                unit_id: 2,
                action: RuntimeDecisionAction::Ability {
                    ability_id: fireball.0 as u64,
                    target: RuntimeAbilityTarget::Unit { unit_id: 1 },
                },
            },
            provenance: Some(provenance),
        })
        .0;

    assert!(matches!(
        response,
        RuntimeResponse::ActionCommitted { action, history, .. }
            if action.starts_with("ability (")
                && history.log.iter().any(|event| matches!(
                    event,
                    pystral_core::log::Event::Log { msg }
                        if msg.contains("resolved forced reaction")
                ))
    ));
    assert!(
        !runtime
            .demo_sim
            .as_ref()
            .unwrap()
            .state
            .reaction_queue
            .iter()
            .any(|(agent, _, _)| *agent == mage)
    );
    assert!(runtime.demo_sim.as_ref().unwrap().state.agents[&target].health < target_health_before);
}

proptest! {
    #[test]
    fn menu_targets_equal_revalidated_mage_targets(
        q in -5i32..=5,
        r in -5i32..=5,
    ) {
        prop_assume!((q, r) != (0, 0));
        prop_assume!(q.abs().max(r.abs()).max((q + r).abs()) <= 6);
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Mage", GridCell::new(hexx::Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Caveman", GridCell::new(hexx::Hex::new(q, r), 0))
            .unwrap();
        let simulation = TacticalSimulation::from_scenario(
            scenario,
            npc_engine_core::MCTSConfiguration { seed: Some(42), ..Default::default() },
        );
        let fireball = simulation
            .state
            .ability_registry
            .values()
            .find(|ability| ability.name == "Fireball")
            .unwrap()
            .id;
        let menu_targets = simulation
            .ability_targets(1, fireball.0 as u64)
            .0
            .into_iter()
            .filter_map(|target| match target.kind {
                AbilityTargetKind::Unit { unit_id } => Some(unit_id),
                AbilityTargetKind::Cell => None,
            })
            .collect::<std::collections::BTreeSet<_>>();

        let diff = TacticalDiff::default();
        let context = npc_engine_core::Context::with_state_and_diff(
            0,
            &simulation.state,
            &diff,
            npc_engine_core::AgentId(1),
        );
        let task_targets = TacticalDomain::get_tasks(context)
            .into_iter()
            .filter_map(|task| match task.display_action() {
                TacticalDisplayAction::Ability { target, ability }
                    if ability == fireball => Some(target.0 as u64),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();

        prop_assert_eq!(menu_targets, task_targets);
    }
}

#[test]
fn area_targets_use_radius_one_and_spend_ap_once() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Mage", GridCell::new(hexx::Hex::new(1, 0), 0))
        .unwrap();
    scenario
        .add_unit(3, 2, "Mage", GridCell::new(hexx::Hex::new(0, 1), 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let ability = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Primal Roar")
        .unwrap()
        .id;
    let before_ap = simulation.state.agents[&npc_engine_core::AgentId(1)].action_points;
    let affected = simulation
        .commit_area_ability(
            npc_engine_core::AgentId(1),
            ability,
            GridCell::new(hexx::Hex::ZERO, 0),
        )
        .unwrap();

    assert_eq!(affected.len(), 2);
    assert_eq!(
        simulation.state.agents[&npc_engine_core::AgentId(1)].action_points,
        before_ap - 3
    );
}

#[test]
fn raise_skeleton_is_not_cell_targeted() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    let simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let ability = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Raise Skeleton")
        .unwrap();
    assert!(matches!(
        ability.delivery,
        pystral_games::AbilityDelivery::SelfTarget
    ));
    let (targets, reason) = simulation.ability_targets(1, ability.id.0 as u64);
    assert!(reason.is_none());
    assert_eq!(targets.len(), 1);
    assert!(matches!(
        targets[0].kind,
        AbilityTargetKind::Unit { unit_id: 1 }
    ));
}

#[test]
fn area_without_enemy_targets_rejects_without_spending_ap() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Caveman", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    let mut simulation = TacticalSimulation::from_scenario(
        scenario,
        npc_engine_core::MCTSConfiguration {
            seed: Some(42),
            ..Default::default()
        },
    );
    let ability = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Primal Roar")
        .unwrap()
        .id;
    let before_ap = simulation.state.agents[&npc_engine_core::AgentId(1)].action_points;
    let result = simulation.commit_area_ability(
        npc_engine_core::AgentId(1),
        ability,
        GridCell::new(hexx::Hex::ZERO, 0),
    );

    assert!(result.is_err());
    assert_eq!(
        simulation.state.agents[&npc_engine_core::AgentId(1)].action_points,
        before_ap
    );
}

#[test]
fn ability_commit_rejects_forged_target_session() {
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
    let ability_id = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Club Smash")
        .unwrap()
        .id
        .0 as u64;
    let mut runtime = Runtime::new();
    runtime.demo_sim = Some(simulation);
    runtime.demo_history = Some(HistoryManager::new());
    runtime.continuation = RuntimeContinuation::AwaitPlayerDecision { unit_id: 1 };
    let targets = runtime
        .process_request(RuntimeRequest::OpenAbilityTargets {
            request_id: 1,
            unit_id: 1,
            ability_id,
        })
        .0;
    let (target_session_id, state_version, snapshot_fingerprint) = match targets {
        RuntimeResponse::AbilityTargets {
            target_session_id,
            state_version,
            snapshot_fingerprint,
            ..
        } => (target_session_id, state_version, snapshot_fingerprint),
        other => panic!("expected target session, got {other:?}"),
    };
    let decision = RuntimeDecision {
        unit_id: 1,
        action: RuntimeDecisionAction::Ability {
            ability_id,
            target: RuntimeAbilityTarget::Unit { unit_id: 2 },
        },
    };
    let forged = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 2,
            decision: decision.clone(),
            provenance: Some(DecisionProvenance {
                state_version,
                target_session_id: target_session_id + 1,
                snapshot_fingerprint,
            }),
        })
        .0;
    assert!(
        matches!(forged, RuntimeResponse::Error(message) if message.contains("Stale ability target session"))
    );
    let accepted_provenance = runtime
        .process_request(RuntimeRequest::CommitDecision {
            request_id: 3,
            decision,
            provenance: Some(DecisionProvenance {
                state_version,
                target_session_id,
                snapshot_fingerprint,
            }),
        })
        .0;
    assert!(
        matches!(accepted_provenance, RuntimeResponse::ActionCommitted { action, .. } if action.starts_with("ability ("))
    );
}

proptest! {
    #[test]
    fn navigation_connects_unique_stacked_targets(points in prop::collection::vec((-4i32..=4, -4i32..=4, 0i32..=2), 2..12)) {
        let unique = points.into_iter().collect::<std::collections::BTreeSet<_>>();
        prop_assume!(unique.len() >= 2);
        let targets = unique.into_iter().map(|(q, r, layer)| AbilityTarget {
            kind: AbilityTargetKind::Cell, hex: hexx::Hex::new(q, r), layer, label: format!("{q}:{r}:{layer}"),
        }).collect::<Vec<_>>();
        let mut reached = std::collections::BTreeSet::from([0usize]);
        let mut frontier = vec![0usize];
        while let Some(index) = frontier.pop() {
            for direction in ["up", "down", "left", "right", "layer-up", "layer-down"] {
                let next = crate::demo::ability_targets::next_ability_target(&targets, index, direction);
                if reached.insert(next) { frontier.push(next); }
            }
        }
        prop_assert_eq!(reached.len(), targets.len());
    }
}
