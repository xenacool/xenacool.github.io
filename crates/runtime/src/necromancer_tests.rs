use super::*;
use crate::pg_rpg::simulation::TacticalSimulation;
use pystral_games::{GridCell, SkirmishConfig, TacticalDisplayAction};

#[test]
fn raise_skeleton_targets_empty_adjacent_cells_and_spawns_owned_unit() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario.set_unit_mana(1, 0).unwrap();
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
        .find(|ability| ability.name == "Raise Skeleton")
        .unwrap()
        .clone();
    assert!(matches!(
        ability.target_rule,
        pystral_games::AbilityTargetRule::EmptyCell
    ));
    let (targets, reason) = simulation.ability_targets(1, ability.id.0 as u64);
    assert!(reason.is_none());
    assert_eq!(targets.len(), 6);
    assert!(
        targets
            .iter()
            .all(|target| matches!(target.kind, AbilityTargetKind::Cell))
    );

    let summon_cell = GridCell::new(targets[0].hex, targets[0].layer);
    let spawned = simulation
        .commit_area_ability(npc_engine_core::AgentId(1), ability.id, summon_cell)
        .unwrap();
    assert_eq!(spawned, vec![npc_engine_core::AgentId(2)]);
    assert_eq!(
        simulation.state.agents[&npc_engine_core::AgentId(1)].health,
        80
    );
    assert_eq!(
        simulation.state.agents[&npc_engine_core::AgentId(1)].action_points,
        2
    );
    assert_eq!(
        simulation.state.agents[&npc_engine_core::AgentId(1)].mana,
        0
    );
    assert_eq!(
        simulation.state.agents[&npc_engine_core::AgentId(2)].position,
        summon_cell
    );
    assert_eq!(
        simulation.state.summons[&npc_engine_core::AgentId(2)].summoner,
        npc_engine_core::AgentId(1)
    );
}

#[test]
fn necromancer_harvests_owned_summon_and_spends_fresh_soul_on_drain() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario.set_unit_mana(1, 0).unwrap();
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
    let ability_id = |simulation: &TacticalSimulation, name: &str| {
        simulation
            .state
            .ability_registry
            .values()
            .find(|ability| ability.name == name)
            .unwrap()
            .id
    };
    let raise = ability_id(&simulation, "Raise Skeleton");
    let harvest = ability_id(&simulation, "Harvest Skeleton");
    let drain = ability_id(&simulation, "Soul Drain");
    let summon_id = simulation
        .commit_area_ability(
            npc_engine_core::AgentId(1),
            raise,
            GridCell::new(hexx::Hex::new(0, 1), 0),
        )
        .unwrap()[0];

    simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(1))
        .unwrap()
        .action_points = 4;
    simulation
        .apply_npc_action(
            npc_engine_core::AgentId(1),
            TacticalDisplayAction::Ability {
                target: summon_id,
                ability: harvest,
            },
        )
        .unwrap();

    let fresh_soul = simulation.state.ability_registry[&drain].consume_tags[0].0;
    let caster_after_harvest = &simulation.state.agents[&npc_engine_core::AgentId(1)];
    assert_eq!(simulation.state.agents[&summon_id].health, 0);
    assert_eq!(caster_after_harvest.health, 90);
    assert_eq!(caster_after_harvest.mana, 10);
    assert_eq!(caster_after_harvest.action_points, 3);
    assert_eq!(caster_after_harvest.turn_tags.count(fresh_soul), 1);
    assert_eq!(
        simulation.state.ability_registry[&drain]
            .resolve_cost(&caster_after_harvest.turn_tags)
            .ap,
        2
    );

    let target_health_before = simulation.state.agents[&npc_engine_core::AgentId(2)].health;
    let caster_health_before = caster_after_harvest.health;
    simulation
        .apply_npc_action(
            npc_engine_core::AgentId(1),
            TacticalDisplayAction::Ability {
                target: npc_engine_core::AgentId(2),
                ability: drain,
            },
        )
        .unwrap();
    let target_health_after = simulation.state.agents[&npc_engine_core::AgentId(2)].health;
    let actual_damage = target_health_before - target_health_after;
    let caster = &simulation.state.agents[&npc_engine_core::AgentId(1)];
    assert!(actual_damage > 0);
    assert_eq!(caster.mana, 0);
    assert_eq!(caster.action_points, 1);
    assert_eq!(caster.turn_tags.count(fresh_soul), 0);
    assert_eq!(
        caster.health,
        (caster_health_before + actual_damage / 2).min(caster.derived_stats.health_max)
    );
}

#[test]
fn raise_enforces_health_floor_cap_and_atomic_allocator_rollback() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    let mut simulation =
        TacticalSimulation::from_scenario(scenario, npc_engine_core::MCTSConfiguration::default());
    let raise = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Raise Skeleton")
        .unwrap()
        .id;
    simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(1))
        .unwrap()
        .health = 20;
    let before = simulation.snapshot_fingerprint();
    let next_id = simulation.state.next_agent_id;
    assert!(
        simulation
            .commit_area_ability(
                npc_engine_core::AgentId(1),
                raise,
                GridCell::new(hexx::Hex::new(1, 0), 0),
            )
            .is_err()
    );
    assert_eq!(simulation.snapshot_fingerprint(), before);
    assert_eq!(simulation.state.next_agent_id, next_id);

    simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(1))
        .unwrap()
        .health = 100;
    for cell in [hexx::Hex::new(1, 0), hexx::Hex::new(0, 1)] {
        simulation
            .state
            .agents
            .get_mut(&npc_engine_core::AgentId(1))
            .unwrap()
            .action_points = 4;
        simulation
            .commit_area_ability(npc_engine_core::AgentId(1), raise, GridCell::new(cell, 0))
            .unwrap();
    }
    simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(1))
        .unwrap()
        .action_points = 4;
    let before_third = simulation.snapshot_fingerprint();
    let next_id = simulation.state.next_agent_id;
    assert!(
        simulation
            .commit_area_ability(
                npc_engine_core::AgentId(1),
                raise,
                GridCell::new(hexx::Hex::new(-1, 1), 0),
            )
            .is_err()
    );
    assert_eq!(simulation.snapshot_fingerprint(), before_third);
    assert_eq!(simulation.state.next_agent_id, next_id);
}

#[test]
fn summons_do_not_keep_team_alive_and_owner_death_cleans_them_up() {
    let mut scenario = SkirmishConfig::new(42);
    scenario
        .add_unit(1, 1, "Necromancer", GridCell::new(hexx::Hex::ZERO, 0))
        .unwrap();
    scenario
        .add_unit(2, 2, "Caveman", GridCell::new(hexx::Hex::new(3, 0), 0))
        .unwrap();
    let mut simulation =
        TacticalSimulation::from_scenario(scenario, npc_engine_core::MCTSConfiguration::default());
    let raise = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Raise Skeleton")
        .unwrap()
        .id;
    let summon = simulation
        .commit_area_ability(
            npc_engine_core::AgentId(1),
            raise,
            GridCell::new(hexx::Hex::new(0, 1), 0),
        )
        .unwrap()[0];
    simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(1))
        .unwrap()
        .health = 0;
    assert_eq!(simulation.living_team_count(), 1);
    assert_eq!(
        simulation.outcome(),
        Some(pystral_core::log::GameOutcome::Defeat { winning_team: 2 })
    );
    assert_eq!(simulation.state.agents[&summon].health, 80);
    assert_eq!(simulation.state.cleanup_orphaned_summons(), vec![summon]);
    assert_eq!(simulation.state.agents[&summon].health, 0);
}

#[test]
fn dead_roster_units_do_not_block_soul_drain_targeting() {
    let mut scenario = SkirmishConfig::new(42);
    for (id, team, job, hex) in [
        (1, 1, "Caveman", hexx::Hex::new(0, 0)),
        (2, 1, "Mage", hexx::Hex::new(1, -1)),
        (5, 1, "Necromancer", hexx::Hex::new(-1, 1)),
        (3, 2, "Caveman", hexx::Hex::new(3, -3)),
        (4, 2, "Mage", hexx::Hex::new(4, -3)),
        (6, 2, "Skeleton_Minion", hexx::Hex::new(3, -2)),
    ] {
        scenario
            .add_unit(id, team, job, GridCell::new(hex, 0))
            .unwrap();
    }
    let mut simulation =
        TacticalSimulation::from_scenario(scenario, npc_engine_core::MCTSConfiguration::default());
    for id in [1, 2, 4, 6] {
        simulation
            .state
            .agents
            .get_mut(&npc_engine_core::AgentId(id))
            .unwrap()
            .health = 0;
    }
    let drain = simulation
        .state
        .ability_registry
        .values()
        .find(|ability| ability.name == "Soul Drain")
        .unwrap()
        .id;
    let caster = simulation
        .state
        .agents
        .get_mut(&npc_engine_core::AgentId(5))
        .unwrap();
    caster.mana = 10;
    let fresh_soul = simulation.state.ability_registry[&drain].consume_tags[0].0;
    caster.turn_tags.counts.insert(fresh_soul, 1);
    assert!(
        pystral_games::legal_ability_targets(
            &simulation.state,
            npc_engine_core::AgentId(5),
            drain,
        )
            .contains(&npc_engine_core::AgentId(3))
    );
}
